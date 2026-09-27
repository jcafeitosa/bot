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

> Revisão: 2026-09-27. Fonte: `backend/src`, SDDs em `docs/sdd/`, verificação `cargo test --locked` (**387** no bin `bot` + integração workspace).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross`/`ema_cross`), supervisor + backtest via `evaluate_for_kind`, catálogo HTTP `monitor_evaluator` | `monitor_strategy.rs`, `evaluation_binding.rs`, `simulation.rs`, `server.rs` | Auth owner; orders live |
| `modules/orders` | Paper/recording/testnet, idempotência+PG, reconciliação+poll (`LiveExchangeSpotOrderReconciliationQuery` + testnet observe), `SpotOrderSubmitAck` | `spot_order_reconciliation_query.rs`, `binance_spot_testnet_reconcile.rs`, `state.rs` | Prod REST; threat model/Critic |
| `modules/portfolio` | `paper_snapshot_with_fills` + posições com `fill_unit_price`/`BOT_PAPER_FILL_UNIT_PRICE`; HTTP `positions[]` | `controllers.rs`, `http_bridge/portfolio.rs`, `paper_ledger_executor.rs` | Preço de mercado dinâmico (não só env fixo) |
| `modules/agents` | Registry + PG; `assert_runtime_promotion_authorized` (bot_id, capability, lifecycle) | `bot_promotion.rs`, `server.rs` | Auth owner produto |
| `presentation/http` | OpenAPI **36** paths; `GET /meta`; reconciliação orders; `meta_and_*`; seam `HttpAdminAuth` com matriz 401/2xx em [test-matrix](../reference/test-matrix.md) | `server.rs`, `state.rs`, `verify-backend-gates.sh` (**387** testes) | Auth owner produto (Gate 1) |

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

Equivale a: `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot -- --test-threads=1`, `cargo test --locked` (integração workspace). PG opcional: `./scripts/verify-backend-full.sh` (ou `./scripts/run-pg-integration-tests.sh`) com `DATABASE_URL` → `trading_bot` (Timescale + pgvector).

Evidência (2026-09-27): **387** testes no binário `bot`, **9** ignorados (contagem na linha final de `verify-backend-gates.sh`: `OK: backend gates passed (bin bot: test result: …)`) (PG×7 incl. `pg_agent_lifecycle_write_through_round_trip` em `http_bridge/agents.rs`, Neo4j, testnet manual). `./scripts/verify-backend-gates.sh` verde; gate canônico usa `cargo test --locked --bin bot -- --test-threads=1` (`verify-backend-gates.sh`); stress local opcional `--test-threads=8` ~5s quando locks env→ledger respeitados. `./scripts/run-pg-integration-tests.sh` **7/7** com `DATABASE_URL`. HTTP `server.rs` usa `fresh_agent_registry()` por teste; ledger partilhado de orders: `lock_shared_live_order_reconciliation_ledger_for_test()` (env antes do ledger quando ambos) — [test-matrix](../reference/test-matrix.md).

## Documentação relacionada

- [CLI e variáveis HTTP](../reference/cli-and-config.md) (`BOT_HTTP_*`, `BOT_ORDERS_EXECUTION`, `BOT_RUNTIME_ENABLED`, `client_order_id`)
- [module-catalog.md](../architecture/module-catalog.md)
- [module-implementation-status.md](../architecture/module-implementation-status.md) — MVC mínimo vs goal de completude (dois vereditos distintos)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md)



## Matriz de requisitos (objetivo)

| Requisito | Evidência | Status |
|-----------|-----------|--------|
| Completude bots | Registry + catálogo HTTP, runtime promote, supervisor testnet→orders (`client_order_id` + ledger partilhado), backtest `evaluate_for_kind` | **Parcial** (PG mirror monitor; auth owner) |
| Completude orders | Paper/recording/testnet, idempotência PG, reconciliação+poll (recording/testnet observe), redação `BINANCE_TESTNET_*` em erros mapeados, `SpotOrderSubmitAck` | **Parcial** (prod REST; threat model/Critic) |
| Completude agents | Registry + PG; `promote_runtime_bot` capability testada (`promotion_denied_when_capability_false`); HTTP + `HttpAdminAuth` | **Parcial** (auth owner produto) |
| Integração HTTP + camadas | OpenAPI **36** paths; `GET /meta`; `meta_and_*`; orders/bots/agents v1; paper `orders`→`portfolio` (`paper_fill_unit_price`); testnet ccxt opt-in; reconciliação GET/POST poll; PG hydrate | **Parcial** (auth owner; prod REST política) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | **387** + clippy/fmt/import; PG 7/7 opcional (`verify-backend-full.sh`; evidência local com `DATABASE_URL`) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots runtime/evaluator, orders G2 (testnet+reconciliação+poll+redação credenciais), agents PG + promote; `.env.example` seams HTTP | **Parcial** (auth owner; Critic; prod REST política) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `cargo test --locked` → **387** ok; clippy/fmt/import check | Feito nesta revisão |
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

Implementar itens **Não** nos checklists [orders G2](../sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist) e [agents G1](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist); revisão Critic AGENTS.md. Baseline: `./scripts/verify-backend-gates.sh` → **387** testes bin `bot`, **9** ignorados; OpenAPI **36** paths.

| Próxima fatia (escolha) | SDD | Bloqueio típico |
|-------------------------|-----|-----------------|
| Threat model G2 fechado (Critic) + purge PG automatizado | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | Critic independente; retenção ops já em [cli-and-config](../reference/cli-and-config.md#pg-orders-retention-gate-2) |
| Auth owner verificável | [agents-capability-research.md](../research/agents-capability-research.md) | Bootstrap + decisão produto |
| Revisão Critic pacote G1/G2 | AGENTS.md | Instância separada |

### Decisões fora do código (bloqueiam fechamento do goal)

1. **Auth owner** — transporte e bootstrap conforme [agents-capability-research.md](../research/agents-capability-research.md); `BOT_HTTP_*` não substitui.
2. **Critic** — sessão independente com [handoff](#pacote-para-revisão-critic-handoff) abaixo; itens **Não** nos checklists G1/G2 só saem com LGTM registrado.
3. **Prod REST** — permanece bloqueado até decisão explícita (`modules/exchanges/rest.rs`); não confundir com testnet/paper G2.

## Pacote para revisão Critic (handoff)

Escopo sugerido para uma instância **independente** (não substitui decisão de auth owner produto):

| Área | Artefatos | Verificação mínima |
|------|-----------|-------------------|
| HTTP admin seam | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md), `presentation/http/admin_auth.rs`, matriz em [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token) | `./scripts/verify-backend-gates.sh`; spot-check `agents_advisory_requires_admin_bearer_when_enabled` se flake reaparecer |
| Orders G2 | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) (checklist + threat model), `modules/orders/`, `order_execution.rs`, `binance_spot_testnet_submit.rs` (`redact_known_testnet_credentials`) | Confirmar `authorize_rest_use` / prod REST bloqueado; retenção ops documentada; teste `map_bot_error_redacts_*`; sem credenciais em CI |
| Bots runtime G2 | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md), `evaluation_binding.rs`, `runtime_port.rs` | Promote capability + `evaluate_for_kind`; runtime partilhado vs isolado em testes HTTP (linha checklist **Parcial**) |
| Agents G1 | [agents-module-sdd.md](../sdd/agents-module-sdd.md), `bot_promotion.rs`, `http_bridge/agents.rs` (`pg_agent_lifecycle_write_through_round_trip`) | Itens **Não** do checklist permanecem bloqueadores de produto; PG 7/7 opcional |

Comandos canônicos: `./scripts/verify-backend-gates.sh`; opcional `DATABASE_URL=… ./scripts/run-pg-integration-tests.sh` (7 testes); testnet manual `cargo test --locked integration_submits_minimal_market_buy_on_testnet -- --ignored` (fora de CI).

Entrega esperada do Critic: veredito **APROVADO** / **APROVADO COM FOLLOW-UP** / **REPROVADO** por SDD, com achados ligados a teste ou linha de código; autor do pacote não aprova o próprio artefato (`AGENTS.md`).
