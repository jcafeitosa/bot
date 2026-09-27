---
title: SDD W0-09 — V18: nota de verificação e delta restante
description: Verifica no código o que o commit fcaf14f8 entregou para V18 e lista o que falta do critério W0-09 (rollback com outbox, limpeza, evidência)
tags:
  - sdd
  - backend
  - persistence
  - postgres
  - wave0
status: draft
---

# SDD W0-09 — V18: nota de verificação e delta restante

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1 do delta).
- **Plano:** W0-09 em [master-plan](../planning/master-plan.md) §4.1. Critério: erro injetado no meio de TX de domínio (identidade+outbox, idempotência+outbox) → nada persiste, outbox vazia; limpeza verificada (DB dropado ou schema vazio); evidência com host/versão.
- **SDD existente:** [monitor-persistence-v18-sdd](./monitor-persistence-v18-sdd.md). Este arquivo não o substitui; registra a verificação e o que falta.

## Verificação (HEAD `b8370a75`, working tree de 2026-09-27 ~16:45 COT)

O commit `fcaf14f8` ("V18 PG rollback e idempotência de persist_dataset") diz fechar W0-09/T-15. **Está parcial.**

| Item do critério | Estado | Evidência |
|---|---|---|
| Rollback do caminho de candles | feito, com ressalva | `core/persistence/v18_pg_tests.rs:101` (`pg_persist_dataset_transaction_rollback_and_idempotent_replay`). O helper `insert_dataset_in_tx_then_rollback` (:63-99) **copia o SQL** de `persist_dataset` numa TX própria e faz `ROLLBACK` manual: prova o Postgres, não o código de `persist_dataset`. A parte de replay idempotente usa o código real. |
| Manifesto conflitante | feito | `v18_pg_tests.rs:157` (commit `0b3ea04b`) |
| Identidade + outbox | feito (commit `b8370a75`, 16:34 COT) | `modules/agents/adapters/pg_registry.rs:404` (`pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure`) chama a função real `persist_identity_and_enqueue_graph_projection_tx` e faz `rollback()` depois do enqueue; seção "Fatia 2" em [monitor-persistence-v18-sdd](./monitor-persistence-v18-sdd.md) |
| Idempotência + outbox (orders) | **em andamento, não commitado, por outra sessão** | em HEAD, `orders/adapters/pg_idempotency.rs:107-134` abre e fecha a própria TX. A mudança local cria `persist_idempotency_and_enqueue_graph_projection_tx` (`pub(crate)`) e o teste `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure`, com `rollback()` explícito (é o plano B abaixo) |
| Limpeza verificada | **falta** | os testes apagam só as linhas do fixture (`delete_v18_fixture`); nada prova DB dropado ou schema vazio |
| Evidência host/versão | **falta** | `scripts/pg-v18-monitor-persistence-audit.sh` só recusa host não local (:17-24); não registra `server_version` nem host |
| Não passar em skip | **falta** (W0-02) | todos usam `database_for_integration_test()`, que devolve `None` e o teste retorna verde sem PG (`core/persistence/pg_integration.rs:6-7`) |

## Delta restante (decisão)

1. **Idempotência + outbox:** teste PG que chama `persist_idempotency_and_enqueue_graph_projection` com dois messages, o segundo inválido para o PG (por exemplo, `idempotency_key` com byte `\0`, que o PG recusa em `TEXT`). Espera: `Err`, e zero linhas em `order_idempotency_keys` e em `graph_projection_outbox` para o fixture. Isso prova o caminho de erro do código real, sem seam só de teste.
2. **Candles:** trocar a cópia de SQL por injeção equivalente no `persist_dataset` real (candle inválido no fim do lote) ou registrar no SDD base que o teste prova só o Postgres.
3. **Limpeza e evidência:** fica com o job de W0-02 (DB descartável por execução). O job registra `SELECT version()` e o host no log e termina com o DB dropado; o script V18 passa a imprimir `server_version`.
4. Identidade + outbox (commitado) e idempotência + outbox (em andamento): o Critic confere se `rollback()` explícito depois do enqueue basta para o critério "erro injetado no meio da TX". Se não bastar, somar o teste do item 1 (erro real do código), sem remover os existentes.

**Alternativa considerada:** criar variante `_tx(&mut tx, …)` de `persist_idempotency_and_enqueue_graph_projection`, como já existe em agents, e fazer `rollback()` no teste. Mais uniforme, mas prova só que o PG desfaz uma TX aberta pelo teste, não que o código de produção desfaz em erro. É o caminho que a outra sessão está seguindo; aceitável se o Critic concordar (item 4).

## Seams públicos

Nenhum no caminho recomendado. No plano B: `PostgresOrderIdempotencyStore::persist_idempotency_and_enqueue_graph_projection_tx` (visibilidade `pub(crate)`).

## Dependências

- W0-02: limpeza verificada, evidência de host/versão e falha sem PG.
- Habilita W0-10 (G4 de T-15).

## Riscos

- A injeção por `\0` depende do comportamento do driver/PG; se o sqlx recusar antes de enviar, o erro ainda acontece no meio da TX (depois do `INSERT` da chave), o que basta.
- Contagem do manifesto muda de novo; usar a linha `OK:` e o manifesto, não número copiado.

## Validação

- `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável; `backend/scripts/verify-backend-gates.sh`.

## Rollout / rollback

- Só testes e scripts; sem efeito em runtime. Rollback: reverter o commit.
