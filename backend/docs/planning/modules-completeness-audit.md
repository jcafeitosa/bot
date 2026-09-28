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

> Revisão: 2026-09-27. Snapshot machine-readable: [modules-completeness-evidence.json](./modules-completeness-evidence.json). Fonte: `backend/src`, SDDs em `docs/sdd/`. Evidência registrada: etapa de testes do gate **520**/**0** ignored (bin `bot`, `modules-completeness-evidence.json`); manifesto PG **31** (contagem estática; execução PG e CI `backend-ci.yml` sem run verde (0/511 runs `success` até 27/09)); `cargo test --bin bot http_integration -- --test-threads=1` → **62** passed.


> **Manifest PG (verificado em `origin/main` `d42b71a5`):** `persist_dataset_rejects_conflicting_manifest_for_same_id` continua listado no script, mas **não tem fn de teste**: a implementação com `DatasetManifestConflict` foi revertida em `afe1f411`. `persist_dataset` usa `ON CONFLICT (dataset_id) DO NOTHING` (`core/persistence/mod.rs:52-59`). O script roda `cargo test --bin bot <nome>` sem `--exact`, então a entrada passa com 0 testes.
> **Bloqueio de fechamento:** esta linha preserva o baseline histórico do goal em `afe1f411` = código de `d42b71a5` (`verify-backend-gates.sh` **519**/**0** ignored; manifesto PG **29**, 28 com função; seleção sem `--exact`). Evidência atual da etapa de testes: 520/0; manifesto PG: 31 entradas estáticas, sem execução PG registrada. A fatia técnica/doc, `pg_integration`; CI `backend-ci.yml` sem run verde (0/511 runs `success` até 27/09)); **IdP / owner humano verificável** e **Critic** `AGENTS.md` permanecem bloqueadores (bootstrap PG `0010` é fatia parcial — [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)) — ver § [Fechamento do goal (pendente)](#fechamento-do-goal-pendente).

## Resumo executivo

| Módulo / superfície | Completude | Evidência principal | Próximo gate |
|---|---|---|---|
| `modules/bots` | `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross`/`ema_cross`), supervisor + backtest via `evaluate_for_kind`, catálogo HTTP `monitor_evaluator` + períodos v1/v2 (`bots_catalog_http_lists_monitor_registry_v2_periods`); promote SMA/EMA via registry | `monitor_strategy.rs`, `http_bridge/bots.rs`, `evaluation_binding.rs`, `server.rs` | Auth owner; orders live |
| `modules/orders` | Paper/recording/testnet, idempotência+PG, reconciliação+poll (`LiveExchangeSpotOrderReconciliationQuery` + testnet observe), `SpotOrderSubmitAck` | `spot_order_reconciliation_query.rs`, `binance_spot_testnet_reconcile.rs`, `state.rs` | Prod REST; threat model/Critic |
| `modules/portfolio` | `paper_snapshot_with_fills` + posições; HTTP `GET /portfolio/paper-snapshot` via `ApiState::paper_wallet_snapshot` | `controllers.rs`, `http_bridge/portfolio.rs`, `routes/portfolio.rs`, `state.rs` | Preço de mercado dinâmico (não só env fixo) |
| `modules/agents` | Registry + PG; promote capability; bootstrap owner PG (`0010`) + `verify_register_owner_id` | `pg_owner_bootstrap.rs`, `register_owner.rs`, [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md) | IdP / owner humano verificável |
| `presentation/http` | OpenAPI **42** paths; boot `build_api_state_for_http_serve` + owner bootstrap; `GET /meta` (`product_owner_bootstrap_active`); reconciliação; `agents_register_rejects_owner_mismatch_when_product_owner_verified` | `state.rs`, `meta.rs`, `http_integration_tests.rs` | IdP; política prod REST |

Execução live e produção permanecem bloqueadas até gates de segurança.

## Persistência Gate 1 (scaffold)

- Migração SQL `0002_agents_bots_scaffold.sql` (agents + `bot_catalog_entries`); `Database::migrate()` no boot HTTP quando `DATABASE_URL` conecta.
- Teste `postgres_scaffold_tables_exist_after_migrate` em `core/persistence/mod.rs` (skip sem `DATABASE_URL`; manifesto PG **31** (contagem estática; execução PG não registrada); incl. `graph_projection_outbox` migração `0009`).
- Adapter Rust e SDD completo: [Gate 1 draft](../sdd/bots-catalog-persistence-gate1-sdd.md).
- `core/database` expõe Neo4j opcional via `neo4rs` (`readyz` probe quando `BOT_AGENTS_ENABLED`).


## Verificação local

Gate canônico (recomendado):

```text
./scripts/verify-backend-gates.sh
```

