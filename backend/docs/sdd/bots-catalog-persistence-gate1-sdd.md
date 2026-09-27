---
title: SDD — Gate 1 persistência agents/bots (PostgreSQL)
description: Schema scaffold, repositórios Rust e critérios de aceite após stores em memória
tags:
  - sdd
  - backend
  - bots
  - agents
  - persistence
status: draft
---

# SDD — Gate 1: PostgreSQL para agents e catálogo bots

- **Estado:** draft — migração SQL **scaffold** aplicável via `Database::migrate()`; adapters Rust (`PostgresBotCatalogStore`, identity PG) **não** implementados.
- **Referências:** [SDD bots](./bots-module-sdd.md), [SDD agents](./agents-module-sdd.md), `src/core/persistence/migrations/0002_agents_bots_scaffold.sql`.

## Objetivo

1. Schema `0002_agents_bots_scaffold.sql` (tabelas `agent_identities`, `agent_identity_events`, `bot_catalog_entries`).
2. Repositórios Rust que leem/escrevem essas tabelas, preservando seams públicos dos módulos.
3. Testes de integração ignorados com `DATABASE_URL` → `trading_bot`.

## Schema (scaffold existente)

- **Agents:** `agent_identities`, `agent_identity_events` (eventos de lifecycle).
- **Bots:** `bot_catalog_entries` (linhas versionadas por `bot_id`, não snapshot JSONB).

## Validação

```text
cargo test --locked
cargo test postgres_scaffold_tables_exist_after_migrate -- --ignored
```

## Rollback

Reverter migração 0002 e adapters; HTTP e domínio permanecem em memória.
