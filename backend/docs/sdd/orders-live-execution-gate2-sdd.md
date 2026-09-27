---
title: SDD — Gate 2 execução de orders (exchange live)
description: Próxima fase após fail-closed G1; adapter, idempotência e gates de segurança
tags:
  - sdd
  - backend
  - orders
  - security
status: draft
---

# SDD — Gate 2: execução de orders (exchange)

- **Estado:** **parcial** — G1 + `HttpOrderExecutor` (`paper`, `recording`, `testnet`+credenciais); submit testnet ccxt; reconciliação memória/PG + `POST /orders/reconciliation/poll` + `LiveExchangeSpotOrderReconciliationQuery` (binding + `observe_testnet_spot_order_by_client_id`); dedupe `client_order_id` (memória + PG `0004`). Pendente: LGTM **Critic** (threat model ops doc + redação credenciais); prod REST bloqueado.
- **Referências:** [SDD orders G1](./orders-module-sdd.md), [auditoria de completude](../planning/modules-completeness-audit.md), `modules/exchanges/rest`, `modules/risk`.

## Contexto

`submit_order` já valida `OrderIntent` via `modules/risk`. `ApiState` injeta `FailClosedExecutor` em `submit_order_http`. Nenhum caminho autorizado chama REST privado de ordem hoje.

## Objetivo (quando aprovado)

1. Novo adapter `OrderExecutionPort` (ex.: `ExchangeSpotExecutor`) atrás de `authorize_rest_use` explícito para o caso de uso de ordem.
2. Chave de idempotência (`client_order_id` / dedupe em store) e resposta HTTP distinguindo risco rejeitado vs falha de execução vs aceite.
3. Configuração fail-closed: executor live só com flags/documentação operacional; default permanece `FailClosedExecutor`.
4. Testes determinísticos com fake port (sem rede); testes de integração opcionais ignorados com credenciais testnet.

## Não-objetivos

- Produção ou bypass de `Config::validate` para trading live.
- Ordens sem passar por `risk::validate_intent`.
- Simular HTTP 200 de sucesso na exchange enquanto G1 `execution_disabled` for o default em builds de observação.

## Seams públicos (propostos)

| Símbolo | Contrato |
|---------|----------|
| `OrderExecutionPort::execute` | Entrada já validada por risco; retorna `OrderAck` ou erro de domínio mapeável a HTTP. |
| `OrderIdempotencyStore` / `InMemoryOrderIdempotencyStore` | Dedupe em processo; replay HTTP. |
| `PgOrderIdempotencyStore` | Dedupe durável em `order_idempotency_keys` quando PG no `ApiState`; lookup antes de executar + `INSERT ON CONFLICT DO NOTHING` após sucesso. |
| `ApiState::order_executor` | `HttpOrderExecutor` via `HttpApiSeams::from_env()` no `for_http_server`; default fail-closed. |
| `HttpOrderExecutor::live_exchange_wired` / `ApiState::live_exchange_wired` | Fonte única para `GET /meta` e `GET /orders/execution-status`; `true` no modo `LiveExchange` (hoje via seam `recording`; testnet REST pendente). |
| `AcceptingExecutor` | Usado apenas em modo `dev_accept` (não é adapter de exchange). |
| `PaperLedgerExecutor` | `BOT_ORDERS_EXECUTION=paper` → ledger in-process após risco; `live_exchange_wired` permanece `false`. |
| `submit_spot_order` / `LiveExchangeSubmitBackend` | `recording` (in-process); `testnet` + credenciais → `binance_spot_testnet_submit` (ccxt market buy / `quoteOrderQty`). |
| `ExchangeSpotExecutor` | `gate_order_submit` + `submit_spot_order` para `live_exchange` wired. |
| `ReservedLiveExchangeExecutor` | `BOT_ORDERS_EXECUTION=live_exchange` sem backend → `OrdersError::LiveExchangeNotWired` / HTTP `live_exchange_not_wired` até adapter real. |
| `RecordingExecutor` | Double in-process (contagem de chamadas); `submit_invokes_recording_executor_once_after_risk` + `submit_order_http_records_execution_with_recording_executor` (sem rede). |
| `OrderReconciliationLedger` / `InMemoryOrderReconciliationLedger` | Live exchange + `client_order_id`: `mark_pending` no HTTP; `confirm_exchange_order` com `SpotOrderSubmitAck.exchange_order_id` (`recording-N` ou id ccxt testnet). |
| `SpotOrderSubmitAck` / `take_last_spot_submit_ack` | Retorno de `submit_spot_order`; consumido em `ApiState::submit_order_http` após submit wired. |