Equivale a: `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot -- --test-threads=1`, depois `cargo test --locked --test <…>` (5 suítes em `tests/`; evita reexecutar bin `bot` em paralelo). PG opcional: `./scripts/verify-backend-full.sh` (ou `./scripts/run-pg-integration-tests.sh`) com `DATABASE_URL` → `trading_bot` (Timescale + pgvector).

Evidência observada em 2026-09-27: etapa `cargo test --locked --bin bot -- --test-threads=1` → **520** passed, **0** ignored, conforme `modules-completeness-evidence.json`. `verify-backend-gates.sh` parou na asserção de completude por divergência do snapshot; as cinco suítes workspace não foram executadas. Manifesto PG: **31** entradas estáticas; execução PG não registrada. Histórico em `afe1f411`/`d42b71a5`: 519 testes no bin e 29 entradas no manifesto (28 com função). (`run-pg-integration-tests.sh`, manifesto validado por `assert-pg-integration-manifest.sh`; snapshot JSON validado por `assert-completeness-evidence.sh` no gate), incl. agents/bots/orders, `loads_credentials_from_postgres` (0007), `pg_product_owner_bootstrap_*` (0010), `pg_graph_projection_outbox_*` (0009 F2.1), `pg_order_idempotency_and_graph_projection_same_transaction` (F2.1.3+ TX orders), `pg_agent_identity_and_graph_projection_same_transaction` (F2.1.3+ TX agents), `pg_monitor_supervisor_snapshot_round_trip` (0011, [C17 fatia 1](../sdd/monitor-persistence-c17-sdd.md)); unit C17 **fatia 2** (`PersistenceStatus::Gap`, REST `persistence_status` — `monitor_snapshot_maps_*`); Neo4j/testnet via `pg_integration` (skip sem stack). HTTP mutante/bearer: `cargo test --locked --bin bot http_integration -- --test-threads=1` → **62** passed. Neo4j write-only F1–F3.1 + outbox F2.1/F2.1.2 (worker + health): [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md) §5, [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md).

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
| Completude bots | Registry + catálogo HTTP, runtime promote, supervisor testnet→orders (`client_order_id` + ledger partilhado; ramo inalcançável no binário — `--mode testnet` rejeitado em `core/config/mod.rs:390-393`), backtest `evaluate_for_kind` | **Parcial** (PG mirror monitor; auth owner) |
| Completude orders | Paper/recording/testnet, idempotência PG, reconciliação+poll (recording/testnet observe), redação `BINANCE_TESTNET_*` em erros mapeados, `SpotOrderSubmitAck` | **Parcial** (prod REST; threat model/Critic) |
| Completude agents | Registry + PG; promote capability; bootstrap `0010` + `VerifiedProductOwner` ([owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md)) | **Parcial** (IdP; Critic G1) |
| Integração HTTP + camadas | OpenAPI **42** paths; boot `serve`; admin bearer; owner bootstrap + provider credentials admin CRUD; reconciliação; **62** `http_integration`; PG `pg_http_boot_*` + owner `0010`; [layer-mapping](../architecture/layer-mapping.md) | **Parcial** (IdP; política prod REST) |
| Gaps documentados | SDDs + esta auditoria | **Feito** |
| Build/testes observados | **520** testes no bin + clippy/fmt/import; manifesto PG **31** entradas estáticas (execução PG não registrada); CI `backend-ci.yml` nunca verde (0/511 runs `success`; #510 em 27/09 15:56 COT falhou no clippy do job `rust`, `postgres-integration` skipped) | **Parcial** (CI não comprovada) |
| Revisão Critic | AGENTS.md | **Bloqueado** |

## Checklist do objetivo

| Item do goal | Evidência | Status |
|---|---|---|
| Analisar completude (bots, orders, agents, HTTP) | Este documento + `unimplemented-modules-analysis.md` | Feito |
| Identificar gaps | Tabelas acima + SDDs Gate 1 | Feito |
| Expandir/melhorar implementação | Bots/orders/agents G2 parcial; owner bootstrap PG; provider credentials HTTP+PG; outbox Neo4j worker; HTTP **62** testes | **Parcial** (IdP; Critic; prod REST) |
| Atualizar SDD, catálogo, roadmap, README | `module-catalog`, `current-state-and-roadmap`, `cli-and-config`, SDDs | Feito |
| Build/testes observados | etapa do bin em `./scripts/verify-backend-gates.sh` → **520** ok; clippy/fmt/import passaram; gate parou na asserção de evidência, sem execução das 5 suítes workspace | Parcial
| Revisão Critic independente (AGENTS.md) | — | **Bloqueado** (instância separada) |

## Roadmap de gates (pós-G1)

| Gate | Módulo | SDD | Implementado |
|------|--------|-----|--------------|
| G1 PG scaffold | agents + bots catálogo | [bots-catalog-persistence-gate1-sdd.md](../sdd/bots-catalog-persistence-gate1-sdd.md) | **Parcial** (adapters + `run-pg-integration-tests.sh`; CI `postgres-integration` ainda sem execução bem-sucedida; default `cargo test` skip PG sem `DATABASE_URL` via `pg_integration`) |
| G1 HTTP admin seam | presentation/http | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md) | **Sim**, mas fica aberto sem token (falha aberta, `admin_auth.rs:91-94`; correção em W0-01). Não é auth owner produto |
| G2 orders live | orders + idempotência + reconciliação | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) | **Parcial** (paper/recording/testnet; reconciliação PG+HTTP; poller testnet/job periódico; threat model/Critic pendentes) |
| G2 bots runtime | bots + monitor + agents `promote_runtime_bot` quando `BOT_HTTP_AGENCY_ID` | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md) | **Parcial** (`MonitorEvaluatorKind` SMA/EMA no supervisor + `run_sma_crossover`; catálogo `monitor_evaluator`) |
| Provider credentials PG | core/providers + http_bridge | [provider-credentials-db-sdd.md](../sdd/provider-credentials-db-sdd.md) | **Sim** (CRUD admin; secret em texto — criptografia follow-up) |
| Auth owner produto | agents | [agents-owner-bootstrap-g1-sdd.md](../sdd/agents-owner-bootstrap-g1-sdd.md), [agents-module-sdd.md](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist) | **Parcial** (PG+ACK+bind registro; não IdP) |

