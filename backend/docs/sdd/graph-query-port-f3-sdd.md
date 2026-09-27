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
| Leitura HTTP admin read-only (agents, supervision-chain, bots-for-agent, code-impact) | **Sim** (advisory; PG+Neo4j gated) |
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

| GET | `/api/v1/admin/graph/supervision-chain` | `BOT_HTTP_ADMIN_TOKEN` (Bearer) | **200** `ProjectedSupervisionChain`; query `agency_id`, `agent_id`; **400** `invalid_graph_query`; **503** `graph_query_unavailable` sem PG/Neo4j |
| GET | `/api/v1/admin/graph/bots-for-agent` | `BOT_HTTP_ADMIN_TOKEN` (Bearer) | **200** `ProjectedBotsForAgent`; query `agency_id`, `agent_id`, `limit` (1–500, default 32); **400** `invalid_graph_query`; **503** `graph_query_unavailable` sem PG/Neo4j |

Implementação adicional: `ApiState::graph_supervision_chain_advisory` / `graph_bots_for_agent_advisory` → `Neo4jGraph::graph_query()`.

| Tipo | Evidência (supervision-chain / bots-for-agent) |
|------|-----------|
| HTTP integration | `graph_admin_supervision_chain_returns_503_without_postgres`, `graph_admin_supervision_chain_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_supervision_chain_requires_admin_bearer_when_enabled`, `graph_admin_bots_for_agent_returns_503_without_postgres`, `graph_admin_bots_for_agent_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_bots_for_agent_requires_admin_bearer_when_enabled` |

| Neo4j E2E HTTP **200** | skip sem stack (mesmo critério que `neo4j_list_agents_after_local_graph`) |

| GET | `/api/v1/admin/graph/code-impact` | `BOT_HTTP_ADMIN_TOKEN` (Bearer) | **200** `ProjectedCodeImpactForModule`; query `module_path`, `limit` (1–500, default 32); **400** `invalid_graph_query`; **503** `graph_query_unavailable` sem PG/Neo4j |

Implementação: `ApiState::graph_code_impact_for_module_advisory` → `Neo4jGraph::graph_query().code_impact_for_module`.

**Dependência externa (code-impact):** o binário não escreve o subgrafo `code`. A consulta (`CODE_IMPACT_FOR_MODULE`, `core/database/graph_query.rs:65-80`) lê nós `:CodeEntity` com `graph_domain = 'code'`; no repositório só o teste `neo4j_code_impact_for_module_after_seed` cria esses nós (`graph_query.rs:742`). O caminho de ops é `scripts/sync-code-graph-neo4j.sh` (requer CLI `graphify` e `BOT_NEO4J_*`; sem `graphify` o script sai com erro). Sem esse push, com PostgreSQL e Neo4j ligados, CLI e HTTP respondem com `entities: []` (HTTP **200**), não com erro. Sem PG ou sem Neo4j o HTTP responde **503** `graph_query_unavailable` (`presentation/http/state.rs:954-969`); com `BOT_HTTP_ADMIN_TOKEN` configurado e bearer ausente ou errado, **401** `unauthorized` antes da consulta (`routes/graph_admin.rs:131` → `state.rs:800-801` → `admin_auth.rs:91-105`). Ressalva estática, não verificada com Neo4j rodando: o `graphify export neo4j` instalado (graphify 0.9.66, `exporters/graphdb.py:54-56`) cria nós rotulados por `file_type` e chaveados por `id`, sem `:CodeEntity`/`graph_domain`/`source_id` — o push do script pode não popular o que a consulta lê.

| Tipo | Evidência (code-impact) |
|------|-----------|
| HTTP integration | `graph_admin_code_impact_returns_503_without_postgres`, `graph_admin_code_impact_returns_503_without_neo4j_when_postgres_wired`, `graph_admin_code_impact_requires_admin_bearer_when_enabled` |

