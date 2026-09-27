---
title: SDD — core::database dual-store (PostgreSQL 18+ + Neo4j)
description: Unified database seam, migrations, readiness probes, and Gate 1 bot catalog persistence
tags:
  - sdd
  - backend
  - database
  - persistence
  - neo4j
status: draft
---

# SDD — `core::database`

## Contexto e objetivo

Seam único para PostgreSQL 18+ (TimescaleDB + pgvector) e Neo4j opcional. `AppDatabases::bootstrap_runtime` (alias `bootstrap_http_api`) conecta; monitor/backtest via [database-module-integration-sdd.md](./database-module-integration-sdd.md). `AppDatabases::bootstrap_http_api` conecta PG quando `DATABASE_URL` aponta para `trading_bot` e Neo4j quando `BOT_AGENTS_ENABLED` está ativo. Falhas logam warn e o processo continua fail-closed.

## Seams

- `PostgresDatabase` — versão ≥ 18, extensões obrigatórias em readyz quando conectado.
- `PgBotCatalogStore` — tabela `bot_catalog_entries`; HTTP usa `BotCatalogBackend` (PG ou memória).
- `readiness_databases` — probes `postgres`, `postgres_extensions`, `neo4j` (se wired).

## Env

`DATABASE_URL`, `BOT_AGENTS_ENABLED`, `BOT_NEO4J_URI`, `BOT_NEO4J_USER`, `BOT_NEO4J_PASSWORD`, `BOT_NEO4J_DATABASE`.

Compose opcional: `docker-compose.bot.yml` (Neo4j + Postgres agents); market/catálogo exigem DB `trading_bot` provisionado à parte.

## Migrações

`src/core/database/migrations/` (`0000` extensions … `0010` product owner bootstrap). Nenhuma usa sintaxe exclusiva do PG 18; o mínimo PG 18 vem do check em runtime `assert_server_version` (`postgres.rs`, `MIN_SERVER_VERSION_NUM = 180000`).

## Validação

fmt, clippy -D warnings, cargo test --locked, check-import-direction; testes PG via `pg_integration` (skip sem `DATABASE_URL`) + manifesto `run-pg-integration-tests.sh` contra PG 18+ real. CI usa `timescaledb-ha:pg16` (divergente; issue aberta).

## Próximo

Auth owner HTTP e hardening de constraints de hierarquia em PG (pós-[agents-pg-registry-sdd](./agents-pg-registry-sdd.md)).
