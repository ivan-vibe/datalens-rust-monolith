//! `datalens-auth-rs` — standalone Rust replacement for the DataLens auth container.
//!
//! Guards return fully built `Response`s so handlers read as a linear sequence of checks; the
//! clippy `result_large_err` lint is a size preference that does not matter at this traffic.
#![allow(clippy::result_large_err)]
//!
//! Same HTTP protocol, cookies, JWT claims and PostgreSQL tables as
//! `ghcr.io/datalens-tech/datalens-auth:0.27.0`, so mono-ui and the other services can use it
//! without changes. No container is required: it is one binary plus env vars, talking to
//! the shared `pg-auth-db`.

mod config;
mod cookies;
mod db;
mod guards;
mod http;
mod ids;
mod jwt;
mod models;
mod password;
mod routes;
mod users_api;
mod validation;

use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use config::Config;
use db::Db;
use jwt::Jwt;

pub struct AppState {
    pub config: Config,
    pub jwt: Jwt,
    pub db: Db,
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required (PEM key)"))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}

/// Reject the service-internal `/private/*` surface for non-loopback peers.
async fn internal_only(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let private = request
        .uri()
        .path()
        .strip_prefix("/private")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
    if private {
        let loopback = request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|axum::extract::ConnectInfo(address)| address.ip().is_loopback())
            .unwrap_or(false);
        if !loopback {
            return (axum::http::StatusCode::FORBIDDEN, "private API is internal").into_response();
        }
    }
    next.run(request).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();

    let config = Config::from_env().context("invalid auth-rs configuration")?;
    let jwt = Jwt::new(
        &required_env("TOKEN_PRIVATE_KEY")?,
        &required_env("TOKEN_PUBLIC_KEY")?,
        config.access_ttl,
    )
    .context("invalid TOKEN_PRIVATE_KEY/TOKEN_PUBLIC_KEY")?;
    let db = Db::new(config.dsn.clone(), config.db_pool_size).await?;
    let port = config.port;
    tracing::info!(
        port,
        dsn = %config.dsn,
        access_ttl = config.access_ttl,
        refresh_ttl = config.refresh_ttl,
        session_ttl = config.session_ttl,
        signup_disabled = config.signup_disabled,
        "datalens-auth-rs listening"
    );

    let state = Arc::new(AppState { config, jwt, db });
    // `/private/v1/...` is the service-to-service surface (master token). The auth service is
    // reachable from the LAN, so keep it loopback-only: the UI and the monolith call the public
    // routes, and a leaked/predictable master token must not be enough from another host.
    let app = routes::router(state).layer(axum::middleware::from_fn(internal_only));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("bind port {port}"))?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("serve")?;
    Ok(())
}
