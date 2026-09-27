---
title: SDD — Módulo orders (fail-closed)
description: Seam de submissão de ordens sem execução live; validação via risk::OrderIntent
tags:
  - sdd
  - backend
  - orders
status: draft
---

# SDD — Módulo `modules/orders`

- **Estado:** implementado (fundação G1 + G2 parcial) — `submit_order`, `PaperLedgerExecutor`, `HttpOrderExecutor` (`paper`/`recording`/`testnet`+credenciais); `client_order_id` memória + PG (`0004`); reconciliação memória/PG + poll HTTP; testnet ccxt via `binance_spot_testnet_submit` (erros mapeados com redação de `BINANCE_TESTNET_*`); prod REST bloqueado.
- **Referências:** [Catálogo de módulos](../architecture/module-catalog.md), `modules/exchanges/rest` (`ExecutionDisabled`), `modules/risk` (`OrderIntent`). Gate 2: [execução live](./orders-live-execution-gate2-sdd.md) (parcial — checklist de fechamento na seção **Critérios de fechamento G2**; exchange pendente).

## Contexto

O produto não envia ordens reais. Ainda assim, o mapa alvo reserva `modules/orders` para concentrar intenção de execução, validação de risco e o port de exchange — separado de `risk` (gate de sinal) e de adapters REST (backfill público apenas).

## Objetivo

1. Modelar `SubmitOrderRequest` e erros de domínio (`ExecutionDisabled`).
2. Expor `OrderExecutionPort` e `FailClosedExecutor` (sempre `ExecutionDisabled`).
3. `submit_order`: valida via `risk::validate_intent` + `OrderIntent`; só então chama o port (bloqueado).
4. Testes comportamentais determinísticos.

## Não-objetivos

- Execução **prod** REST ou trading live fora dos seams documentados (`authorize_rest_use` bloqueado). Reconciliação G2 (memória/PG + poll HTTP + observe testnet) está em [Gate 2](./orders-live-execution-gate2-sdd.md), não nesta lista.
- HTTP que simule sucesso de envio à exchange real (default **503** `execution_disabled`; opt-in local `BOT_ORDERS_EXECUTION=dev_accept` usa double `AcceptingExecutor`, não rede). Bearer admin quando `BOT_HTTP_ADMIN_TOKEN` — ver [SDD HTTP admin](./http-admin-auth-seam-sdd.md).
- Remover gates `authorize_rest_use` ou habilitar trading live.
- Duplicar política de risco fora de `modules/risk`.

## Seams públicos

| Símbolo | Contrato |
|---------|----------|
| `SubmitOrderRequest` | Campos alinhados a `OrderIntent` + metadados (`symbol`, `side`); `paper_fill_unit_price` opcional alimenta `PaperLedgerExecutor` → portfolio `positions`. |
| `OrderExecutionPort::execute` | Único caminho para “enviar” ordem. |
| `FailClosedExecutor` | Implementação padrão; nunca chama rede. |
| `AcceptingExecutor` | Double de teste do port. |
| `PaperLedgerExecutor` | Modo `paper`; grava fills in-process; preço via body HTTP ou `BOT_PAPER_FILL_UNIT_PRICE`. |
| `HttpOrderExecutor` | Seleção em `ApiState` (`disabled`; `dev_accept`; `paper` → `PaperLedgerExecutor`; `live_exchange` → `ExchangeSpotExecutor` quando wired (`recording`/`testnet`+credenciais) ou `ReservedLiveExchangeExecutor`); `live_exchange_wired()` com `GET /meta` e `GET /orders/execution-status`. |
| `ReservedLiveExchangeExecutor` | Placeholder Gate 2; `OrdersError::LiveExchangeNotWired` / HTTP `live_exchange_not_wired`. |
| `RecordingExecutor` | Double determinístico para testes de `submit_order` após risco (sem rede). |
| `submit_order` | Valida risco; retorna `OrdersError::ExecutionDisabled` se risco OK e executor disabled. |
| `OrderIdempotencyStore` / `InMemoryOrderIdempotencyStore` | Dedupe síncrono em processo (`http_bridge/orders::submit_order_http`). |
| `OrderReconciliationLedger` / `shared_live_order_reconciliation_ledger` | Ledger compartilhado HTTP + monitor testnet; `pending`→`reconciled`/`divergent`; `LiveExchangeSpotOrderReconciliationQuery` no poll. |
| `PgOrderReconciliationStore` | Espelha `order_reconciliation` (`0006`); hidrata `symbol`/`side`; `GET` + `POST /orders/reconciliation/poll`. |
| `PgOrderIdempotencyStore` | Dedupe durável (`0004`) quando `DATABASE_URL` conecta. |

## Validação

`./scripts/verify-backend-gates.sh` (fmt, clippy `--bin bot`, import check, bin `bot` `--test-threads=1` + 5 suítes `tests/`); PG **21/21** opcional: `./scripts/verify-backend-full.sh`. Comportamento HTTP: `GET /orders/execution-status` em `server.rs`; `orders_submit_*` (admin bearer, paper, dev_accept, live_exchange) em `http_integration_tests.rs`; DTO/submit em `http_bridge/orders.rs` ([catálogo §3c](../architecture/module-catalog.md#3c-módulo-orders-srcmodulesorders)); `order_execution.rs` + `state.rs` (`ApiState::for_http_server` lê `BOT_ORDERS_EXECUTION`). Evidência: **469** testes bin `bot` (incl. reconciliação poll, `live_query_*`, supervisor `record_monitor_spot_submit_reconciliation`). PG: `pg_order_*` + idempotência/lookup/hydrate em `state.rs` — [test-matrix](../reference/test-matrix.md#integração-opcional-no-bin-bot-script-pg-21--neo4jtestnet-3--24-casos-0-ignore).

## Rollback

Remover `pub mod orders` e diretório `modules/orders/`.
