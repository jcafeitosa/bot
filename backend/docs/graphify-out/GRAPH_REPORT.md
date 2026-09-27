# Graph Report - docs  (2026-09-27)

## Corpus Check
- 61 files · ~59,737 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 669 nodes · 692 edges · 34 communities (32 shown, 2 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Graphify Core Pipeline
- Backend Architecture
- Incremental Graph Updates
- Graphify Agent Integration
- Graph Export Formats
- Testing and Acceptance
- Agent Capability Research
- CLI and Monitor Contracts
- Backend Design Specifications
- Persistence Policy
- Work Planning and Limits
- Pause Resume Design
- Redirect Security
- Legacy Cleanup
- Community 14
- Community 15
- Community 16
- Community 17
- Community 18
- Community 19
- Community 20
- Community 21
- Community 22
- Community 23
- Community 24
- Community 25
- Community 26
- Community 27
- Community 28
- Community 29
- Community 30
- Community 31
- Community 32
- Community 33

## God Nodes (most connected - your core abstractions)
1. `SKILL` - 74 edges
2. `update` - 35 edges
3. `unimplemented-modules-analysis` - 34 edges
4. `modules-mvc-convention-sdd` - 32 edges
5. `module-catalog` - 28 edges
6. `0001-backend-core-modules-mvc` - 23 edges
7. `agents-module-sdd` - 23 edges
8. `index` - 21 edges
9. `current-state-and-roadmap` - 21 edges
10. `modules-completeness-audit` - 20 edges

## Surprising Connections (you probably didn't know these)
- `index` --references--> `AGENTS`  [EXTRACTED]
  index.md → AGENTS.md
- `index` --references--> `backend-module-reference`  [EXTRACTED]
  index.md → architecture/backend-module-reference.md
- `0001-backend-core-modules-mvc` --references--> `backend-module-reference`  [EXTRACTED]
  proposals/0001-backend-core-modules-mvc.md → architecture/backend-module-reference.md
- `index` --references--> `integrations`  [EXTRACTED]
  index.md → architecture/integrations.md
- `current-state-and-roadmap` --references--> `integrations`  [EXTRACTED]
  planning/current-state-and-roadmap.md → architecture/integrations.md

## Communities (34 total, 2 thin omitted)

### Community 0 - "Graphify Core Pipeline"
Cohesion: 0.03
Nodes (75): 1. uv tool installs — most reliable on modern Mac/Linux, 2. Read shebang from graphify binary (pipx and direct pip installs), 3. Fall back to python3, a detected file whose chunk failed or was omitted must stay unstamped so the, a stale semantic_hash from a prior run; clear it so detect_incremental re-queues, Always (re)write the cache file: write hits, else DELETE any leftover from a prior, base so the full build and incremental --update never drift apart on re-extract., by the AST pass (Part A); flattening every category here makes subagents re-read (+67 more)

### Community 1 - "Backend Architecture"
Cohesion: 0.04
Nodes (48): DATABASE_URL (market / migrations), postgres-and-graph-dev, Neo4j (opcional), PostgreSQL 18+ e Neo4j — desenvolvimento, Provider credentials (LLM API keys), backend-work-plan, Escopo e gates, Plano de execução das correções pendentes do backend (+40 more)

### Community 2 - "Incremental Graph Updates"
Cohesion: 0.05
Nodes (41): Alternatives, Configuração e persistência, Critério de escolha: reorganizar ou manter, Design, Destino e seams de tipos, Direção e admissão observável do core, 0001-backend-core-modules-mvc, Drawbacks e riscos (+33 more)

### Community 3 - "Graphify Agent Integration"
Cohesion: 0.05
Nodes (40): Alternativas consideradas, Contexto e problema, Contratos e seams para acordo do usuário antes dos testes, Dados, segurança e observabilidade, Design, monitor-pause-resume-sdd, Entregas, TDD e validação, Estado e transições (+32 more)

### Community 4 - "Graph Export Formats"
Cohesion: 0.06
Nodes (38): 1. Contexto e objetivo, 2. Convenção MVC, 3. Seams públicos, 4. Regras de domínio (IdentityOnly), 5. Alternativas, 6. Riscos e validação, 7. Referências, Critérios de fechamento G1 (checklist) (+30 more)

### Community 5 - "Testing and Acceptance"
Cohesion: 0.06
Nodes (36): are dropped rather than masquerading as deletions; untouched rows preserved (#1908)., as the freshly merged nodes and would DELETE the re-extracted content (#1178 is moot, cached files instead of missing every one after a move (#1417)., Changed semantic files dispatched this run but NOT stamped had their chunk fail, (cli._stamped_manifest_files + clear_semantic + scan_corpus)., directed=IS_DIRECTED: replace IS_DIRECTED with True if --directed was given, else, Do NOT add `changed` here: with root= passed, prune_set relativizes to the same base, update (+28 more)

### Community 6 - "Agent Capability Research"
Cohesion: 0.06
Nodes (36): Auditoria de completude — bots, orders, agents e HTTP, Checklist do objetivo, Decisões fora do código (bloqueiam fechamento do goal), modules-completeness-audit, Documentação relacionada, Fechamento do goal (pendente), Matriz de requisitos (objetivo), Pacote para revisão Critic (handoff) (+28 more)

### Community 7 - "CLI and Monitor Contracts"
Cohesion: 0.06
Nodes (34): AGENTS, graphify, `core/`, Critério de linha, module-implementation-status, Gates (G4), Lacunas conhecidas (não bloqueiam o objetivo literal), `modules/` (+26 more)

### Community 8 - "Backend Design Specifications"
Cohesion: 0.06
Nodes (34): Arquitetura do backend, Camadas e módulos, Cobertura de testes, backend-module-reference, Fluxo do backtest, Fluxo do monitor, Interfaces e seams importantes, Limites conhecidos (+26 more)

### Community 9 - "Persistence Policy"
Cohesion: 0.07
Nodes (30): 10. Referências, 1. Contexto e objetivo, 2.1 Mapeamento de camadas, 2.2 `core/` — MVC ou só infra?, 2.3 `presentation/terminal` — só View?, 2.4 Contratos neutros entre módulos, 2. Convenção MVC Rust (definição normativa), 3.1 `modules/market` (+22 more)

### Community 10 - "Work Planning and Limits"
Cohesion: 0.08
Nodes (26): Comandos, Configuração em camadas, Contratos de segurança, cli-and-config, Opções do monitor, Persistência, PG orders retention (Gate 2), Referência de CLI e configuração (+18 more)

### Community 11 - "Pause Resume Design"
Cohesion: 0.08
Nodes (26): Bot runtime no `serve` vs testes HTTP (G2 parcial), Critério de atualização, test-matrix, equivale a:, Lacunas explícitas, Matriz de testes do backend, Rotas mutantes com `BOT_HTTP_ADMIN_TOKEN`, Testes de integração (+18 more)

### Community 12 - "Redirect Security"
Cohesion: 0.10
Nodes (21): Binance REST, Binance WebSocket, ccxt vendorizado, integrations, Integrações do backend, Mapa de integrações, Matriz de risco de integração, PostgreSQL (+13 more)

### Community 13 - "Legacy Cleanup"
Cohesion: 0.11
Nodes (18): Contexto, core-providers-jev-sdd, Env vars, Não-objetivos, Objetivo, SDD — `core::providers` e migração do Jev, Seams públicos, Contexto (+10 more)

### Community 14 - "Community 14"
Cohesion: 0.12
Nodes (17): 1. Contexto, 2. Objetivo, 3. Mapeamento coluna ↔ modelo, 4. Riscos, 5. Validação, 6. Rollback, 7. Próximo, agents-pg-registry-sdd (+9 more)

### Community 15 - "Community 15"
Cohesion: 0.12
Nodes (16): 1. Mapa de execução, 2. Inventário por camada, 3. Módulo `agents` (`src/modules/agents/`), 3b. Módulo `bots` (`src/modules/bots/`), 3c. Módulo `orders` (`src/modules/orders/`), 3d. Facade `http_bridge` (`src/modules/http_bridge/`), 3e. Camada `presentation::http`, 4. Módulos de exchanges (`src/modules/exchanges/`) (+8 more)

### Community 16 - "Community 16"
Cohesion: 0.14
Nodes (14): Alternativas e decisões, Contexto e problema, Desenho, backtest-trades-and-slippage-sdd, Evidência de C12 (G3 aprovado por /root/c12_critic), Evidência de C13 (G3 aprovado com follow-up documental por /root/c13_critic), Fixture em barras agregadas, Objetivos e critérios de aceite (+6 more)

### Community 17 - "Community 17"
Cohesion: 0.18
Nodes (11): query, Find best matching node, Find best-matching start nodes, For /graphify explain, For /graphify path, graphify reference: query, path, explain, or: graphify query "QUESTION" --dfs --budget 3000, Score each node by term overlap for ranked output (+3 more)

### Community 18 - "Community 18"
Cohesion: 0.20
Nodes (10): Artefatos verificados, Auditoria completa de `docs/`, Auditoria do backend (2026-09-27), Correções aplicadas nesta auditoria, Critério de saúde, docs-audit, Duplicidade física (resolvida), Estrutura canônica (+2 more)

### Community 19 - "Community 19"
Cohesion: 0.20
Nodes (10): Contexto e limites, Contratos que o código atual procura implementar, backend-corrections-sdd, Entregas possíveis após revisão, Não objetivos, Objetivos propostos, SDD — Correções de configuração, mercado e organização do backend, Segurança e limites conhecidos (+2 more)

### Community 20 - "Community 20"
Cohesion: 0.20
Nodes (10): Contexto, core-completeness-sdd, Escopo — o que entra no `core`, Matriz de dependência, Não-objetivos, Objetivo, Rollback, SDD — Completude transversal do `core` (+2 more)

### Community 21 - "Community 21"
Cohesion: 0.20
Nodes (10): Alternativas e decisão, Contexto e objetivo, Design e seams propostos para acordo do usuário, rest-redirect-sdd, Entregas, TDD e verificação, Evidência de C10 — 2026-09-27, Evidência de C9 — 2026-09-27, Preflight T-17 (somente leitura) (+2 more)

### Community 22 - "Community 22"
Cohesion: 0.22
Nodes (9): exports, graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag) (+1 more)

### Community 23 - "Community 23"
Cohesion: 0.22
Nodes (9): Add --backend gemini|kimi|openai|deepseek|claude-cli depending on which API key you have set, Clone each repo, run the full pipeline on each, then merge, github-and-merge, graphify reference: GitHub clone and cross-repo merge, Run /graphify on each local path to produce their graph.json files, Step 0 - Clone GitHub repo(s) (only if a GitHub URL was given), Then merge:, Then merge at the project root: (+1 more)

### Community 24 - "Community 24"
Cohesion: 0.22
Nodes (9): Alternativas, Contexto e objetivo, legacy-file-cleanup-sdd, Evidência de uso e decisão por arquivo, Riscos e controles, Rollback e gate, SDD — Remoção de arquivos legados de configuração e migração, Seams públicos e comportamento preservado (+1 more)

### Community 25 - "Community 25"
Cohesion: 0.33
Nodes (6): Alternativas e decisão, Contexto e objetivo, backend-module-map-sdd, SDD T-16 — Mapa de módulos do backend no README, Superfície e desenho documental, Validação e gates

### Community 26 - "Community 26"
Cohesion: 0.33
Nodes (6): centralized-config-sdd, Removidos (cleanup), Árvore única `src/core/config/`, SDD: Configuração centralizada, Source of truth, Validação

### Community 27 - "Community 27"
Cohesion: 0.33
Nodes (6): Bootstrap, core-services-integration-sdd, Gaps, Matriz módulo × core, SDD — Integração de módulos com serviços do `core`, Validação

### Community 28 - "Community 28"
Cohesion: 0.33
Nodes (6): Addendum (2026-09-27 sessão 2), modules-relocation-phase4-5-sdd, Escopo, Fora de escopo (próximo turno), SDD — Relocação F4/F5/F6 parcial, Validação

### Community 29 - "Community 29"
Cohesion: 0.40
Nodes (5): transcribe, graphify reference: transcribe video and audio, print progress to stdout, which would otherwise corrupt the JSON file (#1392)., Step 2.5 - Transcribe video / audio files (only if video files detected), Write the JSON from Python (NOT a shell '>' redirect): transcribe_all/Whisper

### Community 30 - "Community 30"
Cohesion: 0.50
Nodes (4): add-watch, For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 31 - "Community 31"
Cohesion: 0.50
Nodes (4): hooks, For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration

## Knowledge Gaps
- **611 isolated node(s):** `/graphify`, `What graphify is for`, `What You Must Do When Invoked`, `Step 0 - GitHub repos and multi-path merge (only if a URL or several paths)`, `Step 1 - Ensure graphify is installed` (+606 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 611 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `module-catalog` connect `Community 15` to `Backend Architecture`, `Incremental Graph Updates`, `Graphify Agent Integration`, `Graph Export Formats`, `Agent Capability Research`, `CLI and Monitor Contracts`, `Backend Design Specifications`, `Persistence Policy`, `Work Planning and Limits`, `Pause Resume Design`, `Redirect Security`?**
  _High betweenness centrality (0.140) - this node is a cross-community bridge._
- **Why does `0001-backend-core-modules-mvc` connect `Incremental Graph Updates` to `Graphify Agent Integration`, `Graph Export Formats`, `CLI and Monitor Contracts`, `Backend Design Specifications`, `Persistence Policy`, `Redirect Security`, `Community 15`, `Community 20`?**
  _High betweenness centrality (0.107) - this node is a cross-community bridge._
- **Why does `agents-module-sdd` connect `Graph Export Formats` to `Backend Architecture`, `Incremental Graph Updates`, `Agent Capability Research`, `Backend Design Specifications`, `Work Planning and Limits`, `Pause Resume Design`, `Community 14`, `Community 15`?**
  _High betweenness centrality (0.087) - this node is a cross-community bridge._
- **What connects `/graphify`, `What graphify is for`, `What You Must Do When Invoked` to the rest of the system?**
  _611 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Graphify Core Pipeline` be split into smaller, more focused modules?**
  _Cohesion score 0.02666666666666667 - nodes in this community are weakly interconnected._
- **Should `Backend Architecture` be split into smaller, more focused modules?**
  _Cohesion score 0.0425531914893617 - nodes in this community are weakly interconnected._
- **Should `Incremental Graph Updates` be split into smaller, more focused modules?**
  _Cohesion score 0.05 - nodes in this community are weakly interconnected._