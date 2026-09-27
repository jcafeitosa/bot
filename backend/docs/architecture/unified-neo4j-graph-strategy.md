---
title: Estratégia de grafo unificado Neo4j
description: Visão, matriz módulo×grafo, SoT PG vs Neo4j, seams e fases para um único grafo operacional
tags:
  - architecture
  - backend
  - neo4j
  - graph
  - agents
status: draft
---

# Estratégia de grafo unificado (Neo4j)

> **G1 — análise e proposta.** Revisão independente pendente. Não autoriza CRUD massivo no runtime; define onde o grafo entra e como evitar dois grafos ou bypass de fail-closed.

## 1. Contexto e objetivo

O produto trata **governança** (owner → CEO → … → workers), **bots executores**, **código** e **operações** (orders, monitor, providers) como partes de um mesmo sistema cognitivo-operacional. A proposta de produto já prevê **banco de grafo** para conhecimento e relações ([pesquisa agents](../research/agents-capability-research.md) — fase “Conhecimento e memória”). Em paralelo, **graphify** materializa um grafo AST/docs em `backend/docs/graphify-out/` e pode **MERGE-push** no mesmo Neo4j local (`scripts/sync-code-graph-neo4j.sh`).

**Objetivo:** um **único cluster Neo4j** (instância lógica por ambiente) com namespaces de nós/arestas canônicos, alimentado por:

1. **Sync offline/CI** — código e documentação (graphify).
2. **Projeção eventual** — eventos e snapshots de domínio a partir de PostgreSQL (SoT transacional).
3. **Leitura consultiva** — travessias para impacto, governança e ferramentas de agente (sem autoridade de ordem).

**Não objetivos desta fatia:** live trading; substituir PG; expor segredos no grafo; segundo grafo (FalkorDB, `graph.json` como SoT de runtime).

## 2. Estado atual (F0) — inventário verificado

| Área | Uso do grafo hoje | SoT / evidência |
|------|-------------------|-----------------|
| `core::database::Neo4jGraph` | `connect`, `ping`, `node_count` | `src/core/database/neo4j.rs` |
| `AppDatabases::bootstrap_runtime` | Conecta Neo4j se `BOT_AGENTS_ENABLED=true` + credenciais | `bundle.rs`; config `load_agents_stack_from_env` |
| `core::health::readiness_databases` | Probe `neo4j` quando wired | `health/mod.rs` |
| `presentation/http` `/readyz` | Campo `neo4j: "ok"` se handle presente | `routes/health.rs` |
| `modules/agents` | **Nenhum** uso de Neo4j; registry + `PgAgentIdentityStore` | PG `agent_identities` / events |
| `modules/bots` | **Nenhum**; catálogo PG/memória | `PgBotCatalogStore` |
| `modules/orders` | **Nenhum**; idempotência/reconciliação PG | migrações `0004`/`0006` |
| `modules/monitor` | **Nenhum**; persistência candles PG opcional | `bootstrap_monitor_postgres` |
| `core::providers` | **Nenhum**; credenciais PG `provider_credentials` | migração `0007` |
| `modules/market` / `exchanges` | **Nenhum** | REST/WS apenas |
| Dev/ops | `graphify update` + `graphify export neo4j --push` | `sync-code-graph-neo4j.sh`; compose serviço `graph` |
| Agentes de documentação | `graphify-out/graph.json` local (não é o runtime HTTP) | skill graphify em `docs/.codex/skills/graphify` |

**Gap principal:** o runtime Rust **abre** Neo4j para readiness, mas **nenhum módulo de domínio** projeta ou consulta o grafo. O grafo “cheio” em dev vem quase só do **push graphify**, não do domínio bot.

### Nota sobre `agents_stack`