## Validação (baseline G1 antes de Gate 2)

```text
./scripts/verify-backend-gates.sh
```

Evidência G1 (2026-09-27): **387** testes bin `bot`, **11** ignorados; `orders_submit_fail_closed_returns_503`, `orders_submit_dev_accept_executor_returns_200`, `orders_submit_live_exchange_reserved_returns_503_with_code`, `orders_submit_live_exchange_wired_returns_200`, `orders_submit_paper_executor_returns_200` (+ snapshot portfolio **995**), `meta_and_orders_execution_status_live_exchange_wired_true`, `HttpOrderExecutor` + `BOT_ORDERS_EXECUTION` (`paper` → ledger; `live_exchange`+recording → `ExchangeSpotExecutor`), `duplicate_client_order_id_replays_without_second_execute`, `GET /orders/execution-status`.

## Validação Gate 2 (quando implementado)

- Testes unitários do fake executor + idempotência.
- HTTP: 200/202 apenas com executor de teste injetado; default build mantém 503.
- Revisão Critic + threat model (credenciais, replay, rate limit).
- `./scripts/verify-backend-gates.sh` verde.

## Rollout / rollback

- Feature flag ou config `orders.execution = disabled|paper|testnet` com default `disabled`.
- Rollback: reverter wiring em `ApiState` para `FailClosedExecutor` único.

## Pendências de decisão

