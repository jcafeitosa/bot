---
title: SDD — Projeção Neo4j OrderIntent redigido (F3 / G2 orders)
description: MERGE idempotente OrderIntent após submit HTTP OK e writes PG opcionais; graph_domain=trading; best-effort
tags:
  - sdd
  - backend
  - orders
  - neo4j
status: draft
---

# SDD — Projeção Neo4j OrderIntent (fatia F3)

- **Estado:** draft — implementação fatia mínima; PG/idempotência/reconciliação permanecem SoT; Neo4j espelho write-only best-effort.
- **Referências:** [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md) §6–7, [bots-neo4j-projection-sdd](./bots-neo4j-projection-sdd.md), [orders-module-sdd](./orders-module-sdd.md).

## 1. Contexto

Orders já validam `OrderIntent` via `modules/risk`, persistem idempotência/reconciliação em PG quando wired, e **nunca** dependem do grafo para aceitar submit (fail-closed de trading). F1/F2 projetam agents/bots; linhagem operacional de ordens fica como gap na strategy (§5).

## 2. Objetivo

| Seam | Comportamento |
|------|----------------|
| `GraphProjectionPort::project_order_intent` | MERGE `:OrderIntent` redigido; chave `client_order_id`. |
| `Neo4jOrderIntentProjector` | Cypher via `Neo4jGraph`; sem `neo4rs` fora de `core::database`. |
| `project_order_intent_after_submit` | Monitor supervisor pós-`submit_order` OK (PG outbox TX dedicada + drain inline quando wired)
| `best_effort_project_order_intent` | Após `ApiState::submit_order_http` OK (sem claim PG) ou fallback Neo4j após `submit_order` OK com `client_order_id`; opcional `submitting_bot_id` → `project_submitted_edge`. |
| `GraphProjectionPort::project_submitted_edge` | MERGE idempotente `(:Bot)-[:SUBMITTED]->(:OrderIntent)` quando `submitting_bot_id` presente (F3.1). |
| `OrderIntentProjection` | `client_order_id`, `symbol`, `side`, `status`, `execution_mode`, `submitted_at_ms`; **sem** `quote_amount`, credenciais, preços de conta ou payload exchange. |

**Fora de escopo (fatia F3):** leitura autorizativa HTTP; outbox durável (**F2.1**); memória semântica (F3 strategy §10 knowledge).

## 3. Modelo de grafo

- Label `:OrderIntent` chave `client_order_id`; `graph_domain = "trading"`.
- Propriedades: `symbol`, `side` (`buy`/`sell`), `status` (`submitted`), `execution_mode` (rótulo HTTP, ex. `dev_accept`, `paper`).
- Idempotência: MERGE nó; SET metadados redigidos.
- Aresta `SUBMITTED`: MERGE `Bot` stub + relação para `OrderIntent` existente (mesma chave `client_order_id`).

## 4. Configuração

Mesmo stack F0–F2: `BOT_AGENTS_ENABLED`, `BOT_NEO4J_*` (`load_agents_stack_from_env`).

## 5. Validação

- `./scripts/verify-backend-gates.sh`
- Unit: mock `GraphProjectionPort`; mapeamento submit → projection sem campos sensíveis.
- Integração: `neo4j_order_intent_after_redacted_projection`, `neo4j_submitted_edge_after_order_intent_projection` (skip sem compose `graph`).

## 6. Rollback

Desligar `BOT_AGENTS_ENABLED` ou remover hook; PG/orders intactos.

## 7. Próxima fatia

- Aresta `SUBMITTED` `:Monitor` sem `bot_id` (sessão opcional).
- Outbox PG → worker MERGE (F2.1).
