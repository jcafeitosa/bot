---
title: SDD — core::database (PostgreSQL 18+ dual-store)
description: Fachada dual-store PostgreSQL (TimescaleDB + pgvector) e Neo4j
tags:
  - sdd
  - backend
  - database
status: implemented
---

# SDD — `core::database`

## Contexto

PostgreSQL opcional (`DATABASE_URL` → `trading_bot`) e Neo4j opcional (`BOT_AGENTS_ENABLED` + `BOT_NEO4J_*`). Orders/agents/bots permanecem fail-closed; sem live trading.

## Stack PostgreSQL

- **Mínimo:** PostgreSQL **18+** (`server_version_num >= 180000`).
- **Extensões obrigatórias** quando PG está wired no health: `timescaledb`, `vector` (pgvector).
- Migrations em `src/core/database/migrations/` (`0000` extensões, `0001` market, `0002` agents/bots scaffold, `0003` vector scaffold).

## Neo4j

- Driver `neo4rs`; `Neo4jGraph::connect` / `ping`.
- Sem URI ou com `BOT_AGENTS_ENABLED=false`: grafo omitido (noop); readyz não exige Neo4j.

## Seams

| Seam | Uso |
|------|-----|
| `PostgresDatabase` | Pool, migrate, ping, `extension_health` |
| `Neo4jGraph` | Grafo agents/knowledge (futuro) |
| `AppDatabases::bootstrap_http_api` | HTTP `serve` |
| `core::persistence::Database` | Wrapper de domínio (datasets) |
| `health::readiness_databases` | `/readyz` |

## Dev

[postgres-and-graph-dev.md](../operations/postgres-and-graph-dev.md), `docker-compose.bot.yml`.

## Validação

`cargo fmt --check`, `clippy -D warnings`, `cargo test --locked`, `check-import-direction.sh`.
