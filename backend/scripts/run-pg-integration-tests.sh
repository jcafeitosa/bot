#!/usr/bin/env bash
# Runs 20 PostgreSQL domain integration tests in the bot binary (CI job postgres-integration).
# Requires DATABASE_URL → database `trading_bot` on PostgreSQL 18+ with TimescaleDB + pgvector
# (see docs/operations/postgres-and-graph-dev.md and docker-compose.bot.yml).
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required (postgresql://USER:PASS@HOST:PORT/trading_bot)" >&2
  exit 1
fi

if [[ "${DATABASE_URL}" != *trading_bot* ]]; then
  echo "error: DATABASE_URL must target database trading_bot (postgresql://…/trading_bot)" >&2
  exit 1
fi

PG_TESTS=(
  postgres_scaffold_tables_exist_after_migrate
  persist_dataset_round_trip
  persist_dataset_rejects_conflicting_manifest_for_same_id
  pg_catalog_store_round_trip
  pg_identity_snapshot_round_trip
  pg_agent_lifecycle_write_through_round_trip
  pg_cold_start_apply_snapshot_after_write_through
  pg_order_idempotency_round_trip
  pg_order_idempotency_try_claim_and_release
  pg_order_reconciliation_round_trip
  pg_hydrate_order_reconciliation_from_pg_after_durable_write
  pg_order_reconciliation_lookup_reads_pg_when_memory_empty
  pg_bot_catalog_snapshot_round_trip_via_api_state
  pg_submit_order_idempotency_reads_pg_when_memory_empty
  pg_submit_order_idempotency_releases_claim_when_submit_fails
  pg_register_agent_and_persist_cold_start_via_snapshot
  pg_http_boot_sequence_mirrors_serve_wiring
  loads_credentials_from_postgres
  pg_graph_projection_outbox_enqueue_and_drain_mock
  pg_graph_projection_outbox_drain_marks_retry_on_port_failure
)

# Keep in sync with docs (test-matrix, modules-completeness-audit, README).
EXPECTED_PG_INTEGRATION_TESTS=20
if [[ ${#PG_TESTS[@]} -ne ${EXPECTED_PG_INTEGRATION_TESTS} ]]; then
  echo "error: PG_TESTS manifest drift: expected ${EXPECTED_PG_INTEGRATION_TESTS}, got ${#PG_TESTS[@]}" >&2
  exit 1
fi

for test_name in "${PG_TESTS[@]}"; do
  echo "==> ${test_name}"
  CARGO_INCREMENTAL=0 cargo test --locked --bin bot "${test_name}" -- --nocapture
done
echo "OK: PostgreSQL integration tests passed (${#PG_TESTS[@]} tests)"
