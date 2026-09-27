#!/usr/bin/env bash
# V18 (T-15 / W0-09): preflight + single PG integration test for monitor market persist.
# Requires disposable local DATABASE_URL → database trading_bot (see postgres-and-graph-dev.md).
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "error: export DATABASE_URL=postgresql://USER:PASS@HOST:PORT/trading_bot" >&2
  exit 1
fi

if [[ "${DATABASE_URL}" != *trading_bot* ]]; then
  echo "error: DATABASE_URL must target database trading_bot" >&2
  exit 1
fi

if [[ "${DATABASE_URL}" =~ @([^:/]+) ]]; then
  host="${BASH_REMATCH[1]}"
  case "$host" in
    127.0.0.1 | localhost | host.docker.internal) ;;
    *)
      echo "error: V18 audit refuses non-local host (${host}); use an isolated trading_bot" >&2
      exit 1
      ;;
  esac
fi

echo "V18 audit: DATABASE_URL targets trading_bot on local host (credentials not logged)"
CARGO_INCREMENTAL=0 cargo test --locked --bin bot pg_persist_dataset_transaction_rollback_and_idempotent_replay -- --nocapture
echo "OK: V18 monitor persistence PostgreSQL audit passed"
