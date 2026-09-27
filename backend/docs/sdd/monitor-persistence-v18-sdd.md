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

## Estado

G4 de T-15 permanece **parcial**: W0-09 amplo (erro injetado em TX domínio+outbox) continua no [master-plan](../planning/master-plan.md); esta fatia fecha rollback/idempotência do caminho de candles do monitor.
