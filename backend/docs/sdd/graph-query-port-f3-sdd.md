---
title: SDD — GraphQueryPort read-only (F3)
description: Leitura limitada do subgrafo agents no Neo4j; fail-closed sem stack; não substitui PG como SoT
tags:
  - sdd
  - neo4j
  - graph
  - agents
---

# SDD — GraphQueryPort (F3 read-only)

## Contexto

Projeção **write-only** F1–F3.1 alimenta o Neo4j a partir de PG/domínio ([unified-neo4j-graph-strategy](../architecture/unified-neo4j-graph-strategy.md)). F3 introduz **leitura operacional** mínima via seam `core::database::graph_query`, sem autoridade de runtime sobre ordens ou identidade (PG permanece SoT).

## Seams públicos

| API | Saída | Fail-closed |
|-----|-------|-------------|
| `GraphQueryPort::list_agents(limit)` | `{ agents[] }` com `agent_id`, `agency_id`, `role`, `lifecycle` | Sem `BOT_AGENTS_ENABLED` + Neo4j → CLI/erro driver |
| `GraphQueryPort::supervision_chain(agency_id, agent_id)` | `{ agency_id, agent_id, chain[] }` com `depth`, `kind`, `id` | IDs vazios → `Invalid`; agente ausente no grafo → `Invalid` |

Implementação: `Neo4jGraphQuery` em `graph_query.rs`. CLI: `bot graph query agents --limit N` e `bot graph query supervision-chain --agency-id … --agent-id …` ([cli-and-config](../reference/cli-and-config.md)).

## Validação

| Tipo | Evidência |
|------|-----------|
| Unit | `graph_query_port_list_agents_returns_projected_nodes`, `graph_query_port_supervision_chain_returns_ordered_nodes`, `supervision_chain_rejects_empty_ids` |
| CLI | `graph_cli_parses_query_agents_with_limit`, `graph_cli_parses_query_supervision_chain` |
| Neo4j (skip sem stack) | `neo4j_list_agents_after_local_graph`; `neo4j_agent_supervision_chain_after_projection` chama `graph_query().supervision_chain` |
| Gate | `./scripts/verify-backend-gates.sh` |

## Critérios de fechamento (fatia F3 parcial)

| Item | Status |
|------|--------|
| Port + Neo4j impl + testes unitários | **Sim** |
| CLI agents + supervision-chain | **Sim** |
| Leitura autorizativa no runtime HTTP | **Não** (roadmap) |
