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

> Revisão: 2026-09-27. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (**353** no bin `bot` + integração workspace).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross`/`ema_cross`), supervisor + backtest via `evaluate_for_kind`, catálogo HTTP `monitor_evaluator` | `monitor_strategy.rs`, `evaluation_binding.rs`, `simulation.rs`, `server.rs` | Auth owner; orders live |
| `modules/orders` | Paper/recording/testnet, idempotência+PG, reconciliação memória+PG+`GET/POST /orders/reconciliation/*` (incl. poll), `SpotOrderSubmitAck` | `orders.rs`, `state.rs`, `pg_reconciliation.rs`, `server.rs` | Poller HTTP (`POST …/poll`); divergência testnet/job; prod REST; threat model/Critic |
| `modules/portfolio` | `paper_snapshot_with_fills` + posições com `fill_unit_price`/`BOT_PAPER_FILL_UNIT_PRICE`; HTTP `positions[]` | `controllers.rs`, `http_bridge/portfolio.rs`, `paper_ledger_executor.rs` | Preço de mercado dinâmico (não só env fixo) |
| `modules/agents` | Registry + PG; `assert_runtime_promotion_authorized` (bot_id, capability, lifecycle) | `bot_promotion.rs`, `server.rs` | Auth owner produto |
| `presentation/http` | OpenAPI **36** paths; `GET /meta` (`order_reconciliation_pending`); catálogo bots `monitor_evaluator`; contratos `meta_and_*`; `HttpAdminAuth` | `meta.rs`, `server.rs` (`bots_catalog_http_*`) | Auth owner produto (Gate 1) |

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

Evidência (2026-09-27): **353** testes no binário `bot`, **8** ignorados (PG×6 incl. `pg_order_reconciliation_round_trip`, Neo4j, testnet manual). `./scripts/verify-backend-gates.sh` verde; `./scripts/run-pg-integration-tests.sh` **6/6** com `DATABASE_URL`. HTTP `server.rs` usa `fresh_agent_registry()` por teste.

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
| Completude orders | Paper/recording/testnet, idempotência PG, reconciliação memória+PG+HTTP GET, `SpotOrderSubmitAck` | **Parcial** (prod; poller divergência; threat model; Critic) |
| Completude agents | Registry + PG; `promote_runtime_bot` capability testada (`promotion_denied_when_capability_false`); HTTP + `HttpAdminAuth` | **Parcial** (auth owner produto) |
| Integração HTTP + camadas | OpenAPI **36** paths; `GET /meta`; `meta_and_*`; orders/bots/agents v1; paper `orders`→`portfolio` (`paper_fill_unit_price`); testnet ccxt opt-in; PG hydrate | **Parcial** (auth owner; reconciliação; prod) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | **353** + clippy/fmt/import; PG 6/6 opcional (`verify-backend-full.sh`) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots runtime/evaluator, orders G2 (testnet+reconciliação PG), agents PG + promote | **Parcial** (auth owner; poller; prod) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → **353** ok; clippy/fmt/import check | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Roadmap de gates (pós-G1)

| Gate | Módulo | SDD | Implementado |
|------|--------|-----|--------------|
| G1 PG scaffold | agents + bots catálogo | [bots-catalog-persistence-gate1-sdd.md](../sdd/bots-catalog-persistence-gate1-sdd.md) | **Parcial** (adapters + `run-pg-integration-tests.sh` + CI `postgres-integration`; default `cargo test` ignora PG) |
| G1 HTTP admin seam | presentation/http | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md) | **Sim** (não é auth owner produto) |
| G2 orders live | orders + idempotência + reconciliação | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | **Parcial** (paper/recording/testnet; reconciliação PG+HTTP; poller testnet/job periódico; threat model/Critic pendentes) |
| G2 bots runtime | bots + monitor + agents `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | **Parcial** (`MonitorEvaluatorKind` SMA/EMA no supervisor + `run_sma_crossover`; catálogo `monitor_evaluator`) |
| Auth owner produto | agents | [agents-module-sdd.md](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist), [agents-capability-research.md](../research/agents-capability-research.md) | **Bloqueado** (seam `BOT_HTTP_*` ok; owner humano não) |

## Fechamento do goal (pendente)

Implementar itens **Não** nos checklists [orders G2](../sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist) e [agents G1](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist); revisão Critic AGENTS.md. Baseline: `./scripts/verify-backend-gates.sh` → **353** testes bin `bot`, **8** ignorados; OpenAPI **36** paths.

| Próxima fatia (escolha) | SDD | Bloqueio típico |
|-------------------------|-----|-----------------|
| Poller reconciliação / divergência | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | Exchange status + Critic |
| Threat model G2 fechado | orders G2 SDD | Critic |
| Auth owner verificável | [agents-capability-research.md](../research/agents-capability-research.md) | Bootstrap + segurança |