`AgentsStackConfig` / `load_agents_stack_from_env` **não é código morto**: acopla `BOT_AGENTS_ENABLED` à conexão Neo4j ([core-database-sdd](../sdd/core-database-sdd.md)). O nome sugere “stack de agentes”, mas hoje significa **“grafo habilitado para o processo”**, sem espelhar identidades. Risco de confusão operacional: ops pode assumir que agentes “vivem” no Neo4j quando vivem em memória + PG.

## 3. Visão — um grafo, namespaces canônicos

Um grafo por ambiente. Distinção por **labels**, **propriedades estáveis** e **`graph_domain`** (string) — não por bancos Neo4j separados.

### 3.1 Nós canônicos (evolução)

| Label | `graph_domain` | Chave estável | Origem | Notas |
|-------|----------------|---------------|--------|-------|
| `CodeEntity` | `code` | `source_id` (graphify) | graphify MERGE | Funções, módulos, docs; já no push |
| `Module` | `platform` | `module_id` (ex. `agents`, `orders`) | seed + graphify link | Ponte código ↔ domínio |
| `Agent` | `governance` | `agency_id` + `agent_id` | projeção PG | Papéis, lifecycle; **sem** prompts |
| `Owner` | `governance` | `owner_id` | projeção PG / config bind | Nó humano lógico |
| `Bot` | `execution` | `bot_id` canônico | PG catálogo + runtime | Distinto de `Agent` |
| `Strategy` | `execution` | `strategy@version` | catálogo / config | Liga bot ↔ código strategy |
| `OrderIntent` | `trading` | `client_order_id` / id interno | evento PG (redigido) | Sem preços de conta/secrets |
| `Provider` | `integrations` | `provider_id` | config + PG metadata | **Sem** `secret` |
| `OrgUnit` / `Position` | `org` | TBD | futuro [org-module-sdd](../sdd/org-module-sdd.md) | Após SDD org |

### 3.2 Arestas canônicas

| Tipo | De → Para | Significado |
|------|-----------|-------------|
| `SUPERVISES` | Owner/Agent → Agent | Hierarquia administrativa |
| `MEMBER_OF` | Agent → Agency | Isolamento por agência |
| `IMPLEMENTS` | Bot → Strategy | Executor versionado |
| `PROMOTED_BY` | Bot runtime → Agent | Gate `promote_runtime_bot` |
| `OWNS` | Agent → Bot | Responsabilidade (futuro, policy) |
| `SUBMITTED` | Bot/Monitor → OrderIntent | Linhagem operacional (evento) |
| `CALLS` / `IMPORTS` | CodeEntity → CodeEntity | graphify (já) |
| `DOCUMENTS` | CodeEntity → Module | sync manual ou regra graphify |
| `USES_PROVIDER` | Agent → Provider | `consult_jev` / capability flags |
| `AFFECTS` | CodeEntity → Module | impacto de mudança de código |

## 4. Matriz módulo × papel no grafo

Legenda: **R** read (travessia), **W** write (projeção), **S** sync batch (graphify/job), **E** evento (outbox/stream), **—** fora do grafo nesta fase.

| Módulo / camada | F0 (hoje) | F1 code + agents mirror | F2 runtime events | F3 knowledge |
|-----------------|-----------|-------------------------|-------------------|--------------|
| `core::database` | W (ping only) | W (driver + ports) | W | W |
| `core::health` | R (probe) | R | R | R |
| `modules/agents` | — | **W/E** espelho hierarquia | R (who supervises) | R (policy links) |
| `modules/bots` | — | **W** catálogo + `PROMOTED_BY` | **E** promote/demote | R |
| `modules/orders` | — | — | **E** `OrderIntent` redigido | — |
| `modules/monitor` | — | **S** link Module/Strategy | **E** sessão (opcional) | — |
| `modules/org` (futuro) | — | — | W | R |
| `core::providers` | — | **W** nós `Provider` | — | R |
| `modules/market` | — | — | — | — (séries em PG/Timescale) |
| `modules/exchanges` | — | — | — | — |
| `modules/portfolio` | — | — | E paper snapshot refs | — |
| `presentation/http` | R (`/readyz`) | R | — | — |
| `presentation/terminal` | — | — | — | — |
| Dev: graphify | **S** | **S** | **S** | **S** |
| CI (opcional) | — | **S** push em job dedicado | — | — |

