#!/usr/bin/env bash
# Thin wrapper around docker compose with the generated secret overlay.
#
#   ./deploy/cluster.sh up        build images and start the cluster
#   ./deploy/cluster.sh ps        status
#   ./deploy/cluster.sh logs -f   logs
#   ./deploy/cluster.sh down      stop (keeps the postgres volume)
#   ./deploy/cluster.sh reset     stop and delete the postgres volume (re-seeds on next up)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=(docker compose -p datalens-rust \
  -f deploy/docker-compose.yml -f deploy/secrets/compose.secrets.yml)
command="${1:-up}"
shift || true

case "$command" in
  up)
    "$ROOT/deploy/bootstrap.sh"
    "${COMPOSE[@]}" up -d --build "$@"
    echo
    echo "UI      : http://localhost:8091   (admin / admin — change the password)"
    echo "UI (h2) : https://localhost:8092  (self-signed certificate)"
    echo "API     : http://localhost:8099/ping"
    ;;
  down)  "${COMPOSE[@]}" down "$@" ;;
  reset) "${COMPOSE[@]}" down -v "$@" ;;
  ps|logs|restart|stop|start|build|config)
    "${COMPOSE[@]}" "$command" "$@" ;;
  *) echo "usage: $0 up|down|reset|ps|logs|restart|stop|start|build|config ..." >&2; exit 2 ;;
esac
