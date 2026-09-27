---
title: Matriz de testes do backend
description: Mapeamento de testes por módulo, comportamento validado e lacunas de integração
tags:
  - reference
  - backend
  - tests
  - verification
---

# Matriz de testes do backend

> Revisão: 2026-09-27 (PG agents/bots, HTTP admin bearer, hydrate). A matriz descreve os testes presentes no código e o limite da evidência disponível.

## Testes de integração

| Arquivo | Comportamento coberto | Dependências | Estado |
|---|---|---|---|
| `tests/config_cli.rs` | Arquivo padrão, caminho explícito, ausência e arquivo ilegível; precedência de CLI. | Sistema de arquivos local. | Passa. |
| `tests/backtest_fixture.rs` | Presets/timeframes produzem fixture determinística e ao menos um trade fechado. | Nenhuma. | Passa. |
| `tests/redirect_origin_test.rs` | Origem, porta efetiva, downgrade, userinfo, histórico vazio e limite de redirects. | Nenhuma. | Passa. |
| `tests/redirect_policy_test.rs` | Redirect same-origin aceito e cross-origin rejeitado antes do contato com o segundo listener. | Loopback local. | Passa fora do sandbox; pode falhar em sandbox sem permissão de listener. |

## Testes unitários por módulo

| Módulo | Comportamentos observáveis |
|---|---|
| `config` | Defaults dev/observe, produção fail-closed, presets e rejeição de HFT em feed REST 1m. |
| `market` | Rejeita barra parcial, gaps, duplicatas e dados incompatíveis; preserva OHLCV no round-trip. |
| `market_feed` | Uma avaliação por timestamp, upsert WS, catch-up REST, contiguidade, watermark monotônico e preenchimento tardio. |
| `strategy` | Períodos por operação; `evaluate` / `evaluate_ema` / `evaluate_for_kind` (SMA e EMA). |
| `risk` | Limite conservador/agressivo, tamanho, stop/take profit e incompatibilidade de modo. |
| `agents` | Registry, lifecycle, `assert_runtime_promotion_authorized` (capability + not-active), PG snapshot (ciclo lifecycle); HTTP `agents_register_promote_runtime_bot_visible_via_http_get`, `agents_audit_lists_lifecycle_events_after_register_and_pause` (`http_integration_tests.rs`). |
| `http_bridge/agents` | `apply_agent_identity_snapshot` no-op quando registry já populado; `register_agent_maps_promote_runtime_bot_capability`; `agent_lifecycle_snapshot_for_persist_reflects_latest_audit_kind` (em `mod.rs`); PG ignorados `pg_agent_lifecycle_write_through_round_trip`, `pg_cold_start_apply_snapshot_after_write_through`. |
| `http_bridge/config` | `map_config` expõe `monitor_registry` com `evaluator`; HTTP `GET /config/active` + `GET /config/snapshot` via `ApiState` (`config_snapshot_from_path_loads_bundled_default_toml`, smoke em `documented_get_routes_respond`). |
| `http_bridge/monitor` | `attach_bot_runtime_status` enriquece snapshot HTTP (incl. `sma-cross@2`). |
| `http_bridge/orders` | `submit_order_http_records_execution_with_recording_executor`; dedupe `client_order_id`; HTTP `orders_submit_*` (paper, live_exchange wired/recording, reserved, admin bearer) em `http_integration_tests.rs`; `orders_submit_fail_closed_returns_503` em `server.rs`. |
| `modules/orders` reconciliation | `reconciliation_pending_to_reconciled`, `reconciliation_mark_divergent_from_pending`, `reconciliation_seed_entry_restores_pending_count`, `reconciliation_poll_confirms_pending_when_recording_binding_exists`, `reconciliation_seed_hydrated_row_preserves_symbol_for_poll`; `recording_submit_returns_deterministic_exchange_order_id`. |
| `presentation/http/state` | `build_api_state_for_http_serve_without_database_wires_executor` (`ApiState::build_api_state_for_http_serve`); `submit_order_recording_live_exchange_auto_reconciles_client_order_id`; ledger partilhado: `lock_shared_live_order_reconciliation_ledger_for_test()`; com `EnvTestGuard`, **env antes** do ledger. |
| `http_bridge/portfolio` | `paper_wallet_snapshot_reflects_in_process_ledger`; HTTP E2E paper submit + snapshot em `http_integration_tests.rs` (`portfolio_paper_snapshot_http_reflects_paper_submit`); `ApiState::paper_wallet_snapshot` (`paper_wallet_snapshot_via_api_state_reflects_paper_submit` em `state.rs`). |
| `http_bridge/bots` | Catálogo com `monitor_evaluator` e períodos (`catalog_for_config_lists_monitor_registry_v2_periods`, `catalog_for_config_includes_monitor_strategy_periods`); `persist_catalog_bridge_materializes_monitor_registry_v2`; HTTP `bots_catalog_http_lists_monitor_registry_v2_periods` em `http_integration_tests.rs`; `assert_bot_promotion_allowed_accepts_monitor_registry_v2`. |
| `orders` | `PaperLedgerExecutor`; `ExchangeSpotExecutor` + `submit_spot_order` (`testnet_backend_is_not_wired_yet`); `RecordingExecutor`; `ReservedLiveExchangeExecutor`; idempotência PG (ignorado). |
| `bots` | `build_catalog_from_monitor_registry_includes_each_registered_strategy` (`catalog.rs`); `MonitorEvaluatorKind`; `monitor_evaluation_for_promoted_identity`; `strategy_evaluation_binding_*`; runtime promote v2 (`persist_catalog_then_promote_monitor_registry_v2_bot` em `state.rs`); PG persist v2 em `state.rs`. |
| `portfolio` | Snapshot paper, ativos, posição e erro de inconsistência. |
| `backtest` | `run_sma_crossover` respeita `StrategyDefinition::evaluator`; fees, slippage, stop/take-profit (`ema_crossover_backtest_uses_strategy_evaluator`). |
| `domain` | Ranking, métricas, janela de avaliação e tipos de identidade. |
| `exchanges/mod` | Chave de conta estável e rótulo vazio rejeitado. |
| `exchanges/account_file` | Parsing, seleção de ambiente e validação de origem. |
| `exchanges/binance` | Origem testnet, dados fechados válidos, duplicata conflitante, alinhamento e finitude. |
| `exchanges/bootstrap` | Registro default e seleção de conta Spot. |
| `exchanges/capabilities` | Catálogo coerente de capacidades. |
| `exchanges/live` | Endpoint seguro, rejeição de origem não confiável, filtro de candle aberto e payload inválido. |
| `exchanges/registry` | Registro, consulta e duplicidade de contas. |
| `exchanges/resources` | Recursos autorizados e plano por ambiente. |
| `exchanges/rest` | Backfill público Spot dev; `OrderSubmit` só com seam `BOT_ORDERS_EXCHANGE_SUBMIT=recording`; demais privados fail-closed. |
| `exchanges/router` | Mapeamento de necessidade para transporte e streams default. |
| `exchanges/stream` | Assinatura e validação de evento. |
| `exchanges/ws` | Configuração e plano `1m` validado. |
| `app` | Pause sem bloquear, resume com drain, stale REST/WS, gerações, falha de resume, cancelamento, shutdown e fila de persistência. |
| `ui` | Comando de espaço de acordo com o estado confirmado. |
| `presentation/http` | OpenAPI **36** paths; `router_after_build_api_state_serves_catalog_and_meta`; `GET /meta` + `meta_and_*` (`server.rs`); admin bearer + orders executors (`http_integration_tests.rs` — matriz [rotas mutantes](#rotas-mutantes-com-bot_http_admin_token)); smoke POST stateless (`documented_post_routes_accept_valid_json`) + GET (`documented_get_routes_respond`) e catálogo (`server.rs`); portfolio HTTP (`portfolio_paper_snapshot_http_reflects_paper_submit` + `routes/portfolio.rs`); PG ignorados em `state.rs`. | `server.rs`, `http_integration_tests.rs`, `state.rs`, `routes/meta.rs`, `admin_auth.rs`, `order_execution.rs`. |
| `persistence` | Round-trip de migração, gravação e contagem, condicionado a PostgreSQL. |

## Verificação executada

```text
./scripts/verify-backend-gates.sh
# equivale a:
cargo fmt --check
cargo clippy --locked --bin bot -- -D warnings
./scripts/check-import-direction.sh
cargo test --locked --bin bot -- --test-threads=1
cargo test --locked --test backtest_fixture
cargo test --locked --test config_cli
cargo test --locked --test monitor_startup_cli
cargo test --locked --test redirect_origin_test
cargo test --locked --test redirect_policy_test
```

O gate canônico executa `cargo test --locked --bin bot -- --test-threads=1` (locks de env + ledger compartilhado não podem atravessar `.await` com paralelismo default), depois as cinco suítes acima — **não** `cargo test --locked` completo (reexecutaria o bin `bot` em paralelo e pode flake). A linha final de `./scripts/verify-backend-gates.sh` inclui o resumo `test result:` do bin `bot` para alinhar docs com evidência.

**CI** (`.github/workflows/backend-ci.yml`): job `rust` → `./scripts/verify-backend-gates.sh`; job `postgres-integration` (após `rust`, service PostgreSQL `trading_bot`) → `./scripts/run-pg-integration-tests.sh` (**14/14** testes `#[ignore]` de domínio).

Evidência típica (atualizar após mudanças de teste): **385** aprovados + **16** ignorados = **402** casos no bin `bot` (só aprovados na linha `OK:` do gate); integração workspace (redirect, config CLI, backtest fixture, etc.) além do bin; PG **14/14** via `./scripts/run-pg-integration-tests.sh` quando `DATABASE_URL` → `trading_bot` (CI `postgres-integration` ou compose local `:55433` — [postgres-and-graph-dev](../operations/postgres-and-graph-dev.md)).

Bin `bot`: **385** aprovados, **16** ignorados (incl. `integration_submits_minimal_market_buy_on_testnet` manual testnet). PG: `./scripts/run-pg-integration-tests.sh` com `DATABASE_URL` → `trading_bot` (Timescale + pgvector). Neo4j: teste `ping_and_node_count_against_local_graph` separado (`BOT_AGENTS_ENABLED` + compose `graph`).

### Testes `#[ignore]` no bin `bot` (16)

| Teste | Arquivo | Como executar |
|-------|---------|---------------|
| `postgres_scaffold_tables_exist_after_migrate` | `core/persistence/mod.rs` | `run-pg-integration-tests.sh` ou `cargo test -- --ignored postgres_scaffold` |
| `persist_dataset_round_trip` | `modules/market/models.rs` | idem |
| `pg_identity_snapshot_round_trip` | `modules/agents/adapters/pg_registry.rs` | idem |
| `pg_agent_lifecycle_write_through_round_trip` | `modules/http_bridge/agents.rs` | idem |
| `pg_cold_start_apply_snapshot_after_write_through` | `modules/http_bridge/agents.rs` | boot `serve`: hydrate registry vazio após PG |
| `pg_register_agent_and_persist_cold_start_via_snapshot` | `presentation/http/state.rs` | `POST /agents` → `persist_agent_after_mutation` + cold-start snapshot |
| `pg_catalog_store_round_trip` | `modules/bots/adapters/pg_catalog.rs` | adapter store |
| `pg_bot_catalog_snapshot_round_trip_via_api_state` | `presentation/http/state.rs` | `persist_bot_catalog` + `GET /bots/catalog/snapshot` via `catalog_from_store` |
| `pg_order_idempotency_round_trip` | `modules/orders/adapters/pg_idempotency.rs` | adapter store |
| `pg_submit_order_idempotency_reads_pg_when_memory_empty` | `presentation/http/state.rs` | `submit_order_http` dedupe via PG sem cache em memória |
| `pg_order_reconciliation_round_trip` | `modules/orders/adapters/pg_reconciliation.rs` | idem |
| `pg_hydrate_order_reconciliation_from_pg_after_durable_write` | `presentation/http/state.rs` | boot `serve`: `hydrate_order_reconciliation_from_pg` após linhas só em PG |
| `pg_order_reconciliation_lookup_reads_pg_when_memory_empty` | `presentation/http/state.rs` | `GET /orders/reconciliation/{id}` fallback PG sem hydrate |
| `pg_http_boot_sequence_mirrors_serve_wiring` | `presentation/http/state.rs` | cold-start agents + `for_http_server` + hydrate orders/catálogo (espelha `server::run`) |
| `ping_and_node_count_against_local_graph` | `core/database/neo4j.rs` | compose `graph` + `BOT_AGENTS_ENABLED=true`; fora do script PG |
| `integration_submits_minimal_market_buy_on_testnet` | `exchanges/adapters/binance_spot_testnet_submit.rs` | credenciais testnet + rede; `cargo test -- --ignored integration_submits` |

### Bot runtime no `serve` vs testes HTTP (G2 parcial)

Checklist [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist): linha **Parcial** (“runtime injetado em testes HTTP = `serve`”).

| Modo | Onde | Evidência |
|------|------|-----------|
| **Produção / boot** | `server::run`, `ApiState::build_api_state_for_http_serve`, `HttpApiSeams::from_env` | `shared_bot_runtime()` — mesmo `Arc` process-wide |
| **Paridade explícita (unit)** | `presentation/http/state.rs` | `for_http_server_wires_process_wide_bot_runtime_like_serve`; `from_env_shares_process_wide_bot_runtime_with_serve` |
| **Paridade boot + PG** | `presentation/http/state.rs` | `pg_http_boot_sequence_mirrors_serve_wiring` (`#[ignore]`; script PG **14/14**) |
| **Router após boot canônico** | `presentation/http/server.rs` | `router_after_build_api_state_serves_catalog_and_meta` usa `build_api_state_for_http_serve` |
| **Isolado por teste** | Maioria dos `bots_runtime_*` / promote em `http_integration_tests.rs` e helpers em `state.rs` | `Arc::new(InMemoryBotRuntime::new())` — evita vazamento de estado entre casos; **não** prova sozinho o wiring do `serve` |

Conclusão documentada: G2 **não** exige que todo teste HTTP use runtime partilhado; exige seams + testes que provam o mesmo `Arc` que o `serve`. Fechamento total continua bloqueado por auth owner e Critic.

## Rotas mutantes com `BOT_HTTP_ADMIN_TOKEN`

Testes abaixo em `presentation/http/http_integration_tests.rs` (salvo rotas OpenAPI/meta ainda em `server.rs`).

| Rota | 401 sem Bearer | 2xx com Bearer (quando aplicável) |
|------|----------------|-----------------------------------|
| `POST /api/v1/agents` | `agents_register_requires_admin_bearer_when_enabled` | mesmo teste (201) |
| `POST /api/v1/agents/{id}/pause` | `agents_pause_requires_admin_bearer_when_enabled` | `agents_pause_succeeds_with_admin_bearer_after_register` |
| `POST /api/v1/agents/{id}/resume` | `agents_resume_requires_admin_bearer_when_enabled` | `agents_resume_succeeds_with_admin_bearer_after_pause` |
| `POST /api/v1/agents/{id}/retire` | `agents_retire_requires_admin_bearer_when_enabled` | `agents_retire_succeeds_with_admin_bearer_after_register` |
| `POST /api/v1/agents/{id}/advisory` | `agents_advisory_requires_admin_bearer_when_enabled` | `agents_advisory_returns_503_with_admin_bearer_when_agent_cannot_consult_jev` (sem Jev) |
| `POST /api/v1/bots/catalog/persist` | `bots_catalog_persist_requires_admin_bearer_when_enabled` | mesmo teste (200) |
| `POST /api/v1/bots/runtime/promote` | `bots_runtime_promote_requires_admin_bearer_when_enabled` | `bots_runtime_promote_and_demote_succeed_with_admin_bearer` |
| `POST /api/v1/bots/runtime/demote` | `bots_runtime_demote_requires_admin_bearer_when_enabled` | `bots_runtime_promote_and_demote_succeed_with_admin_bearer` (204) |
| `POST /api/v1/orders/submit` | `orders_submit_requires_admin_bearer_when_enabled` | `orders_submit_succeeds_with_admin_bearer_when_paper_executor` |
| `POST /api/v1/orders/reconciliation/poll` | `orders_reconciliation_poll_requires_admin_bearer_when_enabled` | `orders_reconciliation_poll_succeeds_with_admin_bearer_when_enabled` |
| `POST /api/v1/monitor/commands` | `monitor_commands_requires_admin_bearer_when_enabled` | `monitor_commands_succeeds_with_admin_bearer_when_handle_present` |

Não cobre auth owner produto; ver [http-admin-auth-seam-sdd.md](../sdd/http-admin-auth-seam-sdd.md).


## Lacunas explícitas

- Não há teste end-to-end contra Binance real; isso é intencional para evitar dependência de rede e credenciais.
- CI não envia ordem testnet real (sem credenciais em pipeline): `recording` + `paper` são determinísticos; `binance_spot_testnet_submit` coberto por testes de contrato (buy/sell exigem credenciais e rede manual; CI usa `recording`/`paper`); `map_bot_error_redacts_configured_testnet_credentials_from_message` garante que valores de `BINANCE_TESTNET_*` não aparecem em `OrdersError::InvalidRequest` mapeado.
- Não há teste de saldo privado ou produção porque esses caminhos são bloqueados.
- A integração Jev externa é validada por contrato/configuração; disponibilidade do serviço e qualidade da recomendação não são gates operacionais.
- O listener HTTP local requer permissão de loopback no ambiente de execução.
- Cada alteração no vendor de `ccxt-core` deve repetir os testes puros e de transporte.

## Critério de atualização

Atualize esta matriz no mesmo change set quando:

1. um módulo ganhar ou perder contrato público;
2. um teste mudar de unidade para integração;
3. uma dependência externa tornar-se obrigatória;
4. uma pendência de SDD mudar de estado;
5. uma execução de integração PostgreSQL produzir evidência reproduzível;
6. a linha `OK:` de `verify-backend-gates.sh` mudar (passed/ignored no bin `bot`) ou testes HTTP admin/orders mudarem de módulo (`http_integration_tests.rs` vs `server.rs`).
