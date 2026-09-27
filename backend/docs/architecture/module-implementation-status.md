---
title: Status de implementação MVC mínimo (objetivo literal)
description: Checklist da árvore alvo core/modules/presentation com evidências e gates
tags:
  - architecture
  - mvc
  - compliance
---

# Status de implementação — MVC mínimo real

**Data da verificação:** 2026-09-27 (`./scripts/verify-backend-gates.sh` → **518** passed, **0** ignored; manifesto PG **28** em `run-pg-integration-tests.sh` — contagem estática, execução não registrada; CI `backend-ci.yml` sem run verde (0/511 runs `success` até 27/09)). **C17 fatia 1:** snapshot PG do supervisor (`0011`); **fatia 2:** `PersistenceStatus::Gap` + REST `persistence_status` — [monitor-persistence-c17-sdd](../sdd/monitor-persistence-c17-sdd.md). Orders PG indisponível → `OrdersError::StoreUnavailable` / HTTP **503** `order_store_unavailable`. Retenção ops Gate 2: CLI `bot orders retention-purge` ([cli-and-config § retention](../reference/cli-and-config.md#pg-orders-retention-gate-2)).  
**Escopo:** árvore alvo do objetivo literal (com PG opcional em runtime (fail-closed), sem live trading, sem `technical_analysis`).  
**Fatia goal completude (HTTP):** `presentation/http/http_integration_tests.rs` — **62** testes `http_integration` (bearer admin, owner bind verificável no seam (`BOT_HTTP_OWNER_ID` + token), orders executors, portfolio paper, catálogo `monitor_registry` v2, F3 admin graph read-only agents/supervision-chain/bots-for-agent); smoke/meta/OpenAPI em `server.rs`. Baseline docs: linha `OK:` de `verify-backend-gates.sh`.  
**Stack Neo4j (runtime):** `BOT_GRAPH_ENABLED` (preferido) ou legado `BOT_AGENTS_ENABLED` + `BOT_NEO4J_*` ([postgres-and-graph-dev](../operations/postgres-and-graph-dev.md)).  
**Stack Neo4j (runtime):** `BOT_GRAPH_ENABLED` (preferido) ou legado `BOT_AGENTS_ENABLED` + `BOT_NEO4J_*` ([postgres-and-graph-dev](../operations/postgres-and-graph-dev.md)).  
**Fatia PG / grafo:** agents/bots write-through + hydrate; orders `0004`/`0006` + purge retenção (`orders retention-purge`); `provider_credentials` `0007`; **F2.1** outbox Neo4j `0009`; **F2.1.3+** enqueue outbox na mesma TX de domínio **fechado** nos caminhos principais (orders idempotency, agents identity, bots catalog, monitor supervisor); **C17 fatia 1** `monitor_supervisor_snapshot` `0011`; **fatia 2** mapeamento TUI → `MonitorSnapshot` / REST `persistence_status`; boot `build_api_state_for_http_serve` (manifesto PG **28**). **F3** leitura `GraphQueryPort` + CLI `graph query` completa; HTTP admin read-only advisory (`GET /admin/graph/agents`, `supervision-chain`, `bots-for-agent`, `code-impact` — [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md)). **Gate 1 HTTP (parcial):** [http-admin-auth-seam-sdd](../sdd/http-admin-auth-seam-sdd.md) — seam `BOT_HTTP_*` fechado; bootstrap PG `0010` parcial ([owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)); IdP pendente. **Completude de produto:** [auditoria de completude](../planning/modules-completeness-audit.md) — goal amplo **não fechado** (IdP + Critic AGENTS.md).

## Gates (G4)

| Gate | Resultado | Evidência |
|------|-----------|-----------|
| `cargo fmt --check` | PASS | exit 0 |
| `cargo clippy --locked --bin bot -- -D warnings` | PASS local / **FAIL no CI** | mesmo escopo que `verify-backend-gates.sh`; CI run #510 (27/09 15:56 COT) falhou com `clippy::result_large_err` em `modules/exchanges/adapters/live.rs:165` (clippy do Rust stable 1.98 no runner; toolchain não fixado) |
| `cargo test --locked` | PASS | **518** testes (bin `bot`), **0** ignorados; integração PG/Neo4j/testnet via `pg_integration` (skip sem env; manifesto PG **28** no script, contagem estática) |
| `./scripts/verify-backend-gates.sh` | PASS | fmt + clippy `--bin bot` + import-direction + `assert-pg-integration-manifest.sh` + `cargo test --bin bot -- --test-threads=1` + 5 suítes `tests/*`; linha `OK:` com resumo `test result:` |
| `./scripts/verify-backend-full.sh` | **Não registrado** (exige `DATABASE_URL` → `trading_bot`, PG 18+) | gates + manifesto PG **28**; nenhuma execução registrada em `modules-completeness-evidence.json`; job CI `postgres-integration` nunca executou com sucesso |

## Critério de linha

Para cada item: **existe no `src`**, com **models + controllers** (ou **adapters** no lugar de controllers onde o domínio é port/IO), e **testes unitários/integração** ou **uso em `main` / `presentation/http`**.

Legenda **MVC:** `M+C` = models + controllers; `M+A` = models + adapters; `Infra` = utilitário transversal (sem camada MVC clássica, coberto por uso em runtime).

## `core/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| config | `core/config/mod.rs` | Infra (`Config`, validação) | `core::config::tests`, `tests/config_cli.rs` |
| error | `core/error.rs` | Infra (`BotError`) | Usado em todos os módulos + HTTP |
| logging | `core/logging.rs` | Infra (`init`) | `main.rs` (`core::logging::init`) |
| database | `core/database/` | Infra (PG 18+, Neo4j F1/F2 projection, `AppDatabases`) | boot unificado `bootstrap_runtime` / monitor / backtest; `Neo4jBotProjector` ([SDD](../sdd/bots-neo4j-projection-sdd.md)) |
| persistence | `core/persistence/` | Infra (`Database`, dataset, `pg_integration`) | `market/models` PG round-trip (skip sem env), monitor startup |
| health | `core/health/mod.rs` | Infra (liveness/readiness) | `core::health::tests` |
| notifications | `core/notifications/` | Infra + `stub` adapter | `core::notifications::tests` |
| providers | `core/providers/` | `jev`: M+C+A; NIM/nine_router/openai: adapters HTTP | Testes em cada provider; Jev via `agents` |

## `modules/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| market | `modules/market/` | M+C+A (`adapters/persistence`) | Feed + models tests |
| strategy | `modules/strategy/` | M+C | `modules::strategy::tests` |
| risk | `modules/risk/` | M+C (`models.rs`, `controllers.rs`) | `modules::risk::tests` |
| portfolio | `modules/portfolio/` | M+C | `modules::portfolio::tests` |
| backtest | `modules/backtest/` | M+C + `cli` | simulation tests, `tests/backtest_fixture.rs` |
| exchanges | `modules/exchanges/` | M+A; `controllers` reexporta orquestração | `exchanges::tests`, adapters tests |
| monitor | `modules/monitor/` | M+C+views | supervisor/handle/startup tests; `main` monitor path |
| agents | `modules/agents/` | M+C+A (`jev`, `pg_registry`, `bot_promotion`) | `modules/agents/tests.rs`, HTTP agents + PG hydrate; `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` |
| bots | `modules/bots/` | M+C+A (`PgBotCatalogStore`, `BotRuntimePort`, `BotCatalogBackend`, Neo4j F2) | `modules/bots/tests.rs`, HTTP catalog + `/bots/runtime/*`, `graph_projection` best-effort; supervisor `strategy_evaluation_binding` |
| orders | `modules/orders/` | M+C+A + Neo4j F3 `OrderIntent` redigido (best-effort pós-submit) | `modules/orders/tests.rs`, `adapters/graph_projection.rs`, `http_bridge/orders`, HTTP 422/503 + reconciliação; CLI `orders retention-purge` (dry-run/`--apply`); [SDD](../sdd/orders-neo4j-projection-sdd.md) |
| http_bridge | `modules/http_bridge/` | Facades por domínio | `bridge_tests::persist_catalog_bridge_wires_store_seam` |
| config_api | `modules/config_api.rs` | Reexport tipado (evita `presentation` → `core::config`) | Rotas HTTP `config` |
| application_contracts | `modules/application_contracts.rs` | Tipos compartilhados (`Signal`, `BotSignal`) | `application_contracts::tests` |

## `presentation/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| terminal | `presentation/terminal/` | View TUI | `monitor_state_tests`, monitor supervisor |
| http | `presentation/http/` | Rotas + `server` + `state` + `admin_auth` | `server::tests` (OpenAPI **42**, `meta_and_*`, smoke bots/orders); `admin_auth.rs` (owner bind só com token); `http_integration_tests.rs` (**62** `http_integration`, admin bearer + owner bind + F3 graph admin — [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token)) |

## Restrições do objetivo literal

| Restrição | Status |
|-----------|--------|
| Sem live trading / ordens reais | OK — `orders::FailClosedExecutor` / `ExecutionDisabled` por padrão; produção bloqueada; monitor `--mode testnet` rejeitado na validação; submit Spot **testnet** (`ExchangeSpotExecutor`) só com opt-in `BOT_ORDERS_EXECUTION=live_exchange` + `BOT_ORDERS_EXCHANGE_SUBMIT=testnet` + `BINANCE_TESTNET_*` |
| Sem módulo `technical_analysis` | OK — ausente em `src/` |
| Agents / bots / orders fail-closed | OK — orders 503 após risco; bots catálogo PG ou memória; agents advisory sem autoridade de ordem |

## Lacunas conhecidas (não bloqueiam o objetivo literal)

- `PgBotCatalogStore` wired quando `DATABASE_URL` ok; `PgAgentIdentityStore` + hydrate no `serve`; **Gate 1 HTTP seam** fechado ([SDD](../sdd/http-admin-auth-seam-sdd.md)); auth owner humano verificável (IdP/bootstrap) ainda pendente.
- `core/error` e `core/logging` sem testes dedicados: aceitável como infraestrutura com cobertura indireta.

## Veredito

**MVC_MINIMO_LITERAL_MET=yes** — árvore `core` / `modules` / `presentation` com seams e testes conforme tabelas acima; `./scripts/verify-backend-gates.sh` verde (**518** testes bin `bot`, **0** ignorados).

**COMPLETUDE_MODULOS_GOAL=parcial** — veredito e [ENTREGA G4 Builder](../planning/modules-completeness-audit.md#entrega-pacote-completude-módulos--g4-builder) na [auditoria de completude](../planning/modules-completeness-audit.md). Fundação bots/orders/agents/HTTP documentada e testada (**518** testes, **0** ignorados — registro local; manifesto PG **28**, execução não registrada; **62** `http_integration`); **C17 fatia 1** snapshot PG supervisor + **fatia 2** `PersistenceStatus`/`persistence_status` REST ([SDD](../sdd/monitor-persistence-c17-sdd.md)); outbox **F2.1.3+** TX fechada nos caminhos principais; **Gate 1 HTTP** (admin bearer + owner bind no seam) fechado; F3 HTTP admin read-only (agents, supervision-chain, bots-for-agent, code-impact); bots runtime parcial (`MonitorEvaluatorKind`, catálogo `monitor_evaluator`); orders G2 parcial (idempotência PG `try_claim`/`release_claim`); **bloqueadores:** IdP (bootstrap PG `0010` parcial) e revisão Critic independente ([fechamento do goal](../planning/modules-completeness-audit.md#fechamento-do-goal-pendente)).

**Raciocínio:** Todos os nós da árvore alvo existem em `src/`. Domínios expõem models + controllers/adapters com testes ou HTTP/main. Não há live trading nem `technical_analysis`. O veredito MVC **não** substitui o fechamento do goal de completude de módulos.
