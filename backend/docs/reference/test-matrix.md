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
| `agents` | Registry, lifecycle, `assert_runtime_promotion_authorized` (capability + not-active), PG snapshot. |
| `http_bridge/agents` | `apply_agent_identity_snapshot` no-op quando registry já populado (cold-start). |
| `http_bridge/config` | `map_config` expõe `monitor_registry` com `evaluator` (`map_config_preserves_ema_evaluator_on_registry_entry`). |
| `http_bridge/monitor` | `attach_bot_runtime_status` enriquece snapshot HTTP (incl. `sma-cross@2`). |
| `http_bridge/orders` | `submit_order_http_records_execution_with_recording_executor`; dedupe `client_order_id`; HTTP `orders_submit_*` (paper, live_exchange wired/recording, reserved) em `server.rs`. |
| `modules/orders` reconciliation | `reconciliation_pending_to_reconciled`, `reconciliation_mark_divergent_from_pending`, `reconciliation_seed_entry_restores_pending_count`; `recording_submit_returns_deterministic_exchange_order_id`. |
| `presentation/http/state` | `submit_order_recording_live_exchange_auto_reconciles_client_order_id`. |
| `http_bridge/portfolio` | `paper_wallet_snapshot_reflects_in_process_ledger`; HTTP paper submit + `GET /portfolio/paper-snapshot` em `server.rs`. |
| `http_bridge/bots` | Catálogo com `monitor_evaluator` (`catalog_for_config_exposes_ema_evaluator_from_registry`); promote/catalog gates v1/v2. |
| `orders` | `PaperLedgerExecutor`; `ExchangeSpotExecutor` + `submit_spot_order` (`testnet_backend_is_not_wired_yet`); `RecordingExecutor`; `ReservedLiveExchangeExecutor`; idempotência PG (ignorado). |
| `bots` | `MonitorEvaluatorKind`; `strategy_evaluation_binding_uses_ema_evaluator_from_registry`; runtime promote; catálogo multi-estratégia. |
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
| `presentation/http` | OpenAPI **35** paths; `GET /meta` + `meta_and_*`; catálogo/config `monitor_evaluator` / `evaluator` (`config_active_preserves_ema_evaluator_on_monitor_registry`); bots runtime; orders/agents HTTP. | `server.rs`, `state.rs`, `routes/meta.rs`, `admin_auth.rs`, `order_execution.rs`. |
| `persistence` | Round-trip de migração, gravação e contagem, condicionado a PostgreSQL. |

## Verificação executada

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
./scripts/check-import-direction.sh
./scripts/verify-backend-gates.sh

cargo check --locked --all-targets
exit 0; sem warnings

cargo test --locked
350 testes unitários passaram (bin bot) (HTTP em `presentation/http/server.rs` e `state.rs` `state_tests`)
1 fixture + 2 config CLI + 3 redirect-origin + 2 redirect-policy HTTP passaram
6 testes ignorados (PostgreSQL 18+ / Neo4j; ver `#[ignore]` em persistence, pg_catalog, pg identity, pg order idempotency, market, neo4j)
```

Bin `bot`: **350** aprovados, **8** ignorados (incl. `integration_submits_minimal_market_buy_on_testnet` manual testnet). PG: `./scripts/run-pg-integration-tests.sh` com `DATABASE_URL` → `trading_bot` (Timescale + pgvector). Neo4j: teste `ping_and_node_count_against_local_graph` separado (`BOT_AGENTS_ENABLED` + compose `graph`).

## Lacunas explícitas

- Não há teste end-to-end contra Binance real; isso é intencional para evitar dependência de rede e credenciais.
- CI não envia ordem testnet real (sem credenciais em pipeline): `recording` + `paper` são determinísticos; `binance_spot_testnet_submit` coberto por testes de contrato (buy/sell exigem credenciais e rede manual; CI usa `recording`/`paper`).
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
5. uma execução de integração PostgreSQL produzir evidência reproduzível.
