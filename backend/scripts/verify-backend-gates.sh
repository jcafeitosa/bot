#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --check
CARGO_INCREMENTAL=0 cargo clippy --locked --bin bot -- -D warnings
bash scripts/check-import-direction.sh
CARGO_INCREMENTAL=0 cargo test --locked --bin bot
CARGO_INCREMENTAL=0 cargo test --locked
echo "OK: backend gates passed"