- Escopo inicial: **paper ledger** implementado; testnet Spot market buy/sell via `binance_spot_testnet_submit` + reconciliação parcial (binding + observe + poll).
- Idempotência: memória + PostgreSQL (`0004_order_idempotency_keys.sql`); reconciliação `0006_order_reconciliation.sql` — ver [catálogo §3c](../architecture/module-catalog.md#3c-módulo-orders-srcmodulesorders).
- Autorização owner/agency além de `BOT_HTTP_ADMIN_TOKEN` (Gate 1 auth).

## Critérios de fechamento G2 (checklist)

| Critério | Evidência atual | Fechado |
|----------|-----------------|--------|
| `HttpOrderExecutor` + `BOT_ORDERS_EXECUTION` | `order_execution.rs`, testes `orders_submit_*` | Sim |
| Idempotência `client_order_id` (memória + PG opcional) | `PgOrderIdempotencyStore`, `duplicate_client_order_id_*` | Sim |
| `live_exchange_not_wired` até adapter real | `ReservedLiveExchangeExecutor`, meta + execution-status | Sim (seam) |
| `RecordingExecutor` / test double sem rede | `orders/tests.rs`, `http_bridge/orders.rs` | Sim |
| `PaperLedgerExecutor` (modo `paper`) | `paper_ledger_executor.rs`, `orders_submit_paper_executor_returns_200` | Sim |
| Adapter `OrderExecutionPort` com exchange/testnet REST | `binance_spot_testnet_submit.rs` (buy/sell market por quote); CI sem credenciais | **Parcial** |
| Reconciliação pós-submit | Memória + PG `0006`; GET/POST reconciliation; `LiveExchangeSpotOrderReconciliationQuery` (binding + `observe_testnet_spot_order_by_client_id`); job poll opcional | **Parcial** (prod REST; Critic) |
| `live_exchange_wired == true` com prova determinística | `HttpOrderExecutor::live_exchange` + testes `from_env_live_exchange_wired_*`, `orders_submit_live_exchange_wired_returns_200`, `meta_and_orders_execution_status_live_exchange_wired_true` | **Parcial** (recording determinístico; testnet exige credenciais/rede) |
| Threat model + revisão Critic | Tabela de riscos + retenção ops ([cli-and-config](../reference/cli-and-config.md#pg-orders-retention-gate-2)); Critic instância separada | **Parcial** (ops doc ok; Critic + purge PG automatizado pendentes) |
| `./scripts/verify-backend-gates.sh` verde | **387** testes bin `bot` (2026-09-27) | Sim (baseline G1/G2 parcial) |

## Threat model (rascunho)

| Risco | Mitigação atual | Gap |
|-------|-----------------|-----|
| Envio acidental de ordem live | Default `BOT_ORDERS_EXECUTION` fail-closed; `authorize_rest_use` permite `OrderSubmit` só com seam `recording` ou testnet+credenciais em dev Spot | Prod REST desabilitado; testnet opt-in explícito |
| Replay de `client_order_id` | `OrderIdempotencyStore` memória + PG `0004` | Política de retenção **ops** documentada abaixo; job de purge PG **não** implementado |
| Credenciais testnet em log/resposta | CI sem credenciais; `map_bot_error` redige valores de `BINANCE_TESTNET_*` em mensagens (`redact_known_testnet_credentials`; teste `map_bot_error_redacts_configured_testnet_credentials_from_message`) | Revisar tracing ccxt em outros adapters; Critic |
| Bypass de risco | `submit_order` sempre chama `risk::validate_intent` antes do port | — |
| Admin token vazado | `BOT_HTTP_ADMIN_TOKEN` em rotas mutantes; não substitui auth owner | [agents G1](./agents-module-sdd.md) |
| Estado de reconciliação inconsistente | Memória + PG; GET reconciliation; `POST /orders/reconciliation/poll` + job `BOT_ORDERS_RECONCILIATION_POLL_SECS` (recording) | Consulta testnet via fetch_order + binding; prod ausente |
| Vazamento de `exchange_order_id` em logs HTTP | Resposta JSON só em GET reconciliation; submit retorna `accepted` apenas | Revisar tracing em adapters ccxt |

Revisão Critic e hardening de produção permanecem **pendentes** antes de fechar G2.

### Próximo slice: poller de reconciliação (design)

1. Listar `state = 'pending'` (PG) ou `pending_count` > 0 (memória).
2. Consultar status na exchange (testnet ccxt; **sem** prod).
3. `confirm_exchange_order` ou `mark_divergent` (memória + `upsert_state` PG).
4. Job opcional no `serve` (intervalo configurável; default desligado).

Implementado (recording): `run_reconciliation_poll_once`, `SpotOrderReconciliationQuery` + `recording_bind_client_exchange` no submit HTTP wired; job periódico no `serve` (`BOT_ORDERS_RECONCILIATION_POLL_SECS`); `LiveExchangeSpotOrderReconciliationQuery` (recording + testnet via binding map); consulta REST testnet por `client_order_id` **pendente**.

### Critérios para sair de “rascunho” (threat model)

- [ ] Critic independente registra LGTM com achados tratados ou aceitos.
- [x] TTL/retenção de `order_idempotency_keys` e `order_reconciliation` definidos (ops) — ver recomendação abaixo e [cli-and-config](../reference/cli-and-config.md#pg-orders-retention-gate-2); purge automatizado permanece fora do código até decisão SRE.
- [x] Prod REST permanece bloqueado em `authorize_rest_use` até decisão explícita — evidência: `modules/exchanges/rest.rs` (`only_public_spot_dev_backfill_is_allowed` com `Environment::Prod`; `order_rest_paths_stay_disabled` sem seam).

**Recomendação ops (não automatizada no código):** retenção sugerida `order_idempotency_keys` **90 dias**; linhas `order_reconciliation` em estado terminal (`reconciled`/`divergent`) **180 dias**; pendências além de **7 dias** devem acionar alerta + poll manual (`POST /orders/reconciliation/poll`). Job `BOT_ORDERS_RECONCILIATION_POLL_SECS` ≥ **60** em ambientes com submit live wired.
