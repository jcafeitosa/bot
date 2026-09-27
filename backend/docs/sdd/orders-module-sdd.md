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
- **Referências:** [Catálogo de módulos](../architecture/module-catalog.md), `modules/exchanges/rest` (`ExecutionDisabled`), `modules/risk` (`OrderIntent`).

## Contexto

O produto não envia ordens reais. Ainda assim, o mapa alvo reserva `modules/orders` para concentrar intenção de execução, validação de risco e o port de exchange — separado de `risk` (gate de sinal) e de adapters REST (backfill público apenas).

## Objetivo

1. Modelar `SubmitOrderRequest` e erros de domínio (`ExecutionDisabled`).
2. Expor `OrderExecutionPort` e `FailClosedExecutor` (sempre `ExecutionDisabled`).
3. `submit_order`: valida via `risk::validate_intent` + `OrderIntent`; só então chama o port (bloqueado).
4. Testes comportamentais determinísticos.

## Não-objetivos

- Execução live, idempotência, reconciliação ou produção.
- HTTP que simule sucesso de envio à exchange (o endpoint `POST /api/v1/orders/submit` valida risco e responde **503** `execution_disabled` com o executor padrão).
- Remover gates `authorize_rest_use` ou habilitar trading live.
- Duplicar política de risco fora de `modules/risk`.

## Seams públicos

| Símbolo | Contrato |
|---------|----------|
| `SubmitOrderRequest` | Campos alinhados a `OrderIntent` + metadados (`symbol`, `side`) sem efeito de exchange. |
| `OrderExecutionPort::execute` | Único caminho para “enviar” ordem. |
| `FailClosedExecutor` | Implementação padrão; nunca chama rede. |
| `submit_order` | Valida risco; retorna `OrdersError::ExecutionDisabled` se risco OK. |

## Validação

`cargo fmt`, `clippy -D warnings`, `cargo test --locked`, `./scripts/check-import-direction.sh`.

## Rollback

Remover `pub mod orders` e diretório `modules/orders/`.
