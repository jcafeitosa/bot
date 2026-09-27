#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --check
CARGO_INCREMENTAL=0 cargo clippy --locked --bin bot -- -D warnings
bash scripts/check-import-direction.sh
if out=$(rg 'env::var\(|std::env::var\(' src --glob '*.rs' | rg -v '(^|/)config\.rs:|/core/config/|env_parse\.rs|/core/providers/credentials/' || true); then
  if [[ -n "$out" ]]; then
    echo "FAIL: env var reads outside *config.rs / env_parse.rs"
    echo "$out"
    exit 1
  fi
fi
# Shared ledger + env guards must not be held across .await while other tests run in parallel.
BOT_TEST_LOG="$(mktemp)"
trap 'rm -f "$BOT_TEST_LOG"' EXIT
CARGO_INCREMENTAL=0 cargo test --locked --bin bot -- --test-threads=1 2>&1 | tee "$BOT_TEST_LOG"
# Source of truth for docs baseline (passed/ignored): sync SDDs, README, test-matrix to this summary.
BOT_TEST_SUMMARY="$(rg '^test result:' "$BOT_TEST_LOG" | tail -1)"
# Do not re-run `cargo test --locked` (would execute bin `bot` unit tests again in parallel and flake).
INTEGRATION_TESTS=(
  backtest_fixture
  config_cli
  monitor_startup_cli
  redirect_origin_test
  redirect_policy_test
)
for test_name in "${INTEGRATION_TESTS[@]}"; do
  CARGO_INCREMENTAL=0 cargo test --locked --test "$test_name"
done
echo "OK: backend gates passed (bin bot: ${BOT_TEST_SUMMARY})"
