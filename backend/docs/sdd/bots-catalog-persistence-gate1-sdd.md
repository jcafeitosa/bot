---
title: SDD — Gate 1 persistência agents/bots (PostgreSQL)
description: Schema scaffold, stores PostgreSQL e critérios de aceite
tags:
  - sdd
  - backend
  - bots
  - agents
  - persistence
status: draft
---

# SDD — Gate 1: PostgreSQL para agents e catálogo bots

- **Estado:** parcial — migrações em `src/core/database/migrations/`; **`PgBotCatalogStore`** e **`PgAgentIdentityStore`** (upsert, eventos, `load_snapshot`); HTTP espelha mutações e `server::run` hidrata registry vazio; **auth owner** e promoção SDD `approved` pendentes.
- **Referências:** [SDD bots](./bots-module-sdd.md), [SDD agents](./agents-module-sdd.md), [core database](./core-database-sdd.md).

## Implementado

| Área | Artefato | Comportamento |
|------|----------|---------------|
| Bots | `PgBotCatalogStore`, `BotCatalogBackend` | Persist/snapshot HTTP usa PG quando pool disponível; `server::run` faz write-through best-effort do catálogo no boot quando `DATABASE_URL` conecta |
| Agents | `PgAgentIdentityStore`, `persist_identity_rows` | Write-through após register/pause/resume/retire |
| Agents boot | `load_agent_identity_snapshot` + `apply_agent_identity_snapshot` | Cold-start quando registry compartilhado vazio |
| Schema | `0002_agents_bots_scaffold.sql`, `0005_agent_promote_runtime_bot.sql` | `agent_*`, `bot_catalog_entries`, capability `promote_runtime_bot` |

Testes ignorados (PG): ver `./scripts/run-pg-integration-tests.sh` (**12** testes: scaffold, market dataset, bots catalog adapter + `ApiState` snapshot, agents identity + lifecycle write-through + cold-start hydrate, orders adapter idempotency/reconciliation + HTTP idempotency submit + hydrate + PG lookup fallback).

## Pendente

- Auth owner verificável no transporte HTTP.
- Revisão Critic independente (AGENTS.md) e status SDD `approved`.
- Runtime bots live e orders exchange (fora deste SDD).

## Validação

```text
./scripts/verify-backend-gates.sh
./scripts/run-pg-integration-tests.sh   # requer DATABASE_URL → trading_bot
```

## Rollback

`BotCatalogBackend::from_databases` força memória; remover chamadas PG em `routes/agents.rs` e boot em `server.rs`.
