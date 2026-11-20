#!/usr/bin/env bash
# Build the monolith and auth binaries for the host platform.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release
(cd auth && cargo build --release)
echo "monolith: $ROOT/target/release/datalens-monolith"
echo "auth    : $ROOT/auth/target/release/datalens-auth-rs"
