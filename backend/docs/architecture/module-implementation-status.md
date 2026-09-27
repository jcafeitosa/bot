---
title: Status de implementação MVC mínimo (objetivo literal)
description: Checklist da árvore alvo core/modules/presentation com evidências e gates
tags:
  - architecture
  - mvc
  - compliance
---

# Status de implementação — MVC mínimo real

**Data da verificação:** 2026-09-27 (`./scripts/verify-backend-gates.sh` → **470** passed, **0** ignored; PG **22/22** com `DATABASE_URL` via `run-pg-integration-tests.sh`). Orders PG indisponível → `OrdersError::StoreUnavailable` / HTTP **503** `order_store_unavailable`.  
**Escopo:** árvore alvo do objetivo literal (com PG opcional em runtime (fail-closed), sem live trading, sem `technical_analysis`).  
**Fatia goal completude (HTTP):** `presentation/http/http_integration_tests.rs` — bearer admin, owner bind verificável no seam (`BOT_HTTP_OWNER_ID` + token), orders executors, portfolio paper, catálogo `monitor_registry` v2; smoke/meta/OpenAPI em `server.rs`. Baseline docs: linha `OK:` de `verify-backend-gates.sh`.  
**Fatia PG / grafo:** agents/bots write-through + hydrate; orders `0004`/`0006`; `provider_credentials` `0007`; **F2.1** outbox Neo4j `0009` (`5061a14f`); boot `build_api_state_for_http_serve` (**22/22** script). **Gate 1 HTTP (parcial):** [http-admin-auth-seam-sdd](../sdd/http-admin-auth-seam-sdd.md) — seam `BOT_HTTP_*` fechado; bootstrap PG `0010` parcial ([owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)); IdP pendente. **Completude de produto:** [auditoria de completude](../planning/modules-completeness-audit.md) — goal amplo **não fechado** (IdP + Critic AGENTS.md).

## Gates (G4)

| Gate | Resultado | Evidência |
|------|-----------|-----------|
| `cargo fmt --check` | PASS | exit 0 |
| `cargo clippy --locked --bin bot -- -D warnings` | PASS | mesmo escopo que `verify-backend-gates.sh` |
| `cargo test --locked` | PASS | **470** testes (bin `bot`), **0** ignorados; integração PG/Neo4j/testnet via `pg_integration` (skip sem env; PG **22/22** no script com `DATABASE_URL`) |
| `./scripts/verify-backend-gates.sh` | PASS | fmt + clippy `--bin bot` + import-direction + `assert-pg-integration-manifest.sh` + `cargo test --bin bot -- --test-threads=1` + 5 suítes `tests/*`; linha `OK:` com resumo `test result:` |
| `./scripts/verify-backend-full.sh` | PASS (com `DATABASE_URL` → `trading_bot`) | gates + PG **22/22**; mensagem `OK: backend full verification passed` |

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
| orders | `modules/orders/` | M+C+A + Neo4j F3 `OrderIntent` redigido (best-effort pós-submit) | `modules/orders/tests.rs`, `adapters/graph_projection.rs`, `http_bridge/orders`, HTTP 422/503 + reconciliação; [SDD](../sdd/orders-neo4j-projection-sdd.md) |
| http_bridge | `modules/http_bridge/` | Facades por domínio | `bridge_tests::persist_catalog_bridge_wires_store_seam` |
| config_api | `modules/config_api.rs` | Reexport tipado (evita `presentation` → `core::config`) | Rotas HTTP `config` |
| application_contracts | `modules/application_contracts.rs` | Tipos compartilhados (`Signal`, `BotSignal`) | `application_contracts::tests` |

## `presentation/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| terminal | `presentation/terminal/` | View TUI | `monitor_state_tests`, monitor supervisor |
| http | `presentation/http/` | Rotas + `server` + `state` + `admin_auth` | `server::tests` (OpenAPI **36**, `meta_and_*`, smoke bots/orders); `admin_auth.rs` (owner bind só com token); `http_integration_tests.rs` (**48** `http_integration`, admin bearer + owner bind — [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token)) |

## Restrições do objetivo literal

| Restrição | Status |
|-----------|--------|
| Sem live trading / ordens reais | OK — `orders::FailClosedExecutor`, `ExecutionDisabled`, REST order paths disabled em exchanges |
| Sem módulo `technical_analysis` | OK — ausente em `src/` |
| Agents / bots / orders fail-closed | OK — orders 503 após risco; bots catálogo PG ou memória; agents advisory sem autoridade de ordem |

## Lacunas conhecidas (não bloqueiam o objetivo literal)

- `PgBotCatalogStore` wired quando `DATABASE_URL` ok; `PgAgentIdentityStore` + hydrate no `serve`; **Gate 1 HTTP seam** fechado ([SDD](../sdd/http-admin-auth-seam-sdd.md)); auth owner humano verificável (IdP/bootstrap) ainda pendente.
- `core/error` e `core/logging` sem testes dedicados: aceitável como infraestrutura com cobertura indireta.

## Veredito

**MVC_MINIMO_LITERAL_MET=yes** — árvore `core` / `modules` / `presentation` com seams e testes conforme tabelas acima; `./scripts/verify-backend-gates.sh` verde (**470** testes bin `bot`, **0** ignorados).

**COMPLETUDE_MODULOS_GOAL=parcial** — veredito e [ENTREGA G4 Builder](../planning/modules-completeness-audit.md#entrega-pacote-completude-módulos--g4-builder) na [auditoria de completude](../planning/modules-completeness-audit.md). Fundação bots/orders/agents/HTTP documentada e testada (**470** testes, **0** ignorados; PG **22/22** via `verify-backend-full.sh`); **Gate 1 HTTP** (admin bearer + owner bind no seam) fechado; bots runtime parcial (`MonitorEvaluatorKind`, catálogo `monitor_evaluator`); orders G2 parcial (idempotência PG `try_claim`/`release_claim`); **bloqueadores:** IdP (bootstrap PG `0010` parcial) e revisão Critic independente ([fechamento do goal](../planning/modules-completeness-audit.md#fechamento-do-goal-pendente)).

**Raciocínio:** Todos os nós da árvore alvo existem em `src/`. Domínios expõem models + controllers/adapters com testes ou HTTP/main. Não há live trading nem `technical_analysis`. O veredito MVC **não** substitui o fechamento do goal de completude de módulos.
