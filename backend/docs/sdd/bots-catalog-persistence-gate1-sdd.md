---
title: SDD — Gate 1 persistência agents/bots (PostgreSQL)
description: Schema scaffold, PgBotCatalogStore e critérios de aceite
tags:
  - sdd
  - backend
  - bots
  - agents
  - persistence
status: draft
---

# SDD — Gate 1: PostgreSQL para agents e catálogo bots

- **Estado:** parcial — migrações em `src/core/database/migrations/` (incl. `0002_agents_bots_scaffold.sql`); **`PgBotCatalogStore`** implementado; `PgAgentIdentityStore` (upsert + events) via `http_bridge::agents::persist_identity_rows`; wiring automático do `AgentRegistry` e auth owner **pendentes**.
- **Referências:** [SDD bots](./bots-module-sdd.md), [SDD agents](./agents-module-sdd.md).

## Implementado

- `PgBotCatalogStore` (`modules/bots/adapters/pg_catalog.rs`) — `save_catalog` / `load_catalog` em `bot_catalog_entries`.
- `BotCatalogBackend::from_databases` — PostgreSQL quando `AppDatabases` tem pool; senão memória.
- Teste ignorado `pg_catalog_store_round_trip`.

## Pendente

- `PgAgentRegistry` / persistência de identities.
- Auth owner nas rotas HTTP.
- Evolução SDD para `approved` + revisão Critic (AGENTS.md).

## Validação

```text
cargo fmt --check && cargo clippy --locked -- -D warnings && cargo test --locked --bin bot
cargo test pg_catalog_store_round_trip -- --ignored
cargo test postgres_scaffold_tables_exist_after_migrate -- --ignored
```

## Rollback

Desabilitar seleção PG em `BotCatalogBackend::from_databases`; HTTP volta a memória apenas.