## Fechamento do goal (pendente)

Implementar itens **Não** nos checklists [orders G2](../sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist) e [agents G1](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist); revisão Critic AGENTS.md. Baseline observado: etapa de testes do bin em `verify-backend-gates.sh` → **520**/**0** ignored; gate completo interrompido na asserção de evidência; `http_integration` → **62**; manifesto PG → **31** (contagem estática; incl. V18 fatia 1 `pg_persist_dataset_*` + `persist_dataset_rejects_conflicting_manifest_for_same_id`; C17 fatia 1 `pg_monitor_supervisor_snapshot_round_trip` + fatia 2 presentation/REST); OpenAPI **42** paths.

| W0-09 domínio+outbox TX (falha injetada) | [monitor-persistence-v18-sdd](../sdd/monitor-persistence-v18-sdd.md) | **Pendente:** `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure` e `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure` não existem em `origin/main` `d42b71a5` (fatia agents revertida em `2089a496`); W0-09 volta pelo G1 ([wave0-09-v18-verificacao-sdd](../sdd/wave0-09-v18-verificacao-sdd.md)); **defer:** W0-10 G4 CI |
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
| HTTP admin seam | [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md), `admin_auth.rs`, `http_integration_tests.rs`, matriz em [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token) | `./scripts/verify-backend-gates.sh` (etapa bin: **520** passed); `cargo test --locked --bin bot http_integration -- --test-threads=1` (**62** passed) |
| Orders G2 | [orders-live-execution-gate2-sdd.md](../sdd/orders-live-execution-gate2-sdd.md) (checklist + threat model), `modules/orders/`, `order_execution.rs`, `binance_spot_testnet_submit.rs` (`redact_known_testnet_credentials`) | Confirmar `authorize_rest_use` / prod REST bloqueado; retenção ops documentada; teste `map_bot_error_redacts_*`; sem credenciais em CI |
| Bots runtime G2 | [bots-runtime-live-gate2-sdd.md](../sdd/bots-runtime-live-gate2-sdd.md), `evaluation_binding.rs`, `runtime_port.rs` | Promote capability + `evaluate_for_kind`; [matriz runtime vs serve](../reference/test-matrix.md#bot-runtime-no-serve-vs-testes-http-g2-parcial) (linha checklist **Parcial**) |
| Product owner bootstrap G1 | [agents-owner-bootstrap-g1-sdd.md](../sdd/agents-owner-bootstrap-g1-sdd.md), `pg_owner_bootstrap.rs`, `register_owner.rs`, `meta.rs` | PG `pg_product_owner_bootstrap_*`; HTTP `agents_register_rejects_owner_mismatch_when_product_owner_verified`, `bots_runtime_promote_rejects_promoted_by_mismatch_when_product_owner_verified`, `meta_reports_product_owner_bootstrap_active_when_verified` |
| Agents G1 + boot HTTP | [agents-module-sdd.md](../sdd/agents-module-sdd.md), `bot_promotion.rs`, `http_bridge/agents.rs`, `state.rs` (`build_api_state_for_http_serve`, `pg_http_boot_sequence_mirrors_serve_wiring` com `GET /agents`, `GET /bots/catalog`, `GET /config/active`, `GET /orders/reconciliation/*`) | Itens **Não** do checklist permanecem bloqueadores de produto; manifesto PG **31** (`verify-backend-full.sh`; execução local/CI não comprovada) |
| Neo4j F1–F3.1 + outbox F2.1.2/F2.1.3 | [graph-projection-outbox-sdd.md](../sdd/graph-projection-outbox-sdd.md), `graph_projection_outbox.rs`, `graph_projection_cli.rs`, projeções agents/bots/orders | `pg_graph_projection_outbox_*` no script PG; `graph_projection_cli_*` no bin `bot`; CLI `graph-projection drain` ([cli-and-config](../reference/cli-and-config.md)); testes `neo4j_*` fora do script (skip sem stack) |
| Provider credentials admin | [provider-credentials-db-sdd.md](../sdd/provider-credentials-db-sdd.md), `http_bridge/provider_credentials.rs`, `routes/provider_credentials_admin.rs` | HTTP `provider_credentials_admin_*` (incl. DELETE `provider_credentials_admin_delete_removes_row`); PG `loads_credentials_from_postgres` no manifesto PG **31** |
| Graph query F3 (read-only) | [graph-query-port-f3-sdd.md](../sdd/graph-query-port-f3-sdd.md), `graph_query.rs`, `graph_cli.rs` | `graph_query_port_*`, `graph_cli_*`; CLI `graph query` (`agents`, `supervision-chain`, `bots-for-agent`, `code-impact`) — [cli-and-config](../reference/cli-and-config.md); Neo4j skip sem stack: `neo4j_list_agents_after_local_graph`, `neo4j_supervision_chain_query_after_projection`, `neo4j_bots_for_agent_after_catalog_and_promotion_projection`, `neo4j_code_impact_for_module_after_seed`; projeção agents: `neo4j_agent_supervision_chain_after_projection` (`graph_projection.rs`) |
| Portfolio paper (HTTP) | `http_bridge/portfolio.rs`, `routes/portfolio.rs`, `state.rs` (`paper_wallet_snapshot`) | `paper_wallet_snapshot_reflects_in_process_ledger`; `portfolio_paper_snapshot_http_reflects_paper_submit` em `http_integration_tests.rs` |

Comandos canônicos: `./scripts/verify-backend-gates.sh` (etapa do bin observada: **520** passed / **0** ignored; gate completa interrompida na asserção de evidência); com `DATABASE_URL` → `trading_bot`: `./scripts/verify-backend-full.sh` (inclui manifesto PG com **31** entradas estáticas; execução não registrada); só PG: `./scripts/run-pg-integration-tests.sh`. Testnet: `cargo test --locked integration_submits_minimal_market_buy_on_testnet` (skip sem `BINANCE_TESTNET_*`; fora de CI).

Entrega esperada do Critic: veredito **APROVADO** / **APROVADO COM FOLLOW-UP** / **REPROVADO** por SDD, com achados ligados a teste ou linha de código; autor do pacote não aprova o próprio artefato (`AGENTS.md`).

<a id="entrega-pacote-completude-módulos--g4-builder"></a>

## ENTREGA — pacote completude módulos (G4 Builder)

- **Builder:** fatia técnica bots/orders/agents/HTTP + Neo4j F1–F3.1 + outbox F2.1.2 (worker + `/healthz` backlog) + F2.1.3 CLI drain + F2.1.3+ enqueue TX orders (`enqueue_graph_projection_outbox_tx` + `persist_idempotency_and_enqueue_graph_projection`) + F3 read-only `GraphQueryPort` (agents, supervision chain, bots_for_agent, code_impact_for_module); docs SDD/catálogo/roadmap/README alinhados.
- **Registro histórico desta revisão:** `./scripts/verify-backend-gates.sh` + `assert-completeness-evidence.sh` → **519** passed, **0** ignored; manifesto PG **29** (execução não registrada). Evidência atual: etapa do bin 520/0; manifesto PG 31 entradas estáticas, sem execução registrada. `./scripts/verify-backend-full.sh` + `DATABASE_URL` → `http_integration` → **62** passed; `pg_store_error` + HTTP **503** `order_store_unavailable`; F2.1.2 worker + `/healthz` outbox; F2.1.3 `graph_projection_cli_*` + `bot graph-projection drain`; F3 `graph_query_port_*` / `graph_cli_*` + [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md) ([cli-and-config](../reference/cli-and-config.md)).
- **Achados/revisão:** **PENDENTE** — Critic independente (`AGENTS.md`); IdP/owner humano fora do escopo da fatia bootstrap; seam `BOT_HTTP_*` + `VerifiedProductOwner` cobertos por testes.
- **Veredito:** **PENDENTE** até sessão Critic + decisões de produto (auth owner, prod REST).
