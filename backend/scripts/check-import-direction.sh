#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/src"
fail=0
check() {
  local desc="$1" pattern="$2" path="$3" exclude="${4:-}"
  local out
  if out=$(rg -n "$pattern" "$path" 2>/dev/null || true); then
    if [[ -n "$exclude" ]]; then
      out=$(echo "$out" | rg -v "$exclude" || true)
    fi
    if [[ -n "$out" ]]; then
      echo "FAIL: $desc"
      echo "$out"
      fail=1
    fi
  fi
}
check "core must not import modules" 'use crate::modules::' "$SRC/core"
check "presentation must not import strategy" 'modules::strategy' "$SRC/presentation"
check "presentation must not import risk" 'modules::risk' "$SRC/presentation"
check "modules must not import presentation (except monitor supervisor spawn)" 'presentation::' "$SRC/modules" 'supervisor.rs'
check "http routes must not touch AgentRegistry (use ApiState)" 'with_agents' "$SRC/presentation/http/routes"
check "http routes must not import domain agents module" 'modules::agents::' "$SRC/presentation/http/routes"
check "http routes must not read ApiState config directly (use ApiState helpers)" 'app_config\(\)' "$SRC/presentation/http/routes"
legacy=$(find "$SRC" -maxdepth 1 -name '*.rs' ! -name main.rs 2>/dev/null || true)
if [[ -n "$legacy" ]]; then
  echo "FAIL: legacy .rs at src root (only main.rs allowed)"
  echo "$legacy"
  fail=1
fi
if [[ "$fail" -ne 0 ]]; then exit 1; fi
echo "OK: import direction heuristics passed"
