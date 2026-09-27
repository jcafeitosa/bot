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

- **Estado:** implementado (fundação G1) — `submit_order`, `FailClosedExecutor`, testes em `modules/orders/tests.rs` e HTTP fail-closed em `presentation/http/server.rs`; sem exchange live.
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
- HTTP que simule sucesso de envio à exchange (o endpoint `POST /api/v1/orders/submit` valida risco e responde **503** `execution_disabled` com o executor padrão; exige bearer admin quando `BOT_HTTP_ADMIN_TOKEN` está definido — ver [SDD HTTP admin](./http-admin-auth-seam-sdd.md)).
- Remover gates `authorize_rest_use` ou habilitar trading live.
- Duplicar política de risco fora de `modules/risk`.

## Seams públicos

| Símbolo | Contrato |
|---------|----------|
| `SubmitOrderRequest` | Campos alinhados a `OrderIntent` + metadados (`symbol`, `side`) sem efeito de exchange. |
| `OrderExecutionPort::execute` | Único caminho para “enviar” ordem. |
| `FailClosedExecutor` | Implementação padrão; nunca chama rede. |
| `AcceptingExecutor` | Double de teste do port (não usado em `ApiState` HTTP). |
| `submit_order` | Valida risco; retorna `OrdersError::ExecutionDisabled` se risco OK. |

## Validação

`./scripts/verify-backend-gates.sh` (fmt, clippy `--bin bot`, import check, `cargo test --locked`). Comportamento HTTP: `orders_submit_*` em `presentation/http/server.rs`; `submit_order_fail_closed_via_state_returns_execution_disabled` em `presentation/http/state.rs` (`ApiState::submit_order_http` + `FailClosedExecutor`).

## Rollback

Remover `pub mod orders` e diretório `modules/orders/`.
