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
| `GraphQueryPort::bots_for_agent(agency_id, agent_id, limit)` | `{ agency_id, agent_id, bots[] }` com `bot_id`, `strategy_id`, `promotion_state`, `operation_mode` | IDs vazios → `Invalid`; agente ausente → `Invalid`; lista vazia se sem `PROMOTED_BY` |
| `GraphQueryPort::code_impact_for_module(module_path, limit)` | `{ module_path, module_id, entities[] }` com `source_id`, `source_file`, `entity_kind`, `link_kind` | `module_path` vazio → `Invalid`; sem Neo4j → fail-closed; lista vazia se subgrafo `code` ausente |

Implementação: `Neo4jGraphQuery` em `graph_query.rs`. CLI: `bot graph query agents --limit N`, `supervision-chain`, `bots-for-agent` e `code-impact --module-path …` ([cli-and-config](../reference/cli-and-config.md)).

## Validação

| Tipo | Evidência |
|------|-----------|
| Unit | `graph_query_port_list_agents_returns_projected_nodes`, `graph_query_port_supervision_chain_returns_ordered_nodes`, `graph_query_port_bots_for_agent_returns_projected_bots`, `graph_query_port_code_impact_for_module_returns_entities`, `normalize_module_path_*`, `supervision_chain_rejects_empty_ids` |
| CLI | `graph_cli_parses_query_agents_with_limit`, `graph_cli_parses_query_supervision_chain`, `graph_cli_parses_query_bots_for_agent`, `graph_cli_parses_query_code_impact` |
| Neo4j (skip sem stack) | `neo4j_list_agents_after_local_graph`; `neo4j_supervision_chain_query_after_projection`; `neo4j_bots_for_agent_after_catalog_and_promotion_projection`; `neo4j_code_impact_for_module_after_seed` |
| Gate | `./scripts/verify-backend-gates.sh` |

## Critérios de fechamento (fatia F3 parcial)

| Item | Status |
|------|--------|
| Port + Neo4j impl + testes unitários | **Sim** |
| CLI agents + supervision-chain + bots-for-agent | **Sim** |
| `bots_for_agent` port + Neo4j | **Sim** |
| Leitura HTTP admin read-only (agents list) | **Sim** (advisory; PG+Neo4j gated) |
| Leitura autorizativa no runtime HTTP (decisões de domínio) | **Não** (roadmap) |
| `code_impact_for_module` port + Neo4j + CLI | **Sim** |

## HTTP admin read-only (F3 fatia 1)

| Método | Path | Auth | Comportamento |
|--------|------|------|---------------|
| GET | `/api/v1/admin/graph/agents` | `BOT_HTTP_ADMIN_TOKEN` (Bearer), mesmo seam que provider-credentials | **200** corpo `ProjectedAgentList` (advisory; PG SoT); **503** `graph_query_unavailable` sem PostgreSQL ou sem Neo4j wired em `ApiState`; **401** sem bearer quando admin habilitado |

Query: `limit` opcional (1–500, default 32). Implementação: `ApiState::list_graph_agents_advisory` → `Neo4jGraph::graph_query().list_agents`.

| Tipo | Evidência |
|------|-----------|
| HTTP integration | `graph_admin_list_returns_503_without_postgres`, `graph_admin_list_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_list_requires_admin_bearer_when_enabled` em `http_integration_tests.rs` |
| Neo4j E2E HTTP **200** | skip sem stack (mesmo critério que `neo4j_list_agents_after_local_graph`) |

