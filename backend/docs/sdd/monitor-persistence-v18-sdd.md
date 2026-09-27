---
title: SDD addendum — V18 persistência monitor (PostgreSQL)
description: Fatia W0-09 / gate G4 parcial para T-15 — rollback e idempotência em trading_bot isolado
tags:
  - sdd
  - monitor
  - persistence
  - v18
---

# Addendum V18 — persistência de mercado (T-15)

- **Escopo:** primeira fatia formal de [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) item 3 (integração PostgreSQL isolada).
- **Não escopo:** injeção de falha no meio de `persist_dataset` em produção; reconciliação `GAP`; ordens reais.

## Seam público exercitado

- `Database::persist_dataset` em `core/persistence/mod.rs` (transação única manifest + `candles_1m`, `ON CONFLICT DO NOTHING`).

## Entrega

| Artefato | Caminho |
|----------|---------|
| Teste PG | `pg_persist_dataset_transaction_rollback_and_idempotent_replay` em `core/persistence/v18_pg_tests.rs` |
| Preflight auditado | `scripts/pg-v18-monitor-persistence-audit.sh` (exige `DATABASE_URL` → `trading_bot` em host local) |
| Manifesto PG | `scripts/run-pg-integration-tests.sh` (+1 caso) |

## Comportamento comprovado

1. Transação espelhando inserts de `persist_dataset` seguida de `ROLLBACK` não deixa linhas em `market_datasets` nem `candles_1m`.
2. `persist_dataset` com commit e replay idempotente mantém contagem de candles estável.
3. Teardown do fixture por `dataset_id` no próprio teste.

## Validação

```bash
export DATABASE_URL=postgresql://USER:PASS@127.0.0.1:55433/trading_bot
./scripts/pg-v18-monitor-persistence-audit.sh
./scripts/verify-backend-gates.sh
./scripts/verify-backend-full.sh   # gates + manifesto PG completo
```

## Fatia 2 — W0-09 domínio + outbox (agents)

> **Estado em `origin/main` (`d42b71a5`): não entregue.** O seam `persist_identity_and_enqueue_graph_projection_tx` e o teste não existem no código; a fatia foi revertida em `2089a496`. W0-09 volta pelo G1 em [wave0-09-v18-verificacao-sdd](./wave0-09-v18-verificacao-sdd.md). A tabela abaixo descreve o desenho proposto, não uma entrega.

| Artefato | Caminho |
|----------|---------|
| Seam TX | `persist_identity_and_enqueue_graph_projection_tx` em `modules/agents/adapters/pg_registry.rs` (caller faz commit ou rollback) |
| Teste PG | `pg_agent_identity_graph_outbox_transaction_rollback_on_injected_failure` |
| Manifesto PG | `scripts/run-pg-integration-tests.sh` (+1 caso proposto; contagem atual em [test-matrix](../reference/test-matrix.md)) |

### Comportamento proposto (não comprovado em `origin/main`)

1. Na mesma transação PG: upsert `agent_identities`, append `agent_identity_events`, enqueue `graph_projection_outbox`.
2. **Falha injetada:** `ROLLBACK` após enqueue (sem commit) → zero linhas em domínio, audit e outbox para o fixture.
3. Sem trading live.

## Fatia 3 — W0-09 idempotência + outbox (orders)

> **Estado em `origin/main` (`d42b71a5`): não entregue.** O seam `persist_idempotency_and_enqueue_graph_projection_tx` e o teste não existem no código; a fatia nunca chegou a `main`. W0-09 volta pelo G1 em [wave0-09-v18-verificacao-sdd](./wave0-09-v18-verificacao-sdd.md). A tabela abaixo descreve o desenho proposto, não uma entrega.

| Artefato | Caminho |
|----------|---------|
| Seam TX | `persist_idempotency_and_enqueue_graph_projection_tx` em `modules/orders/adapters/pg_idempotency.rs` |
| Teste PG | `pg_order_idempotency_graph_outbox_transaction_rollback_on_injected_failure` |
| Manifesto PG | `scripts/run-pg-integration-tests.sh` (+1 caso proposto; contagem atual em [test-matrix](../reference/test-matrix.md)) |

### Comportamento proposto (não comprovado em `origin/main`)

1. Na mesma transação PG: `INSERT order_idempotency_keys` + enqueue `graph_projection_outbox`.
2. **Falha injetada:** `ROLLBACK` após enqueue → zero linhas em idempotência e outbox para o fixture.

Fluxo proposto da fatia 2 (agents), ainda não implementado:

```mermaid
flowchart TD
  A[BEGIN] --> B[agent_identities UPSERT]
  B --> C[agent_identity_events INSERT]
  C --> D[graph_projection_outbox enqueue]
  D --> E{finalize}
  E -->|produção| F[COMMIT]
  E -->|W0-09 teste| G[ROLLBACK injetado]
  G --> H[(PG sem linhas do fixture)]
```

## Estado

G4 de T-15 permanece **parcial**: W0-09 domínio+outbox rollback **pendente** (fatias agents e orders ausentes em `origin/main` `d42b71a5`); evidência CI (W0-10) continua no [master-plan](../planning/master-plan.md). Fatia 1 fecha rollback/idempotência do caminho de candles do monitor.
