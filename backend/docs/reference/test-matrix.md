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
| `strategy` | Períodos por operação e sinais de cruzamento. |
| `risk` | Limite conservador/agressivo, tamanho, stop/take profit e incompatibilidade de modo. |
| `agents` | Registry, hierarquia, lifecycle, advisory, `assert_runtime_promotion_authorized`, `restore_from_snapshot`, `PgAgentIdentityStore` SQL mapping. |
| `http_bridge/agents` | `apply_agent_identity_snapshot` no-op quando registry já populado (cold-start). |
| `http_bridge/monitor` | `attach_bot_runtime_status` enriquece snapshot HTTP com promoção ativa. |
| `http_bridge/orders` | `submit_order_http` com `FailClosedExecutor` → `ExecutionDisabled`; com `AcceptingExecutor` → `accepted: true`; `client_order_id` duplicado não reexecuta o port (`duplicate_client_order_id_replays_without_second_execute`). |
| `http_bridge/bots` | `assert_catalog_contains_bot`, `assert_bot_promotion_allowed` (catálogo + `bot_id_matches_market`); `catalog_for_config_includes_monitor_strategy_periods`; testes em `catalog_gate_tests`. |
| `orders` | `submit_order` rejeita acima do cap de risco; após risco OK retorna `ExecutionDisabled`; `AcceptingExecutor` cobre caminho aceito no port; `InMemoryOrderIdempotencyStore` + `pg_order_idempotency_round_trip` (ignorado). |
| `bots` | Identidade, ranking, `build_catalog_from_monitor_registry` (multi-estratégia registrada), `BotRuntimePort` + `shared_bot_runtime()`, `MonitorStrategyRegistry`, `strategy_evaluation_binding`, métricas coerentes com `BotId`. |
| `portfolio` | Snapshot paper, ativos, posição e erro de inconsistência. |
| `backtest` | Fees, next-open, slippage na venda, stop/take-profit, histórico insuficiente e ausência de lookahead. |
| `domain` | Ranking, métricas, janela de avaliação e tipos de identidade. |
| `exchanges/mod` | Chave de conta estável e rótulo vazio rejeitado. |
| `exchanges/account_file` | Parsing, seleção de ambiente e validação de origem. |
| `exchanges/binance` | Origem testnet, dados fechados válidos, duplicata conflitante, alinhamento e finitude. |
| `exchanges/bootstrap` | Registro default e seleção de conta Spot. |
| `exchanges/capabilities` | Catálogo coerente de capacidades. |
| `exchanges/live` | Endpoint seguro, rejeição de origem não confiável, filtro de candle aberto e payload inválido. |
| `exchanges/registry` | Registro, consulta e duplicidade de contas. |
| `exchanges/resources` | Recursos autorizados e plano por ambiente. |
| `exchanges/rest` | Somente backfill público Spot em dev é autorizado; usos privados falham. |
| `exchanges/router` | Mapeamento de necessidade para transporte e streams default. |
| `exchanges/stream` | Assinatura e validação de evento. |
| `exchanges/ws` | Configuração e plano `1m` validado. |
| `app` | Pause sem bloquear, resume com drain, stale REST/WS, gerações, falha de resume, cancelamento, shutdown e fila de persistência. |
| `ui` | Comando de espaço de acordo com o estado confirmado. |
| `presentation/http` | Rotas Axum, OpenAPI, `ApiState` (`persist_catalog_then_promote`, agency-bound promote capability, agents capabilities no GET, orders idempotency), boot catálogo PG, `server` promote HTTP. | `server.rs`, `state.rs` (`state_tests`), `admin_auth.rs`, `order_execution.rs`. |
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
265 testes unitários passaram (bin bot) (HTTP em `presentation/http/server.rs` e `state.rs` `state_tests`)
1 fixture + 2 config CLI + 3 redirect-origin + 2 redirect-policy HTTP passaram
6 testes ignorados (PostgreSQL 18+ / Neo4j; ver `#[ignore]` em persistence, pg_catalog, pg identity, pg order idempotency, market, neo4j)
```

Bin `bot`: 265 aprovados, 6 ignorados. Testes `#[ignore]` de PG/Neo4j exigem `DATABASE_URL` → `trading_bot` (PG 18+, extensões) e/ou stack Neo4j local.

## Lacunas explícitas

- Não há teste end-to-end contra Binance real; isso é intencional para evitar dependência de rede e credenciais.
- Não há teste de ordem, saldo privado ou produção porque esses caminhos são bloqueados.
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
