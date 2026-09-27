#!/usr/bin/env bash
# Ensures docs/planning/modules-completeness-evidence.json matches gate + PG manifest (no DATABASE_URL).
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ -z "${BOT_PASSED:-}" ]]; then
  echo "FAIL: BOT_PASSED not set (call from verify-backend-gates.sh)" >&2
  exit 1
fi

python3 <<'PY'
import json
import os
import re
import sys
from pathlib import Path

passed = int(os.environ["BOT_PASSED"])
ev_path = Path("docs/planning/modules-completeness-evidence.json")
if not ev_path.is_file():
    sys.exit("FAIL: modules-completeness-evidence.json missing")
ev = json.loads(ev_path.read_text())
obs = ev.get("observed", {})
if obs.get("bot_tests_passed") != passed:
    sys.exit(
        f"FAIL: evidence bot_tests_passed={obs.get('bot_tests_passed')} != gate {passed}; "
        "update docs/planning/modules-completeness-evidence.json and audit baseline"
    )
if obs.get("bot_tests_ignored") != 0:
    sys.exit(f"FAIL: evidence bot_tests_ignored must be 0, got {obs.get('bot_tests_ignored')}")

sh = Path("scripts/run-pg-integration-tests.sh").read_text()
m = re.search(r"EXPECTED_PG_INTEGRATION_TESTS=(\d+)", sh)
if not m:
    sys.exit("FAIL: EXPECTED_PG_INTEGRATION_TESTS not found")
pg_expected = int(m.group(1))
if obs.get("pg_script_tests") != pg_expected:
    sys.exit(
        f"FAIL: evidence pg_script_tests={obs.get('pg_script_tests')} != manifest {pg_expected}"
    )

audit = Path("docs/planning/modules-completeness-audit.md").read_text()
needle = f"**{passed}**"
if needle not in audit:
    sys.exit(
        f"FAIL: modules-completeness-audit.md missing baseline {needle}; sync audit with gate"
    )

readme = Path("README.md").read_text()
if needle not in readme:
    sys.exit(f"FAIL: README.md missing baseline {needle}")

test_matrix = Path("docs/reference/test-matrix.md").read_text()
if needle not in test_matrix:
    sys.exit(f"FAIL: test-matrix.md missing baseline {needle}")

print(f"OK: completeness evidence aligned (bot {passed}, PG {pg_expected})")
PY
