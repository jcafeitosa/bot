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

> Revisão: 2026-09-27. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (**305** no bin `bot` + integração workspace).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross`/`ema_cross`), supervisor + backtest via `evaluate_for_kind`, catálogo HTTP `monitor_evaluator` | `monitor_strategy.rs`, `evaluation_binding.rs`, `simulation.rs`, `server.rs` | Auth owner; orders live |
| `modules/orders` | `RecordingExecutor`, `ReservedLiveExchangeExecutor`, idempotência, HTTP execution-status/meta | `orders/tests.rs`, `http_bridge/orders.rs` | Adapter exchange real |
| `modules/agents` | Registry + PG; `assert_runtime_promotion_authorized` (bot_id, capability, lifecycle) | `bot_promotion.rs`, `server.rs` | Auth owner produto |
| `presentation/http` | OpenAPI **34** paths; `GET /meta`; catálogo bots `monitor_evaluator`; contratos `meta_and_*`; `HttpAdminAuth` | `meta.rs`, `server.rs` (`bots_catalog_http_*`) | Auth owner produto (Gate 1) |

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

Equivale a: `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot`, `cargo test --locked` (integração workspace). PG opcional: `./scripts/verify-backend-full.sh` (ou `./scripts/run-pg-integration-tests.sh`) com `DATABASE_URL` → `trading_bot` (Timescale + pgvector).

Evidência (2026-09-27): **305** testes no binário `bot`, **6** ignorados (`persist_dataset_round_trip`, `postgres_scaffold_tables_exist_after_migrate`, `pg_catalog_store_round_trip`, `pg_identity_snapshot_round_trip`, `pg_order_idempotency_round_trip`, Neo4j integration). Estabilidade: 5× `cargo test --locked --bin bot` sem falhas; HTTP `server.rs` usa `fresh_agent_registry()` por teste.

## Documentação relacionada

- [CLI e variáveis HTTP](../reference/cli-and-config.md) (`BOT_HTTP_*`, `BOT_ORDERS_EXECUTION`, `BOT_RUNTIME_ENABLED`, `client_order_id`)
- [module-catalog.md](../architecture/module-catalog.md)
- [module-implementation-status.md](../architecture/module-implementation-status.md) — MVC mínimo vs goal de completude (dois vereditos distintos)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md)



## Matriz de requisitos (objetivo)

| Requisito | Evidência | Status |
|-----------|-----------|--------|
| Completude bots | Registry + catálogo HTTP, runtime promote, supervisor + backtest `evaluate_for_kind` (SMA/EMA) | **Parcial** (sem orders live; auth owner) |
| Completude orders | `submit_order`, `RecordingExecutor` (fake port), execution-status, `live_exchange_not_wired`, idempotência | **Parcial** (adapter exchange ausente) |
| Completude agents | Registry + PG; `promote_runtime_bot` capability testada (`promotion_denied_when_capability_false`); HTTP + `HttpAdminAuth` | **Parcial** (auth owner produto) |
| Integração HTTP + camadas | OpenAPI **34** paths; `GET /meta`; `config/active` + catálogo com `evaluator`/`monitor_evaluator`; `meta_and_*`; orders/bots/agents v1; PG hydrate | **Parcial** (auth owner, exchange adapter) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | 305 + clippy/fmt/import; PG 5/5 opcional (`run-pg-integration-tests.sh`) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots (`MonitorEvaluatorKind`, catálogo/config HTTP), orders seams, agents PG + promote capability | **Parcial** (auth owner; adapter exchange orders) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → 305 ok; clippy/fmt/import check | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Roadmap de gates (pós-G1)

| Gate | Módulo | SDD | Implementado |
|------|--------|-----|--------------|
| G1 PG scaffold | agents + bots catálogo | [bots-catalog-persistence-gate1-sdd.md](../sdd/bots-catalog-persistence-gate1-sdd.md) | **Parcial** (adapters + `run-pg-integration-tests.sh` + CI `postgres-integration`; default `cargo test` ignora PG) |
| G1 HTTP admin seam | presentation/http | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md) | **Sim** (não é auth owner produto) |
| G2 orders live | orders + idempotência HTTP `client_order_id` (memória) | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | **Parcial** (`HttpOrderExecutor` + `BOT_ORDERS_EXECUTION`; `live_exchange`/`paper` → `ReservedLiveExchangeExecutor` + **503** `live_exchange_not_wired`; `client_order_id` + memória + `PgOrderIdempotencyStore` opcional; sem adapter exchange real) |
| G2 bots runtime | bots + monitor + agents `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | **Parcial** (`MonitorEvaluatorKind` SMA/EMA no supervisor + `run_sma_crossover`; catálogo `monitor_evaluator`) |
| Auth owner produto | agents | [agents-capability-research.md](../research/agents-capability-research.md) | **Bloqueado** na pesquisa |

## Fechamento do goal (pendente)

Implementar itens **Não** nos checklists [orders G2](../sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist) e [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist), auth owner verificável, revisão Critic AGENTS.md. Baseline: `./scripts/verify-backend-gates.sh` → **305** testes bin `bot`, **6** ignorados; OpenAPI **34** paths.

| Próxima fatia (escolha) | SDD | Bloqueio típico |
|-------------------------|-----|-----------------|
| Adapter exchange orders | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | Threat model + Critic |
| Auth owner verificável | [agents-capability-research.md](../research/agents-capability-research.md) | Bootstrap + segurança |
