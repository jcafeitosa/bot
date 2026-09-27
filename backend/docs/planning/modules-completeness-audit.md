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

> Revisão: 2026-09-27. Snapshot machine-readable: [modules-completeness-evidence.json](./modules-completeness-evidence.json). Fonte: `backend/src`, SDDs em `docs/sdd/`. Evidência: `./scripts/verify-backend-full.sh` (com `DATABASE_URL`) → gates **464**/**0** ignored + PG **21/21**; `cargo test --bin bot http_integration -- --test-threads=1` → **48** passed.
>
> **Bloqueio de fechamento:** fatia técnica/doc do goal entregue (`verify-backend-gates.sh` **464**/**0** ignored, PG **21/21**, `pg_integration`); **IdP / owner humano verificável** e **Critic** `AGENTS.md` permanecem bloqueadores (bootstrap PG `0010` é fatia parcial — [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)) — ver § [Fechamento do goal (pendente)](#fechamento-do-goal-pendente).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross`/`ema_cross`), supervisor + backtest via `evaluate_for_kind`, catálogo HTTP `monitor_evaluator` + períodos v1/v2 (`bots_catalog_http_lists_monitor_registry_v2_periods`); promote SMA/EMA via registry | `monitor_strategy.rs`, `http_bridge/bots.rs`, `evaluation_binding.rs`, `server.rs` | Auth owner; orders live |
| `modules/orders` | Paper/recording/testnet, idempotência+PG, reconciliação+poll (`LiveExchangeSpotOrderReconciliationQuery` + testnet observe), `SpotOrderSubmitAck` | `spot_order_reconciliation_query.rs`, `binance_spot_testnet_reconcile.rs`, `state.rs` | Prod REST; threat model/Critic |
| `modules/portfolio` | `paper_snapshot_with_fills` + posições; HTTP `GET /portfolio/paper-snapshot` via `ApiState::paper_wallet_snapshot` | `controllers.rs`, `http_bridge/portfolio.rs`, `routes/portfolio.rs`, `state.rs` | Preço de mercado dinâmico (não só env fixo) |
| `modules/agents` | Registry + PG; promote capability; bootstrap owner PG (`0010`) + `verify_register_owner_id` | `pg_owner_bootstrap.rs`, `register_owner.rs`, [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md) | IdP / owner humano verificável |
| `presentation/http` | OpenAPI **36** paths; boot `build_api_state_for_http_serve` + owner bootstrap; `GET /meta` (`product_owner_bootstrap_active`); reconciliação; `agents_register_rejects_owner_mismatch_when_product_owner_verified` | `state.rs`, `meta.rs`, `http_integration_tests.rs` | IdP; política prod REST |

Execução live e produção permanecem bloqueadas até gates de segurança.

## Persistência Gate 1 (scaffold)

- Migração SQL `0002_agents_bots_scaffold.sql` (agents + `bot_catalog_entries`); `Database::migrate()` no boot HTTP quando `DATABASE_URL` conecta.
- Teste `postgres_scaffold_tables_exist_after_migrate` em `core/persistence/mod.rs` (skip sem `DATABASE_URL`; script PG **21/21**; incl. `graph_projection_outbox` migração `0009`).
- Adapter Rust e SDD completo: [Gate 1 draft](../sdd/bots-catalog-persistence-gate1-sdd.md).
- `core/database` expõe Neo4j opcional via `neo4rs` (`readyz` probe quando `BOT_AGENTS_ENABLED`).


## Verificação local

Gate canônico (recomendado):

```text
./scripts/verify-backend-gates.sh
```

Equivale a: `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot -- --test-threads=1`, depois `cargo test --locked --test <…>` (5 suítes em `tests/`; evita reexecutar bin `bot` em paralelo). PG opcional: `./scripts/verify-backend-full.sh` (ou `./scripts/run-pg-integration-tests.sh`) com `DATABASE_URL` → `trading_bot` (Timescale + pgvector).

Evidência (2026-09-27; `verify-backend-gates.sh` → **464** passed, **0** ignored; `verify-backend-full.sh` → `OK: backend full verification passed` com `DATABASE_URL`): **464** casos no bin `bot` (linha `OK:` de `verify-backend-gates.sh`). PG×**21** no script (`run-pg-integration-tests.sh`, manifesto validado por `assert-pg-integration-manifest.sh`; snapshot JSON validado por `assert-completeness-evidence.sh` no gate), incl. agents/bots/orders, `loads_credentials_from_postgres` (0007), `pg_product_owner_bootstrap_*` (0010), `pg_graph_projection_outbox_*` (0009 F2.1); Neo4j/testnet via `pg_integration` (skip sem stack). HTTP mutante/bearer: `cargo test --locked --bin bot http_integration -- --test-threads=1` → **48** passed. Neo4j write-only F1–F3.1 + outbox F2.1/F2.1.2 (worker + health): [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md) §5, [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md).

## Documentação relacionada

- [CLI e variáveis HTTP](../reference/cli-and-config.md) (`BOT_HTTP_*`, `BOT_ORDERS_EXECUTION`, `BOT_RUNTIME_ENABLED`, `client_order_id`; [camadas system/bot/env](../reference/cli-and-config.md#configuração-em-camadas))
- [SDD configuração centralizada](../sdd/centralized-config-sdd.md)
- [module-catalog.md](../architecture/module-catalog.md)
- [module-implementation-status.md](../architecture/module-implementation-status.md) — MVC mínimo vs goal de completude (dois vereditos distintos)
- [unimplemented-modules-analysis.md](./unimplemented-modules-analysis.md)
- SDDs: [bots](../sdd/bots-module-sdd.md), [orders](../sdd/orders-module-sdd.md), [agents](../sdd/agents-module-sdd.md), [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)



## Matriz de requisitos (objetivo)

| Requisito | Evidência | Status |
|-----------|-----------|--------|
| Completude bots | Registry + catálogo HTTP, runtime promote, supervisor testnet→orders (`client_order_id` + ledger partilhado), backtest `evaluate_for_kind` | **Parcial** (PG mirror monitor; auth owner) |
| Completude orders | Paper/recording/testnet, idempotência PG, reconciliação+poll (recording/testnet observe), redação `BINANCE_TESTNET_*` em erros mapeados, `SpotOrderSubmitAck` | **Parcial** (prod REST; threat model/Critic) |
| Completude agents | Registry + PG; promote capability; bootstrap `0010` + `VerifiedProductOwner` ([owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)) | **Parcial** (IdP; Critic G1) |
| Integração HTTP + camadas | OpenAPI ≥**36** paths; boot `serve`; admin bearer; owner bootstrap + provider credentials admin CRUD; reconciliação; **48** `http_integration`; PG `pg_http_boot_*` + owner `0010`; [layer-mapping](../architecture/layer-mapping.md) | **Parcial** (IdP; política prod REST) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes verdes | **464** + clippy/fmt/import; PG **21/21** em CI (`postgres-integration`) e opcional local (`verify-backend-full.sh` + `DATABASE_URL`) | **Feito** |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots/orders/agents G2 parcial; owner bootstrap PG; provider credentials HTTP+PG; outbox Neo4j worker; HTTP **48** testes | **Parcial** (IdP; Critic; prod REST) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes verdes | `./scripts/verify-backend-gates.sh` → **464** ok (bin `bot`) + 5 suítes `tests/`; clippy/fmt/import | Feito nesta revisão |
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Roadmap de gates (pós-G1)

| Gate | Módulo | SDD | Implementado |
|------|--------|-----|--------------|
| G1 PG scaffold | agents + bots catálogo | [bots-catalog-persistence-gate1-sdd.md](../sdd/bots-catalog-persistence-gate1-sdd.md) | **Parcial** (adapters + `run-pg-integration-tests.sh` + CI `postgres-integration`; default `cargo test` skip PG sem `DATABASE_URL` via `pg_integration`) |
| G1 HTTP admin seam | presentation/http | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md) | **Sim** (não é auth owner produto) |
| G2 orders live | orders + idempotência + reconciliação | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | **Parcial** (paper/recording/testnet; reconciliação PG+HTTP; poller testnet/job periódico; threat model/Critic pendentes) |
| G2 bots runtime | bots + monitor + agents `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | **Parcial** (`MonitorEvaluatorKind` SMA/EMA no supervisor + `run_sma_crossover`; catálogo `monitor_evaluator`) |
| Provider credentials PG | core/providers + http_bridge | [provider-credentials-db-sdd.md](../sdd/provider-credentials-db-sdd.md) | **Sim** (CRUD admin; secret em texto — criptografia follow-up) |
| Auth owner produto | agents | [agents-owner-bootstrap-g1-sdd.md](../sdd/agents-owner-bootstrap-g1-sdd.md), [agents-module-sdd.md](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist) | **Parcial** (PG+ACK+bind registro; não IdP) |

## Fechamento do goal (pendente)

Implementar itens **Não** nos checklists [orders G2](../sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist) e [agents G1](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist); revisão Critic AGENTS.md. Baseline: `verify-backend-gates.sh` → **464**/**0** ignored; `http_integration` → **48**; PG script → **21/21**; OpenAPI **36** paths.

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
| HTTP admin seam | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md), `admin_auth.rs`, `http_integration_tests.rs`, matriz em [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token) | `./scripts/verify-backend-gates.sh` (**464** passed); `cargo test --locked --bin bot http_integration -- --test-threads=1` (**48** passed) |
| Orders G2 | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) (checklist + threat model), `modules/orders/`, `order_execution.rs`, `binance_spot_testnet_submit.rs` (`redact_known_testnet_credentials`) | Confirmar `authorize_rest_use` / prod REST bloqueado; retenção ops documentada; teste `map_bot_error_redacts_*`; sem credenciais em CI |
| Bots runtime G2 | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md), `evaluation_binding.rs`, `runtime_port.rs` | Promote capability + `evaluate_for_kind`; [matriz runtime vs serve](../reference/test-matrix.md#bot-runtime-no-serve-vs-testes-http-g2-parcial) (linha checklist **Parcial**) |
| Product owner bootstrap G1 | [agents-owner-bootstrap-g1-sdd.md](../sdd/agents-owner-bootstrap-g1-sdd.md), `pg_owner_bootstrap.rs`, `register_owner.rs`, `meta.rs` | PG `pg_product_owner_bootstrap_*`; HTTP `agents_register_rejects_owner_mismatch_when_product_owner_verified`, `bots_runtime_promote_rejects_promoted_by_mismatch_when_product_owner_verified`, `meta_reports_product_owner_bootstrap_active_when_verified` |
| Agents G1 + boot HTTP | [agents-module-sdd.md](../sdd/agents-module-sdd.md), `bot_promotion.rs`, `http_bridge/agents.rs`, `state.rs` (`build_api_state_for_http_serve`, `pg_http_boot_sequence_mirrors_serve_wiring` com `GET /agents`, `GET /bots/catalog`, `GET /config/active`, `GET /orders/reconciliation/*`) | Itens **Não** do checklist permanecem bloqueadores de produto; PG **21/21** via `verify-backend-full.sh` ou CI `postgres-integration` |
| Neo4j F1–F3.1 + outbox F2.1.2/F2.1.3 | [graph-projection-outbox-sdd.md](../sdd/graph-projection-outbox-sdd.md), `graph_projection_outbox.rs`, `graph_projection_cli.rs`, projeções agents/bots/orders | `pg_graph_projection_outbox_*` no script PG; `graph_projection_cli_*` no bin `bot`; CLI `graph-projection drain` ([cli-and-config](../reference/cli-and-config.md)); testes `neo4j_*` fora do script (skip sem stack) |
| Provider credentials admin | [provider-credentials-db-sdd.md](../sdd/provider-credentials-db-sdd.md), `http_bridge/provider_credentials.rs`, `routes/provider_credentials_admin.rs` | HTTP `provider_credentials_admin_*` (incl. DELETE `provider_credentials_admin_delete_removes_row`); PG `loads_credentials_from_postgres` no script **21/21** |
| Graph query F3 (read-only) | [graph-query-port-f3-sdd.md](../sdd/graph-query-port-f3-sdd.md), `graph_query.rs`, `graph_cli.rs` | `graph_query_port_*`, `graph_cli_*`; CLI `graph query agents`, `supervision-chain`, `bots-for-agent` ([cli-and-config](../reference/cli-and-config.md)); `neo4j_list_agents_after_local_graph`; `neo4j_agent_supervision_chain_after_projection` (skip sem stack) |
| Portfolio paper (HTTP) | `http_bridge/portfolio.rs`, `routes/portfolio.rs`, `state.rs` (`paper_wallet_snapshot`) | `paper_wallet_snapshot_reflects_in_process_ledger`; `portfolio_paper_snapshot_http_reflects_paper_submit` em `http_integration_tests.rs` |

Comandos canônicos: `./scripts/verify-backend-gates.sh` (**464** passed / **0** ignored na linha `OK:`); com `DATABASE_URL` → `trading_bot`: `./scripts/verify-backend-full.sh` (gates + PG **21/21**, mensagem `OK: backend full verification passed`); só PG: `./scripts/run-pg-integration-tests.sh`. Testnet: `cargo test --locked integration_submits_minimal_market_buy_on_testnet` (skip sem `BINANCE_TESTNET_*`; fora de CI).

Entrega esperada do Critic: veredito **APROVADO** / **APROVADO COM FOLLOW-UP** / **REPROVADO** por SDD, com achados ligados a teste ou linha de código; autor do pacote não aprova o próprio artefato (`AGENTS.md`).

<a id="entrega-pacote-completude-módulos--g4-builder"></a>

## ENTREGA — pacote completude módulos (G4 Builder)

- **Builder:** fatia técnica bots/orders/agents/HTTP + Neo4j F1–F3.1 + outbox F2.1.2 (worker + `/healthz` backlog) + F2.1.3 CLI drain + F3 read-only `GraphQueryPort` (agents list, supervision chain, bots_for_agent); docs SDD/catálogo/roadmap/README alinhados.
- **Testes/evidências:** `./scripts/verify-backend-gates.sh` + `assert-completeness-evidence.sh` → **464** passed, **0** ignored; `./scripts/verify-backend-full.sh` + `DATABASE_URL` → PG **21/21**; `http_integration` → **48** passed; `pg_store_error` + HTTP **503** `order_store_unavailable`; F2.1.2 worker + `/healthz` outbox; F2.1.3 `graph_projection_cli_*` + `bot graph-projection drain`; F3 `graph_query_port_*` / `graph_cli_*` + [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md) ([cli-and-config](../reference/cli-and-config.md)).
- **Achados/revisão:** **PENDENTE** — Critic independente (`AGENTS.md`); IdP/owner humano fora do escopo da fatia bootstrap; seam `BOT_HTTP_*` + `VerifiedProductOwner` cobertos por testes.
- **Veredito:** **PENDENTE** até sessão Critic + decisões de produto (auth owner, prod REST).
