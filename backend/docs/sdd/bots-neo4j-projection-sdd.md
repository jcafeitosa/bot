---
title: SDD — Projeção Neo4j catálogo e promoção de bots (F2)
description: MERGE idempotente Bot/Strategy/IMPLEMENTS e PROMOTED_BY após PG/runtime; graph_domain=bots; best-effort
tags:
  - sdd
  - backend
  - bots
  - neo4j
status: draft
---

# SDD — Projeção Neo4j bots (fatia F2)

- **Estado:** draft — implementação F2; PG/catálogo e runtime permanecem SoT; Neo4j espelho write-only best-effort.
- **Referências:** [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md) §10, [agents-neo4j-projection-sdd](./agents-neo4j-projection-sdd.md), [bots-catalog-persistence-gate1-sdd](./bots-catalog-persistence-gate1-sdd.md).

## 1. Contexto

F1 projeta hierarquia de agents (`SUPERVISES`). Catálogo de bots persiste em `bot_catalog_entries` (Gate 1) e promoção runtime via `BotRuntimePort` / HTTP promote não alteram SoT do grafo.

## 2. Objetivo

| Seam | Comportamento |
|------|----------------|
| `GraphProjectionPort::project_bot_catalog_entry` | MERGE `:Bot` + `:Strategy` + `IMPLEMENTS`; propriedades redigidas (ids, timeframe, symbol, operation_mode). |
| `GraphProjectionPort::project_bot_promotion` | Atualiza estado de promoção no `:Bot`; aresta `PROMOTED_BY` de `:Agent` quando `BOT_HTTP_AGENCY_ID` está definido no promote HTTP. |
| `Neo4jBotProjector` | Cypher via `Neo4jGraph`; sem `neo4rs` fora de `core::database`. |
| `best_effort_project_bot_catalog` | Após `persist_bot_catalog` (store PG/memória OK). |
| `best_effort_project_bot_promotion` / `best_effort_retract_bot_promotion` | Após promote/demote runtime HTTP OK. |

**Fora de escopo F2:** `OrderIntent`; leitura autorizativa HTTP; outbox durável (**F2.1** — ver §7).

## 3. Modelo de grafo (F2)

- `:Bot` chave `bot_id`; `graph_domain = "bots"`.
- `:Strategy` chave `strategy_id` + `version`; `IMPLEMENTS` do bot para strategy.
- `PROMOTED_BY` de `:Agent` (mesma chave composta F1) para `:Bot` quando agência bound.
- Idempotência: MERGE nós; promoção remove arestas `PROMOTED_BY` entrantes no bot antes de recriar.

## 4. Configuração

Mesmo stack F0/F1: `BOT_AGENTS_ENABLED`, `BOT_NEO4J_*` (`load_agents_stack_from_env`).

## 5. Validação

- `./scripts/verify-backend-gates.sh`
- Unit: mock `GraphProjectionPort`; mapeamento `BotDefinition` → `BotCatalogProjection`.
- Integração: `neo4j_bot_promoted_by_after_catalog_and_promotion_projection` (skip sem compose `graph`).

## 6. Rollback

Desligar `BOT_AGENTS_ENABLED` ou remover hooks; PG/runtime intactos.

## 7. F2.1 (pendente)

Tabela outbox + worker de drain com retry; projeção `OrderIntent` redigido (F2 completo na strategy).
