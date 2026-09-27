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

- **Estado:** implementado (fundação G1) — `submit_order`, `FailClosedExecutor`, `HttpOrderExecutor` em `presentation/http/order_execution.rs` (default fail-closed; `BOT_ORDERS_EXECUTION=dev_accept` só double local); sem exchange live.
- **Referências:** [Catálogo de módulos](../architecture/module-catalog.md), `modules/exchanges/rest` (`ExecutionDisabled`), `modules/risk` (`OrderIntent`). Próximo gate: [Gate 2 execução live](./orders-live-execution-gate2-sdd.md) (draft, não implementado).

## Contexto

O produto não envia ordens reais. Ainda assim, o mapa alvo reserva `modules/orders` para concentrar intenção de execução, validação de risco e o port de exchange — separado de `risk` (gate de sinal) e de adapters REST (backfill público apenas).

## Objetivo

1. Modelar `SubmitOrderRequest` e erros de domínio (`ExecutionDisabled`).
2. Expor `OrderExecutionPort` e `FailClosedExecutor` (sempre `ExecutionDisabled`).
3. `submit_order`: valida via `risk::validate_intent` + `OrderIntent`; só então chama o port (bloqueado).
4. Testes comportamentais determinísticos.

## Não-objetivos

- Execução live, idempotência, reconciliação ou produção.
- HTTP que simule sucesso de envio à exchange real (default **503** `execution_disabled`; opt-in local `BOT_ORDERS_EXECUTION=dev_accept` usa double `AcceptingExecutor`, não rede). Bearer admin quando `BOT_HTTP_ADMIN_TOKEN` — ver [SDD HTTP admin](./http-admin-auth-seam-sdd.md).
- Remover gates `authorize_rest_use` ou habilitar trading live.
- Duplicar política de risco fora de `modules/risk`.

## Seams públicos

| Símbolo | Contrato |
|---------|----------|
| `SubmitOrderRequest` | Campos alinhados a `OrderIntent` + metadados (`symbol`, `side`) sem efeito de exchange. |
| `OrderExecutionPort::execute` | Único caminho para “enviar” ordem. |
| `FailClosedExecutor` | Implementação padrão; nunca chama rede. |
| `AcceptingExecutor` | Double de teste do port. |
| `HttpOrderExecutor` | Seleção em `ApiState` (`disabled` default; `dev_accept` via `BOT_ORDERS_EXECUTION`). |
| `submit_order` | Valida risco; retorna `OrdersError::ExecutionDisabled` se risco OK e executor disabled. |

## Validação

`./scripts/verify-backend-gates.sh` (fmt, clippy `--bin bot`, import check, `cargo test --locked`). Comportamento HTTP: `orders_submit_*` e `orders_submit_dev_accept_executor_returns_200` em `server.rs`; `order_execution.rs` + `state.rs` (`ApiState::for_http_server` lê `BOT_ORDERS_EXECUTION`). Evidência: **235** testes bin `bot`.

## Rollback

Remover `pub mod orders` e diretório `modules/orders/`.
