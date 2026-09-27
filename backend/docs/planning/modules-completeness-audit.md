---
title: Auditoria de completude — bots, orders, agents e HTTP
description: Estado verificável dos módulos alvo do goal, gaps, evidências de teste e próximos gates
tags:
  - planning
  - backend
  - modules
  - audit
---

# Auditoria de completude — bots, orders, agents e HTTP

> Revisão: 2026-09-27. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (192 unitários + integração).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | Fundação + ranking + `PgBotCatalogStore` + `BotCatalogBackend` + HTTP catalog/persist/snapshot | `modules/bots/tests.rs`, `pg_catalog.rs`, `server.rs` | Runtime live, auth, promoção executor |
| `modules/orders` | Seam fail-closed + HTTP 503 após risco | `modules/orders/tests.rs`, testes HTTP orders | Adapter exchange, idempotência, auth |
| `modules/agents` | IdentityOnly + HTTP lifecycle + `PgAgentIdentityStore` (bridge persist) + hook monitor | `modules/agents/tests.rs`, rotas agents | Auth owner; cold-start `server::run` hidrata registry vazio a partir de PG |
| `presentation/http` | OpenAPI ~30 paths, Scalar `/docs` | `openapi.rs`, `server.rs` | Authn/z transversal |

Execução live e produção permanecem bloqueadas até gates de segurança.

## Persistência Gate 1 (scaffold)

- Migração SQL `0002_agents_bots_scaffold.sql` (agents + `bot_catalog_entries`); `Database::migrate()` no boot HTTP quando `DATABASE_URL` conecta.
- Teste ignorado `postgres_scaffold_tables_exist_after_migrate` em `core/persistence/mod.rs`.
- Adapter Rust e SDD completo: [Gate 1 draft](../sdd/bots-catalog-persistence-gate1-sdd.md).
- `core/database` expõe Neo4j opcional via `neo4rs` (`readyz` probe quando `BOT_AGENTS_ENABLED`).


## Verificação local

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./scripts/check-import-direction.sh
./scripts/verify-backend-gates.sh
```

Evidência: **192** testes no binário `bot`, **5** ignorados (`persist_dataset_round_trip`, `postgres_scaffold_tables_exist_after_migrate`, `pg_catalog_store_round_trip`, `pg_identity_snapshot_round_trip`, Neo4j integration).

## Documentação relacionada

- [module-catalog.md](../architecture/module-catalog.md)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md)



## Matriz de requisitos (objetivo)

| Requisito | Evidência | Status |
|-----------|-----------|--------|
| Completude bots | `modules/bots/`, `PgBotCatalogStore`, HTTP `/bots/*` | **Parcial** (sem runtime live) |
| Completude orders | `submit_order`, HTTP 422/503 | **Fundação** (fail-closed) |
| Completude agents | `AgentRegistry`, PG write-through + hydrate, `HttpAdminAuth` seam | **Parcial** (bearer opcional; rotas ainda sem gate) |
| Integração HTTP | OpenAPI 30 paths, `server.rs` testes | **Feito** |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | 192 + clippy/fmt/import (2026-09-27) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots/agents PG best-effort, HTTP orders/bots/agents | **Parcial** (auth owner nas rotas, orders live, runtime bots) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → 192 ok; clippy/fmt/import check | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Fechamento do goal (pendente)

Auth owner, adapter real de orders ([SDD orders](../sdd/orders-module-sdd.md)), runtime bots live, revisão Critic AGENTS.md. PG scaffold: [Gate 1](../sdd/bots-catalog-persistence-gate1-sdd.md).
