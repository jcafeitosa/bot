---
title: SDD — Projeção Neo4j hierarquia de agents (F1)
description: MERGE idempotente Agent/SUPERVISES após commit PG; graph_domain=agents; best-effort fail-open na projeção
tags:
  - sdd
  - backend
  - agents
  - neo4j
status: draft
---

# SDD — Projeção Neo4j hierarquia de agents (fatia F1)

- **Estado:** draft — implementação F1; PG permanece SoT; Neo4j espelho write-only best-effort.
- **Referências:** [unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md) §14, [agents-pg-registry-sdd](./agents-pg-registry-sdd.md), [core-database-sdd](./core-database-sdd.md).

## 1. Contexto

`AgentRegistry` + `PgAgentIdentityStore` autorizam identidade em memória/PG. Neo4j já conecta via `BOT_AGENTS_ENABLED` + `BOT_NEO4J_*` para readiness, sem projeção de domínio (F0).

## 2. Objetivo

| Seam | Comportamento |
|------|----------------|
| `GraphProjectionPort` | `project_agent_hierarchy(AgentHierarchyProjection)` — contrato fino em `core::database`. |
| `Neo4jAgentHierarchyProjector` | MERGE Cypher via `Neo4jGraph`; sem `neo4rs` fora de `core::database`. |
| `best_effort_project_agent_definition` | Após PG OK em `persist_agent_after_mutation`; warn se Neo4j falhar; **não** falha HTTP/PG. |
| `AgentHierarchyProjection` | `agency_id`, `agent_id`, `role`, `lifecycle`, supervisor (`owner` \| `agent` + ids); **sem** `display_name`, capabilities, secrets. |

**Fora de escopo F1:** leitura autorizativa HTTP; nós `Bot`/`OrderIntent`; renomear `BOT_AGENTS_ENABLED`; outbox durável (F2).

## 3. Modelo de grafo (F1)

- Label `:Agent` com chave composta `agent_id` + `agency_id`.
- Label `:Owner` para supervisor humano (CEO → owner).
- Propriedade `graph_domain = "agents"` em nós projetados.
- Aresta `SUPERVISES` do supervisor (Owner ou Agent) para o agente mutado.
- Idempotência: `MERGE` nó agente; remover aresta `SUPERVISES` entrante antes de recriar.

## 4. Configuração

- Defaults: `src/core/config/system.toml` `[neo4j]` + `[agents].enabled`.
- Runtime: `BOT_AGENTS_ENABLED`, `BOT_NEO4J_*` via `load_agents_stack_from_env`.
- **Gap:** `BOT_AGENTS_ENABLED` ainda significa “Neo4j wired no processo”; renomeação fora de F1.

## 5. Validação

- `./scripts/verify-backend-gates.sh`
- Unit: mock `GraphProjectionPort`; mapeamento `AgentDefinition` → projection.
- Integração: `neo4j_agent_supervision_chain_after_projection` com compose `graph`.

## 6. Rollback

Desligar `BOT_AGENTS_ENABLED` ou remover hook; PG intacto.

## 7. F2 (pendente)

Outbox durável, projeção `Bot`/`PROMOTED_BY`, leitura consultiva.
