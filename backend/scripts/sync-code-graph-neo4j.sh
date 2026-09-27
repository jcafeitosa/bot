#!/usr/bin/env bash
# Rebuild the AST code graph (graphify) and MERGE-push it into local Neo4j (docker-compose.bot.yml service `graph`).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v graphify >/dev/null 2>&1; then
  echo "graphify not found; install graphifyy (pipx install graphifyy) and ensure neo4j driver: pipx inject graphifyy neo4j" >&2
  exit 1
fi

ENV_FILE="${ROOT}/.env"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "${ENV_FILE}"
  set +a
fi

: "${BOT_NEO4J_URI:?set BOT_NEO4J_URI in backend/.env}"
: "${BOT_NEO4J_USER:?set BOT_NEO4J_USER in backend/.env}"
: "${BOT_NEO4J_PASSWORD:?set BOT_NEO4J_PASSWORD in backend/.env}"

echo "Updating AST graph in ${ROOT}/graphify-out ..."
graphify update . --no-cluster

echo "Pushing to Neo4j at ${BOT_NEO4J_URI} ..."
graphify export neo4j --push "${BOT_NEO4J_URI}" --user "${BOT_NEO4J_USER}" --password "${BOT_NEO4J_PASSWORD}"

echo "Done. Open Neo4j Browser at http://127.0.0.1:7475 (Bolt: ${BOT_NEO4J_URI})."
