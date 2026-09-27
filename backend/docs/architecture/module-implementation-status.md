---
title: Status de implementação MVC mínimo (objetivo literal)
description: Checklist da árvore alvo core/modules/presentation com evidências e gates
tags:
  - architecture
  - mvc
  - compliance
---

# Status de implementação — MVC mínimo real

**Data da verificação:** 2026-09-27 (PG agents/bots write-through + hydrate)  
**Escopo:** árvore alvo do objetivo literal (com PG opcional em runtime (fail-closed), sem live trading, sem `technical_analysis`).  
**Correção aplicada nesta verificação:** `InMemoryBotCatalogStore` deixou de ser `#[cfg(test)]` para compilar o seam HTTP de catálogo de bots (`presentation/http/state.rs`, `http_bridge/bots.rs`).  
**Fatia pós-goal:** `0002_agents_bots_scaffold.sql` — schema PostgreSQL para agents/bots; espelho PG opcional (agents hydrate + write-through; bots `BotCatalogBackend`); seam HTTP admin (`BOT_HTTP_*`) + `require_bound_agency` em rotas agents. **Completude de produto** (orders live, runtime bots, auth owner): ver [auditoria de completude](../planning/modules-completeness-audit.md) — goal amplo **não fechado**.

## Gates (G4)

| Gate | Resultado | Evidência |
|------|-----------|-----------|
| `cargo fmt --check` | PASS | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | PASS | exit 0 |
| `cargo test --locked` | PASS | 237 testes (bin `bot`) no bin `bot`; 5 ignorados (dataset PG, scaffold, pg catalog, pg identity, Neo4j) |
| `./scripts/verify-backend-gates.sh (fmt, clippy, import-direction, tests)` | PASS | `OK: import direction heuristics passed` |

## Critério de linha

Para cada item: **existe no `src`**, com **models + controllers** (ou **adapters** no lugar de controllers onde o domínio é port/IO), e **testes unitários/integração** ou **uso em `main` / `presentation/http`**.

Legenda **MVC:** `M+C` = models + controllers; `M+A` = models + adapters; `Infra` = utilitário transversal (sem camada MVC clássica, coberto por uso em runtime).

## `core/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| config | `core/config/mod.rs` | Infra (`Config`, validação) | `core::config::tests`, `tests/config_cli.rs` |
| error | `core/error.rs` | Infra (`BotError`) | Usado em todos os módulos + HTTP |
| logging | `core/logging.rs` | Infra (`init`) | `main.rs` (`core::logging::init`) |
| database | `core/database/` | Infra (PG 18+, Neo4j, `AppDatabases`) | `core::database::tests`, HTTP bootstrap |
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
| agents | `modules/agents/` | M+C+A (`jev`, `pg_registry`) | `modules/agents/tests.rs`, HTTP agents + PG hydrate |
| bots | `modules/bots/` | M+C+A (`PgBotCatalogStore`, `BotCatalogBackend`) | `modules/bots/tests.rs`, HTTP bots routes |
| orders | `modules/orders/` | M+C+A (`FailClosedExecutor`) | `modules/orders/tests.rs`, HTTP 503 fail-closed |
| http_bridge | `modules/http_bridge/` | Facades por domínio | `bridge_tests::persist_catalog_bridge_wires_store_seam` |
| config_api | `modules/config_api.rs` | Reexport tipado (evita `presentation` → `core::config`) | Rotas HTTP `config` |
| application_contracts | `modules/application_contracts.rs` | Tipos compartilhados (`Signal`, `BotSignal`) | `application_contracts::tests` |

## `presentation/`

| Módulo | Caminho | MVC / seam | Testes ou wiring |
|--------|---------|------------|------------------|
| terminal | `presentation/terminal/` | View TUI | `monitor_state_tests`, monitor supervisor |
| http | `presentation/http/` | Rotas + `server` + `state` + `admin_auth` | `server::tests` (OpenAPI ~33 paths, admin bearer, agency bind, orders/bots/agents/monitor) |

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

**MVC_MINIMO_LITERAL_MET=yes** — árvore `core` / `modules` / `presentation` com seams e testes conforme tabelas acima; `./scripts/verify-backend-gates.sh` verde (**237** testes bin `bot`, **5** ignorados).

**COMPLETUDE_MODULOS_GOAL=parcial** — fundação bots/orders/agents/HTTP documentada e testada; pendem orders live, runtime bots, auth owner de produto e revisão Critic ([auditoria](../planning/modules-completeness-audit.md)).

**Raciocínio:** Todos os nós da árvore alvo existem em `src/`. Domínios expõem models + controllers/adapters com testes ou HTTP/main. Não há live trading nem `technical_analysis`. O veredito MVC **não** substitui o fechamento do goal de completude de módulos.
