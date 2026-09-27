---
title: Status de implementação MVC mínimo (objetivo literal)
description: Checklist da árvore alvo core/modules/presentation com evidências e gates
tags:
  - architecture
  - mvc
  - compliance
---

# Status de implementação — MVC mínimo real

**Data da verificação:** 2026-09-27 (`./scripts/verify-backend-full.sh` → **389** passed + **17** ignored, PG **15/15**).  
**Escopo:** árvore alvo do objetivo literal (com PG opcional em runtime (fail-closed), sem live trading, sem `technical_analysis`).  
**Fatia goal completude (HTTP):** `presentation/http/http_integration_tests.rs` — bearer admin, orders executors, portfolio paper, catálogo `monitor_registry` v2; smoke/meta/OpenAPI em `server.rs`. Baseline docs: linha `OK:` de `verify-backend-gates.sh`.  
**Fatia PG:** agents/bots write-through + hydrate; orders `0004`/`0006`; `provider_credentials` `0007`; boot `build_api_state_for_http_serve` (**15/15** script). Seam HTTP admin (`BOT_HTTP_*`) — não substitui auth owner. **Completude de produto:** [auditoria de completude](../planning/modules-completeness-audit.md) — goal amplo **não fechado** (auth owner + Critic AGENTS.md).

## Gates (G4)

| Gate | Resultado | Evidência |
|------|-----------|-----------|
| `cargo fmt --check` | PASS | exit 0 |
| `cargo clippy --locked --bin bot -- -D warnings` | PASS | mesmo escopo que `verify-backend-gates.sh` |
| `cargo test --locked` | PASS | 389 testes (bin `bot`); 17 ignorados (PG×15 no script + `loads_credentials_from_postgres`; Neo4j; testnet manual) |
| `./scripts/verify-backend-gates.sh` | PASS | fmt + clippy `--bin bot` + import-direction + `cargo test --bin bot -- --test-threads=1` + 5 suítes `tests/*` (sem `cargo test --locked` completo); linha `OK:` com resumo `test result:` |

## Critério de linha

Para cada item: **existe no `src`**, com **models + controllers** (ou **adapters** no lugar de controllers onde o domínio é port/IO), e **testes unitários/integração** ou **uso em `main` / `presentation/http`**.

Legenda **MVC:** `M+C` = models + controllers; `M+A` = models + adapters; `Infra` = utilitário transversal (sem camada MVC clássica, coberto por uso em runtime).

## `core/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| config | `core/config/mod.rs` | Infra (`Config`, validação) | `core::config::tests`, `tests/config_cli.rs` |
| error | `core/error.rs` | Infra (`BotError`) | Usado em todos os módulos + HTTP |
| logging | `core/logging.rs` | Infra (`init`) | `main.rs` (`core::logging::init`) |
| database | `core/database/` | Infra (PG 18+, Neo4j, `AppDatabases`) | boot unificado `bootstrap_runtime` / monitor / backtest ([SDD](../sdd/database-module-integration-sdd.md)) |
| persistence | `core/persistence/` | Infra (`Database`, dataset) | `market/models` integration (ignored PG), monitor startup |
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
| bots | `modules/bots/` | M+C+A (`PgBotCatalogStore`, `BotRuntimePort`, `BotCatalogBackend`) | `modules/bots/tests.rs`, HTTP catalog + `/bots/runtime/*`, supervisor `strategy_evaluation_binding` |
| orders | `modules/orders/` | M+C+A (`FailClosedExecutor`, `HttpOrderExecutor`, `OrderIdempotencyStore`, `PgOrderIdempotencyStore`) | `modules/orders/tests.rs`, `http_bridge/orders`, HTTP 422/503 + `dev_accept` + `client_order_id` |
| http_bridge | `modules/http_bridge/` | Facades por domínio | `bridge_tests::persist_catalog_bridge_wires_store_seam` |
| config_api | `modules/config_api.rs` | Reexport tipado (evita `presentation` → `core::config`) | Rotas HTTP `config` |
| application_contracts | `modules/application_contracts.rs` | Tipos compartilhados (`Signal`, `BotSignal`) | `application_contracts::tests` |

## `presentation/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| terminal | `presentation/terminal/` | View TUI | `monitor_state_tests`, monitor supervisor |
| http | `presentation/http/` | Rotas + `server` + `state` + `admin_auth` | `server::tests` (OpenAPI **36**, `meta_and_*`, smoke bots/orders); `http_integration_tests.rs` (admin bearer + orders executors — [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token)) |

## Restrições do objetivo literal

| Restrição | Status |
|-----------|--------|
| Sem live trading / ordens reais | OK — `orders::FailClosedExecutor`, `ExecutionDisabled`, REST order paths disabled em exchanges |
| Sem módulo `technical_analysis` | OK — ausente em `src/` |
| Agents / bots / orders fail-closed | OK — orders 503 após risco; bots catálogo PG ou memória; agents advisory sem autoridade de ordem |

## Lacunas conhecidas (não bloqueiam o objetivo literal)

- `PgBotCatalogStore` wired quando `DATABASE_URL` ok; `PgAgentIdentityStore` + hydrate no `serve`; auth owner verificável (Gate 1) ainda pendente.
- `core/error` e `core/logging` sem testes dedicados: aceitável como infraestrutura com cobertura indireta.

## Veredito

**MVC_MINIMO_LITERAL_MET=yes** — árvore `core` / `modules` / `presentation` com seams e testes conforme tabelas acima; `./scripts/verify-backend-gates.sh` verde (**389** testes bin `bot`, **17** ignorados).

**COMPLETUDE_MODULOS_GOAL=parcial** — fundação bots/orders/agents/HTTP documentada e testada; bots runtime parcial (`MonitorEvaluatorKind`, catálogo `monitor_evaluator`); pendem adapter exchange (orders G2), auth owner de produto e revisão Critic ([auditoria](../planning/modules-completeness-audit.md)).

**Raciocínio:** Todos os nós da árvore alvo existem em `src/`. Domínios expõem models + controllers/adapters com testes ou HTTP/main. Não há live trading nem `technical_analysis`. O veredito MVC **não** substitui o fechamento do goal de completude de módulos.