## 5. PostgreSQL vs Neo4j — source of truth

| Domínio | SoT | Neo4j |
|---------|-----|-------|
| Identidade de agentes, eventos admin | PG `agent_identities`, `agent_identity_events` | Projeção read-only; reconstruível via snapshot |
| Catálogo de bots | PG `bot_catalog_entries` (+ memória) | Nós `Bot`/`Strategy`; eventual |
| Orders, idempotência, reconciliação | PG | Apenas metadados de linhagem (ids, status, modo); nunca payload exchange completo |
| Credenciais LLM | PG `provider_credentials` | Só `Provider` + flags; **proibido** `secret` |
| Candles, datasets | PG/Timescale | Não replicar séries no grafo |
| Código e docs | Git + `graphify-out` (artefato) | `CodeEntity` via MERGE; grafo JSON não é SoT |
| Hierarquia “quem pode promover bot” | PG + regras em código | Aresta `PROMOTED_BY` derivada |

**Regra:** escrita de negócio **sempre** PG (ou memória com espelho PG) primeiro; Neo4j **nunca** decide ordem, auth ou promoção sozinho.

## 6. Seams públicos propostos (`core`)

Evitar que cada módulo importe `neo4rs`. Concentrar em `core::database` ou submódulo `core::graph` (nome a fixar em SDD de implementação).

| Seam | Responsabilidade | Consumidores |
|------|------------------|--------------|
| `Neo4jGraph` (existente) | Conexão, `ping`, execução parametrizada interna | health, adapters graph |
| `GraphRuntimeConfig` | Renomear/evoluir `AgentsStackConfig`: `enabled`, URI, database; **desacoplar** semântica “agents” do flag de grafo (alias de migração: `load_graph_runtime_from_env`) | `AppDatabases` |
| `GraphProjectionPort` | `upsert_agent_subgraph`, `upsert_bot_subgraph`, `mark_order_intent` — idempotente MERGE | adapters em `modules/*/adapters/graph_*` |
| `GraphQueryPort` (read) | Travessias limitadas: `supervision_chain`, `bots_for_agent`, `code_impact_for_module` | advisory futuro, ops, HTTP read-only gated |
| `GraphSyncJob` (offline) | Invoca graphify push ou import Cypher; não no hot path HTTP | scripts, CI |

**Fail-closed:** se `enabled` e conexão falha → `neo4j: None` (como hoje), módulos **não** simulam grafo em memória. Features que **exigem** grafo (futuro) retornam erro explícito, não degradam silenciosamente para PG.

## 7. Fases de rollout

```mermaid
flowchart LR
  F0[F0 Conectividade] --> F1[F1 Code + Agents mirror]
  F1 --> F2[F2 Runtime events]
  F2 --> F3[F3 Knowledge memory]

  F0 --- G0[graphify push opcional]
  F1 --- G1[MERGE Agent SUPERVISES]
  F2 --- G2[OrderIntent Bot edges]
  F3 --- G3[Semantic memory SDD]
```

| Fase | Entregável | Validação |
|------|------------|-----------|
| **F0** (atual) | Bolt + readyz; graphify → Neo4j em dev | `ping_and_node_count_against_local_graph`; script sync |
| **F1** | Projeção `Agent`/`Owner`/`SUPERVISES` após mutação PG; labels `graph_domain`; link `Module` ↔ `CodeEntity` documentado | Teste integração: MERGE após `pg_agent_lifecycle_write_through`; query Cypher em CI opcional |
| **F2** | `Bot`/`Strategy`/`PROMOTED_BY`; eventos `OrderIntent` redigidos | Sem live prod; recording/testnet only |
| **F3** | Memória semântica, proveniência ([agents research](../research/agents-capability-research.md)) | SDD próprio + threat model |

