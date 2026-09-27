#!/usr/bin/env bash
# Canonical local verification: unit/workspace gates + optional PostgreSQL integration.
set -euo pipefail
cd "$(dirname "$0")/.."

bash scripts/verify-backend-gates.sh

if [[ -n "${DATABASE_URL:-}" ]]; then
  echo ""
  bash scripts/run-pg-integration-tests.sh
else
  echo ""
  echo "skip: PostgreSQL integration (export DATABASE_URL → .../trading_bot to run ./scripts/run-pg-integration-tests.sh)"
fi

echo ""
echo "OK: backend full verification passed"
