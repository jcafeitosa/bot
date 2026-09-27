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

> Revisão: 2026-09-27. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (**299** no bin `bot` + integração workspace).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry`, `[[strategy.monitor_registry]]`, catálogo multi-estratégia, runtime HTTP | `monitor_strategy.rs`, `catalog.rs`, `server.rs` | Evaluators não-SMA; auth owner |
| `modules/orders` | `RecordingExecutor`, `ReservedLiveExchangeExecutor`, idempotência, HTTP execution-status/meta | `orders/tests.rs`, `http_bridge/orders.rs` | Adapter exchange real |
| `modules/agents` | Registry + PG; `assert_runtime_promotion_authorized` (bot_id, capability, lifecycle) | `bot_promotion.rs`, `server.rs` | Auth owner produto |
| `presentation/http` | OpenAPI **34** paths, `GET /meta` + contratos `meta_and_*_agree_on_*` (orders/bots), `HttpAdminAuth`, rotas v1 | `meta.rs`, `openapi.rs`, `server.rs` | Auth owner produto (Gate 1) |

Execução live e produção permanecem bloqueadas até gates de segurança.

## Persistência Gate 1 (scaffold)

- Migração SQL `0002_agents_bots_scaffold.sql` (agents + `bot_catalog_entries`); `Database::migrate()` no boot HTTP quando `DATABASE_URL` conecta.
- Teste ignorado `postgres_scaffold_tables_exist_after_migrate` em `core/persistence/mod.rs`.
- Adapter Rust e SDD completo: [Gate 1 draft](../sdd/bots-catalog-persistence-gate1-sdd.md).
- `core/database` expõe Neo4j opcional via `neo4rs` (`readyz` probe quando `BOT_AGENTS_ENABLED`).


## Verificação local

Gate canônico (recomendado):

```text
./scripts/verify-backend-gates.sh
```

Equivale a: `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot`, `cargo test --locked` (integração workspace). PG opcional: `./scripts/run-pg-integration-tests.sh` com `DATABASE_URL` → `trading_bot` (Timescale + pgvector).

Evidência (2026-09-27): **299** testes no binário `bot`, **6** ignorados (`persist_dataset_round_trip`, `postgres_scaffold_tables_exist_after_migrate`, `pg_catalog_store_round_trip`, `pg_identity_snapshot_round_trip`, `pg_order_idempotency_round_trip`, Neo4j integration). Estabilidade: 5× `cargo test --locked --bin bot` sem falhas; HTTP `server.rs` usa `fresh_agent_registry()` por teste.

## Documentação relacionada

- [CLI e variáveis HTTP](../reference/cli-and-config.md) (`BOT_HTTP_*`, `BOT_ORDERS_EXECUTION`, `BOT_RUNTIME_ENABLED`, `client_order_id`)
- [module-catalog.md](../architecture/module-catalog.md)
- [module-implementation-status.md](../architecture/module-implementation-status.md) — MVC mínimo vs goal de completude (dois vereditos distintos)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md)



## Matriz de requisitos (objetivo)

| Requisito | Evidência | Status |
|-----------|-----------|--------|
| Completude bots | Registry + catálogo HTTP, runtime promote, supervisor binding | **Parcial** (evaluators não-SMA; sem orders live) |
| Completude orders | `submit_order`, `RecordingExecutor` (fake port), execution-status, `live_exchange_not_wired`, idempotência | **Parcial** (adapter exchange ausente) |
| Completude agents | Registry + PG; `promote_runtime_bot` capability testada (`promotion_denied_when_capability_false`); HTTP + `HttpAdminAuth` | **Parcial** (auth owner produto) |
| Integração HTTP + camadas | OpenAPI **34** paths; `GET /meta` (`http_seams`) + testes `meta_and_*_agree_on_*`; `http_bridge` → domain; orders/bots/agents v1; PG boot hydrate | **Parcial** (auth owner, exchange adapter; evaluators não-SMA) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | 299 + clippy/fmt/import (2026-09-27) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots/agents PG best-effort, HTTP orders/bots/agents | **Parcial** (auth owner de produto, orders live, runtime bots) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → 299 ok; clippy/fmt/import check | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Roadmap de gates (pós-G1)

| Gate | Módulo | SDD | Implementado |
|------|--------|-----|--------------|
| G1 PG scaffold | agents + bots catálogo | [bots-catalog-persistence-gate1-sdd.md](../sdd/bots-catalog-persistence-gate1-sdd.md) | **Parcial** (código + testes `#[ignore]` PG) |
| G1 HTTP admin seam | presentation/http | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md) | **Sim** (não é auth owner produto) |
| G2 orders live | orders + idempotência HTTP `client_order_id` (memória) | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | **Parcial** (`HttpOrderExecutor` + `BOT_ORDERS_EXECUTION`; `live_exchange`/`paper` → `ReservedLiveExchangeExecutor` + **503** `live_exchange_not_wired`; `client_order_id` + memória + `PgOrderIdempotencyStore` opcional; sem adapter exchange real) |
| G2 bots runtime | bots + monitor + agents `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | **Parcial** (`MonitorStrategyRegistry` + `[[strategy.monitor_registry]]` no TOML, `build_catalog_from_monitor_registry`, HTTP promote/demote, `strategy_evaluation_binding`; evaluators não-SMA ainda ausentes) |
| Auth owner produto | agents | [agents-capability-research.md](../research/agents-capability-research.md) | **Bloqueado** na pesquisa |

## Fechamento do goal (pendente)

Implementar G2 orders e/ou G2 bots (com TDD + Critic), auth owner verificável, revisão Critic AGENTS.md sobre o pacote G1 entregue. Baseline reproduzível: `./scripts/verify-backend-gates.sh` → **299** testes bin `bot`, **6** ignorados; OpenAPI **34** paths (`openapi_surface_lists_core_paths` em `server.rs`).

| Próxima fatia (escolha) | SDD | Bloqueio típico |
|-------------------------|-----|-----------------|
| Adapter exchange orders | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | Threat model + Critic |
| Evaluators não-SMA no monitor | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | Backtest + supervisor além de SMA |
| Auth owner verificável | [agents-capability-research.md](../research/agents-capability-research.md) | Bootstrap + segurança |
