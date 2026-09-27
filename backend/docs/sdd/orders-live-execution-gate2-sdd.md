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

- **Estado:** **parcial** — G1 + `HttpOrderExecutor` (`disabled`, `dev_accept`, `paper` → `PaperLedgerExecutor`; `live_exchange` + `BOT_ORDERS_EXCHANGE_SUBMIT=recording` → `ExchangeSpotExecutor` / `live_exchange_wired`; sem submit backend → `live_exchange_not_wired`); dedupe `client_order_id` (memória + `PgOrderIdempotencyStore` / `0004`). Adapter exchange Spot/testnet pendente.
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
| `ExchangeSpotExecutor` | `live_exchange` wired: `authorize_rest_use` + `RecordingSpotOrderSubmitPort` (seam determinístico). |
| `ReservedLiveExchangeExecutor` | `BOT_ORDERS_EXECUTION=live_exchange` sem backend → `OrdersError::LiveExchangeNotWired` / HTTP `live_exchange_not_wired` até adapter real. |
| `RecordingExecutor` | Double in-process (contagem de chamadas); `submit_invokes_recording_executor_once_after_risk` + `submit_order_http_records_execution_with_recording_executor` (sem rede). |

## Validação (baseline G1 antes de Gate 2)

```text
./scripts/verify-backend-gates.sh
```

Evidência G1 (2026-09-27): **321** testes bin `bot`, **6** ignorados; `orders_submit_fail_closed_returns_503`, `orders_submit_dev_accept_executor_returns_200`, `orders_submit_live_exchange_reserved_returns_503_with_code`, `orders_submit_live_exchange_wired_returns_200`, `orders_submit_paper_executor_returns_200` (+ snapshot portfolio **995**), `meta_and_orders_execution_status_live_exchange_wired_true`, `HttpOrderExecutor` + `BOT_ORDERS_EXECUTION` (`paper` → ledger; `live_exchange`+recording → `ExchangeSpotExecutor`), `duplicate_client_order_id_replays_without_second_execute`, `GET /orders/execution-status`.

## Validação Gate 2 (quando implementado)

- Testes unitários do fake executor + idempotência.
- HTTP: 200/202 apenas com executor de teste injetado; default build mantém 503.
- Revisão Critic + threat model (credenciais, replay, rate limit).
- `./scripts/verify-backend-gates.sh` verde.

## Rollout / rollback

- Feature flag ou config `orders.execution = disabled|paper|testnet` com default `disabled`.
- Rollback: reverter wiring em `ApiState` para `FailClosedExecutor` único.

## Pendências de decisão

- Escopo inicial: **paper ledger** implementado (`PaperLedgerExecutor`); testnet Spot via adapter exchange ainda pendente.
- Store de idempotência: memória vs PostgreSQL (`0002` ou migração nova).
- Autorização owner/agency além de `BOT_HTTP_ADMIN_TOKEN` (Gate 1 auth).

## Critérios de fechamento G2 (checklist)

| Critério | Evidência atual | Fechado |
|----------|-----------------|--------|
| `HttpOrderExecutor` + `BOT_ORDERS_EXECUTION` | `order_execution.rs`, testes `orders_submit_*` | Sim |
| Idempotência `client_order_id` (memória + PG opcional) | `PgOrderIdempotencyStore`, `duplicate_client_order_id_*` | Sim |
| `live_exchange_not_wired` até adapter real | `ReservedLiveExchangeExecutor`, meta + execution-status | Sim (seam) |
| `RecordingExecutor` / test double sem rede | `orders/tests.rs`, `http_bridge/orders.rs` | Sim |
| `PaperLedgerExecutor` (modo `paper`) | `paper_ledger_executor.rs`, `orders_submit_paper_executor_returns_200` | Sim |
| Adapter `OrderExecutionPort` com exchange/testnet REST | ccxt/testnet pendente | **Não** (recording seam ok) |
| `live_exchange_wired == true` com prova determinística | `HttpOrderExecutor::live_exchange` + testes `from_env_live_exchange_wired_*`, `orders_submit_live_exchange_wired_returns_200`, `meta_and_orders_execution_status_live_exchange_wired_true` | **Parcial** (recording; não testnet REST) |
| Threat model + revisão Critic | seção rascunho neste SDD; Critic instância separada | **Parcial** |
| `./scripts/verify-backend-gates.sh` verde | **321** testes bin `bot` (2026-09-27) | Sim (baseline G1/G2 parcial) |

## Threat model (rascunho)

| Risco | Mitigação atual | Gap |
|-------|-----------------|-----|
| Envio acidental de ordem live | Default `BOT_ORDERS_EXECUTION` fail-closed; `authorize_rest_use` bloqueia `OrderSubmit` exceto seam `recording` em dev Spot | REST testnet/prod ainda desabilitado |
| Replay de `client_order_id` | `OrderIdempotencyStore` memória + PG `0004` | TTL/expiração operacional não definida |
| Credenciais testnet em log | Sem adapter REST de ordem em CI; recording não usa rede | Revisar logging ao ligar ccxt |
| Bypass de risco | `submit_order` sempre chama `risk::validate_intent` antes do port | — |
| Admin token vazado | `BOT_HTTP_ADMIN_TOKEN` em rotas mutantes; não substitui auth owner | [agents G1](./agents-module-sdd.md) |

Revisão Critic e hardening de produção permanecem **pendentes** antes de fechar G2.
