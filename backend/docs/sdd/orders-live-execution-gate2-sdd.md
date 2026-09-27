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

- **Estado:** **parcial** — G1 + seam `HttpOrderExecutor` / `BOT_ORDERS_EXECUTION` (`disabled` default, `dev_accept` double local). Exchange live, idempotência e auth owner permanecem fora de escopo até aprovação deste gate.
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
| `OrderIdempotencyStore` | Evita duplicata de submit com mesma chave (memória ou PG — decisão no SDD de persistência). |
| `ApiState::order_executor` | `HttpOrderExecutor` via `HttpApiSeams::from_env()` no `for_http_server`; default fail-closed. |
| `AcceptingExecutor` | Usado apenas em modo `dev_accept` (não é adapter de exchange). |

## Validação (baseline G1 antes de Gate 2)

```text
./scripts/verify-backend-gates.sh
```

Evidência G1 (2026-09-27): **227** testes bin `bot`, **5** ignorados; `orders_submit_fail_closed_returns_503`, `orders_submit_dev_accept_executor_returns_200`, `HttpOrderExecutor` + `BOT_ORDERS_EXECUTION`.

## Validação Gate 2 (quando implementado)

- Testes unitários do fake executor + idempotência.
- HTTP: 200/202 apenas com executor de teste injetado; default build mantém 503.
- Revisão Critic + threat model (credenciais, replay, rate limit).
- `./scripts/verify-backend-gates.sh` verde.

## Rollout / rollback

- Feature flag ou config `orders.execution = disabled|paper|testnet` com default `disabled`.
- Rollback: reverter wiring em `ApiState` para `FailClosedExecutor` único.

## Pendências de decisão

- Escopo inicial: paper ledger vs testnet Spot apenas.
- Store de idempotência: memória vs PostgreSQL (`0002` ou migração nova).
- Autorização owner/agency além de `BOT_HTTP_ADMIN_TOKEN` (Gate 1 auth).
