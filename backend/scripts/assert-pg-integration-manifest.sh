#!/usr/bin/env bash
# Validates PG_TESTS count in run-pg-integration-tests.sh matches EXPECTED_PG_INTEGRATION_TESTS (no DATABASE_URL).
set -euo pipefail
cd "$(dirname "$0")/.."

python3 <<'PY'
import re
import sys
from pathlib import Path

sh = Path("scripts/run-pg-integration-tests.sh").read_text()
block = re.search(r"PG_TESTS=\((.*?)\n\)", sh, re.S)
if not block:
    sys.exit("FAIL: PG_TESTS block not found in run-pg-integration-tests.sh")
tests = [ln.strip() for ln in block.group(1).splitlines() if ln.strip()]
m = re.search(r"EXPECTED_PG_INTEGRATION_TESTS=(\d+)", sh)
if not m:
    sys.exit("FAIL: EXPECTED_PG_INTEGRATION_TESTS not set in run-pg-integration-tests.sh")
expected = int(m.group(1))
if len(tests) != expected:
    sys.exit(
        f"FAIL: PG_TESTS manifest drift: expected {expected}, got {len(tests)}"
    )
print(f"OK: PG integration manifest ({len(tests)} tests)")
PY
