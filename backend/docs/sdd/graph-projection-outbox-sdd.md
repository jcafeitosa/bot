---
title: SDD — Outbox PG para projeção Neo4j (F2.1)
description: Fila durável graph_projection_outbox; enqueue pós-commit PG; drain stub MERGE idempotente
tags:
  - sdd
  - backend
  - neo4j
  - postgres
status: draft
---

# SDD — Outbox PG para projeção Neo4j (fatia F2.1)

- **Estado:** draft — F2.1 slice 1 + **F2.1.2** worker/health; PG SoT; Neo4j derivado.
- **Referências:** [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md), [agents-neo4j-projection-sdd](./agents-neo4j-projection-sdd.md), [bots-neo4j-projection-sdd](./bots-neo4j-projection-sdd.md), [orders-neo4j-projection-sdd](./orders-neo4j-projection-sdd.md).

## 1. Contexto

F1/F2/F3 projetam agents/bots/orders em Neo4j via `best_effort_*` após commit PG (ou runtime sem PG). Falha de grafo não reverte PG, mas eventos perdidos não são reprocessáveis.

## 2. Objetivo (F2.1)

| Seam | Comportamento |
|------|----------------|
| `graph_projection_outbox` | Tabela PG com payload JSON mínimo, `graph_domain`, `idempotency_key`, `status`. |
| `enqueue_graph_projection_outbox` | INSERT … ON CONFLICT atualiza payload e reabre `pending`. |
| `graph_projection_best_effort` | Enfileira se PG wired; se PG+Neo4j, chama `drain_graph_projection_outbox` (best-effort inline). Sem PG, MERGE direto no Neo4j (dev). |
| `drain_graph_projection_outbox(limit)` | `FOR UPDATE SKIP LOCKED`; MERGE via `GraphProjectionPort` composto; sucesso → `done`; erro → `retry` + `last_error`. |
| `GraphProjectionSync` | `{ postgres?, neo4j? }` passado pelos três entrypoints `best_effort_*`. |

**Timing transacional:** hooks HTTP/monitor executam **após** persist PG — enqueue **não** está na mesma transação que o domínio hoje. F2.1.2 pode aceitar `&mut Transaction`.

**Fora de escopo F2.1:** DLQ; métricas SLO; enqueue na mesma TX de domínio (F2.1.3+).

## 3. Schema

`graph_projection_outbox`: `id`, `graph_domain`, `event_kind`, `idempotency_key` (unique com domain), `payload` JSONB (`GraphProjectionPayload`), `status` ∈ `pending|processing|done|retry`, `attempt_count`, `last_error`, `created_at`, `processed_at` (TIMESTAMPTZ).

## 4. Fail-closed

Drain com Neo4j down: `ping` falha → linhas permanecem `pending`/`retry`. MERGE falha → `retry`.

## 5. Validação

`./scripts/verify-backend-gates.sh` (baseline **518** bin `bot`, registro local); `./scripts/run-pg-integration-tests.sh` inclui `pg_graph_projection_outbox_*` e `pg_order_idempotency_and_graph_projection_same_transaction` (§8); worker + health cobertos por testes em `graph_projection_outbox_worker.rs` e `routes/health.rs`.

## 6. F2.1.2 (*implemented*)

| Seam | Comportamento |
|------|----------------|
| `spawn_graph_projection_outbox_worker` | Ticker no bootstrap HTTP (`serve`) e monitor quando `DATABASE_URL` + Neo4j agents stack habilitados. |
| Config | `neo4j.graph_projection_outbox_drain_secs` em `system.toml` (default 30; `0` desliga); env `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS`, batch `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_BATCH`. |
| Health | `/healthz` inclui `graph_projection_outbox` (pending/retry/idade) e `status: degraded` com backlog; `/readyz` inalterado (fail-closed só em PG/Neo4j down). |

## 7. F2.1.3 (*partial* — CLI drain)

| Seam | Comportamento |
|------|----------------|
| `bot graph-projection drain --limit N` | Conecta PG (`DATABASE_URL` → `trading_bot`, migrações) + Neo4j agents stack; chama `drain_graph_projection_outbox`. Fail-closed com mensagem clara se PG ou Neo4j ausentes/indisponíveis. Imprime JSON `{ processed, succeeded, failed }`. Default `--limit` = batch worker (`graph_projection_outbox_drain_batch`, 32). |

Implementação: `core/database/graph_projection_cli.rs`; testes `graph_projection_cli_*`.

**Pendente F2.1.3+ (outros domínios):** enqueue na mesma TX fora do caminho orders (§8).

## 8. F2.1.3+ (*partial* — enqueue na mesma TX)

| Seam | Comportamento |
|------|----------------|
| `enqueue_graph_projection_outbox_tx` | INSERT outbox dentro de `sqlx::Transaction` existente (fail-closed → `OrdersError::StoreUnavailable` no caminho orders; `String` no caminho bots/agents PG). |
| `PgOrderIdempotencyStore::persist_idempotency_and_enqueue_graph_projection` | Uma TX: `INSERT order_idempotency_keys` + N enqueues; usado após submit HTTP bem-sucedido quando `try_claim` PG já reservou a chave. |
| `PgAgentIdentityStore::persist_identity_and_enqueue_graph_projection` | Uma TX: upsert identity + audit + enqueue hierarchy. |
| `PgBotCatalogStore::save_catalog` | Uma TX: replace `bot_catalog_entries` + N enqueues `bot_catalog`. |
| `PgBotCatalogStore::enqueue_bot_promotion_graph_projection` / `enqueue_bot_demotion_graph_projection` | TX só outbox após promote/demote HTTP (runtime sem SoT PG). |
| `PgOrderIdempotencyStore::enqueue_graph_projection_outbox_messages` | TX só outbox após submit monitor supervisor (sem SoT idempotência PG no loop TUI; reconciliação opcional via mirror assíncrono). |
| `project_order_intent_after_submit` | Monitor supervisor pós-`submit_order` OK no ramo testnet (inalcançável no binário: `--mode testnet` rejeitado na validação): PG → enqueue TX dedicada + `graph_projection_drain_best_effort`; sem PG → `graph_projection_best_effort` (Neo4j direto). |
| `graph_projection_drain_best_effort` | Drain inline pós-commit (Neo4j wired), espelhando `graph_projection_best_effort`. |

Evidência PG: `pg_order_idempotency_and_graph_projection_same_transaction`, `pg_monitor_supervisor_graph_projection_outbox_same_transaction`, `pg_agent_identity_and_graph_projection_same_transaction`, `pg_bot_catalog_and_graph_projection_same_transaction` (manifesto `run-pg-integration-tests.sh`, **28** testes).

**Pendente F2.1.3+:** nenhum (fatia monitor supervisor fechada); demais domínios fora orders/agents/bots/monitor seguem roadmap.