**Rollback F1+:** desabilitar projeção (flag); truncar subgrafo `graph_domain='governance'` via job; PG intacto.

## 8. Anti-patterns

| Anti-pattern | Por quê evitar |
|--------------|----------------|
| Dois grafos (Neo4j + FalkorDB ou segunda instância “só agents”) | Divergência, custo ops, queries ambíguas |
| `graphify-out/graph.json` como SoT de runtime | Artefato local, stale, sem transação com PG |
| `BOT_AGENTS_ENABLED=false` mas push graphify em produção “só para docs” sem namespace | Mistura domínios sem contrato |
| Ler hierarquia do Neo4j para autorizar HTTP | SoT deve permanecer PG + código; grafo é derivado |
| Propriedades com API keys, `DATABASE_URL`, payloads de ordem | Vazamento em backups Browser/export |
| `neo4rs` direto em `modules::*` | Quebra import-direction e testabilidade |
| Reconciliação bidirecional PG ↔ Neo4j em tempo real | Complexidade; preferir outbox unidirecional |

## 9. Diagrama — fluxo de dados alvo (F1+)

```mermaid
flowchart TB
  subgraph sources [Fontes de verdade]
    PG[(PostgreSQL trading_bot)]
    Git[Git + graphify]
  end

  subgraph runtime [Runtime bot]
    HTTP[presentation/http]
    Bridge[http_bridge]
    Agents[modules/agents]
    Bots[modules/bots]
    DB[AppDatabases]
    GraphPort[GraphProjectionPort]
  end

  subgraph graph [Neo4j único]
    Code[CodeEntity code]
    Gov[Agent Owner governance]
    Exec[Bot Strategy execution]
  end

  Git -->|sync-code-graph-neo4j.sh MERGE| Code
  HTTP --> Bridge --> Agents
  Agents -->|write-through| PG
  Agents -->|event optional F1| GraphPort
  Bots --> PG
  GraphPort --> DB --> Gov
  Code -.->|DOCUMENTS AFFECTS| Exec
  Gov -->|SUPERVISES| Gov
  Exec -->|PROMOTED_BY| Gov
```

## 10. Integração com operações existentes

- Compose: serviço `graph` em `docker-compose.bot.yml` (Bolt `7688`, Browser `7475`).
- Env: [postgres-and-graph-dev.md](../operations/postgres-and-graph-dev.md) — `BOT_AGENTS_ENABLED`, `BOT_NEO4J_*`.
- SDDs relacionados: [core-database-sdd](../sdd/core-database-sdd.md), [database-module-integration-sdd](../sdd/database-module-integration-sdd.md), [agents-module-sdd](../sdd/agents-module-sdd.md), [agents-pg-registry-sdd](../sdd/agents-pg-registry-sdd.md).

## 11. Próximo passo implementável (fatia vertical recomendada)

**F1 — espelho de hierarquia de agentes (write-only, best-effort):**

1. SDD curto `agents-neo4j-projection-sdd.md` (proporcional) com Cypher MERGE e idempotência.
2. Adapter `modules/agents/adapters/graph_projection.rs` chamado de `persist_agent_after_mutation` / hydrate (após PG OK).
3. Teste integração: com Neo4j local, register CEO + worker → `MATCH (a:Agent)-[:SUPERVISES*]->(b:Agent)` conta esperada.
4. Sem rotas HTTP novas; sem leitura autorizativa.

Gate: `./scripts/verify-backend-gates.sh` se tocar código; doc-only não exige.

## 12. Referências

- [layer-mapping.md](./layer-mapping.md) — composition root e infra.
- [module-catalog.md](./module-catalog.md) — contratos por módulo.
- [integrations.md](./integrations.md) — PG e seams HTTP.
