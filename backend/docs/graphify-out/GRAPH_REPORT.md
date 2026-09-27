# Graph Report - docs  (2026-09-26)

## Corpus Check
- 25 files · ~29,835 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 303 nodes · 285 edges · 22 communities (21 shown, 1 thin omitted)
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

## God Nodes (most connected - your core abstractions)
1. `SKILL` - 74 edges
2. `update` - 35 edges
3. `current-state-and-roadmap` - 17 edges
4. `backtest-trades-and-slippage-sdd` - 13 edges
5. `backend-module-reference` - 11 edges
6. `integrations` - 11 edges
7. `index` - 11 edges
8. `monitor-pause-resume-sdd` - 11 edges
9. `query` - 10 edges
10. `agents-capability-research` - 10 edges

## Surprising Connections (you probably didn't know these)
- `index` --references--> `AGENTS`  [EXTRACTED]
  index.md → AGENTS.md
- `index` --references--> `backend-module-reference`  [EXTRACTED]
  index.md → architecture/backend-module-reference.md
- `current-state-and-roadmap` --references--> `backend-module-reference`  [EXTRACTED]
  planning/current-state-and-roadmap.md → architecture/backend-module-reference.md
- `current-state-and-roadmap` --references--> `integrations`  [EXTRACTED]
  planning/current-state-and-roadmap.md → architecture/integrations.md
- `current-state-and-roadmap` --references--> `index`  [EXTRACTED]
  planning/current-state-and-roadmap.md → index.md

## Communities (22 total, 1 thin omitted)

### Community 0 - "Graphify Core Pipeline"
Cohesion: 0.03
Nodes (75): 1. uv tool installs — most reliable on modern Mac/Linux, 2. Read shebang from graphify binary (pipx and direct pip installs), 3. Fall back to python3, a detected file whose chunk failed or was omitted must stay unstamped so the, a stale semantic_hash from a prior run; clear it so detect_incremental re-queues, Always (re)write the cache file: write hits, else DELETE any leftover from a prior, base so the full build and incremental --update never drift apart on re-extract., by the AST pass (Part A); flattening every category here makes subagents re-read (+67 more)

### Community 1 - "Backend Architecture"
Cohesion: 0.06
Nodes (36): are dropped rather than masquerading as deletions; untouched rows preserved (#1908)., as the freshly merged nodes and would DELETE the re-extracted content (#1178 is moot, cached files instead of missing every one after a move (#1417)., Changed semantic files dispatched this run but NOT stamped had their chunk fail, (cli._stamped_manifest_files + clear_semantic + scan_corpus)., directed=IS_DIRECTED: replace IS_DIRECTED with True if --directed was given, else, Do NOT add `changed` here: with root= passed, prune_set relativizes to the same base, update (+28 more)

### Community 2 - "Incremental Graph Updates"
Cohesion: 0.09
Nodes (23): Diagnóstico rápido, runbook, Encerramento e recuperação, Escopo, Inicialização, Operação durante a execução, Pré-voo, Runbook operacional do backend (+15 more)

### Community 3 - "Graphify Agent Integration"
Cohesion: 0.10
Nodes (20): AGENTS, graphify, Binance REST, Binance WebSocket, ccxt vendorizado, integrations, Integrações do backend, Mapa de integrações (+12 more)

### Community 4 - "Graph Export Formats"
Cohesion: 0.14
Nodes (14): Alternativas e decisões, Contexto e problema, Desenho, backtest-trades-and-slippage-sdd, Evidência de C12 (G3 aprovado por /root/c12_critic), Evidência de C13 (G3 aprovado com follow-up documental por /root/c13_critic), Fixture em barras agregadas, Objetivos e critérios de aceite (+6 more)

### Community 5 - "Testing and Acceptance"
Cohesion: 0.17
Nodes (12): Alternativas consideradas, Contexto e problema, Contratos e seams para acordo do usuário antes dos testes, Dados, segurança e observabilidade, Design, monitor-pause-resume-sdd, Entregas, TDD e validação, Estado e transições (+4 more)

### Community 6 - "Agent Capability Research"
Cohesion: 0.18
Nodes (11): query, Find best matching node, Find best-matching start nodes, For /graphify explain, For /graphify path, graphify reference: query, path, explain, or: graphify query "QUESTION" --dfs --budget 3000, Score each node by term overlap for ranked output (+3 more)

### Community 7 - "CLI and Monitor Contracts"
Cohesion: 0.18
Nodes (11): Bloqueios e decisões pendentes, Capacidades recomendadas por fase, Comparação de capacidades e decisão, agents-capability-research, Etapa 1 — identidade `IdentityOnly`, Fases posteriores, com SDD e gates próprios, Fontes primárias, Limites e método (+3 more)

### Community 8 - "Backend Design Specifications"
Cohesion: 0.18
Nodes (11): Alternativas e decisão, Contexto, objetivo e limites, Contratos e seams propostos para acordo do usuário antes de testes, Desenho, monitor-persistence-policy-sdd, Escritas, retries e reconciliação, Inicialização e autoridade do estado, Plano em CLs, TDD e gates (+3 more)

### Community 9 - "Persistence Policy"
Cohesion: 0.20
Nodes (10): Arquitetura do backend, Cobertura de testes, backend-module-reference, Fluxo do backtest, Fluxo do monitor, Interfaces e seams importantes, Limites conhecidos, Módulos de raiz (+2 more)

### Community 10 - "Work Planning and Limits"
Cohesion: 0.20
Nodes (10): Contexto e limites, Contratos que o código atual procura implementar, backend-corrections-sdd, Entregas possíveis após revisão, Não objetivos, Objetivos propostos, SDD — Correções de configuração, mercado e organização do backend, Segurança e limites conhecidos (+2 more)

### Community 11 - "Pause Resume Design"
Cohesion: 0.22
Nodes (9): exports, graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag) (+1 more)

### Community 12 - "Redirect Security"
Cohesion: 0.22
Nodes (9): Add --backend gemini|kimi|openai|deepseek|claude-cli depending on which API key you have set, Clone each repo, run the full pipeline on each, then merge, github-and-merge, graphify reference: GitHub clone and cross-repo merge, Run /graphify on each local path to produce their graph.json files, Step 0 - Clone GitHub repo(s) (only if a GitHub URL was given), Then merge:, Then merge at the project root: (+1 more)

### Community 13 - "Legacy Cleanup"
Cohesion: 0.22
Nodes (9): Alternativas, Contexto e objetivo, legacy-file-cleanup-sdd, Evidência de uso e decisão por arquivo, Riscos e controles, Rollback e gate, SDD — Remoção de arquivos legados de configuração e migração, Seams públicos e comportamento preservado (+1 more)

### Community 14 - "Community 14"
Cohesion: 0.22
Nodes (9): Alternativas e decisão, Contexto e objetivo, Design e seams propostos para acordo do usuário, rest-redirect-sdd, Entregas, TDD e verificação, Evidência de C9 — 2026-09-27, Preflight T-17 (somente leitura), Riscos, operação e reversão (+1 more)

### Community 15 - "Community 15"
Cohesion: 0.25
Nodes (8): Comandos, Contratos de segurança, cli-and-config, Opções do monitor, Persistência, Referência de CLI e configuração, Timeframes por operação, Variáveis de ambiente

### Community 16 - "Community 16"
Cohesion: 0.33
Nodes (6): Alternativas e decisão, Contexto e objetivo, backend-module-map-sdd, SDD T-16 — Mapa de módulos do backend no README, Superfície e desenho documental, Validação e gates

### Community 17 - "Community 17"
Cohesion: 0.40
Nodes (5): transcribe, graphify reference: transcribe video and audio, print progress to stdout, which would otherwise corrupt the JSON file (#1392)., Step 2.5 - Transcribe video / audio files (only if video files detected), Write the JSON from Python (NOT a shell '>' redirect): transcribe_all/Whisper

### Community 18 - "Community 18"
Cohesion: 0.40
Nodes (5): backend-work-plan, Escopo e gates, Plano de execução das correções pendentes do backend, Próximas ações do Orquestrador, Verificação e limites

### Community 19 - "Community 19"
Cohesion: 0.50
Nodes (4): add-watch, For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 20 - "Community 20"
Cohesion: 0.50
Nodes (4): hooks, For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration

## Knowledge Gaps
- **279 isolated node(s):** `/graphify`, `What graphify is for`, `What You Must Do When Invoked`, `Step 0 - GitHub repos and multi-path merge (only if a URL or several paths)`, `Step 1 - Ensure graphify is installed` (+274 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 279 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `current-state-and-roadmap` connect `Incremental Graph Updates` to `Persistence Policy`, `Graphify Agent Integration`?**
  _High betweenness centrality (0.020) - this node is a cross-community bridge._
- **What connects `/graphify`, `What graphify is for`, `What You Must Do When Invoked` to the rest of the system?**
  _279 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Graphify Core Pipeline` be split into smaller, more focused modules?**
  _Cohesion score 0.02666666666666667 - nodes in this community are weakly interconnected._
- **Should `Backend Architecture` be split into smaller, more focused modules?**
  _Cohesion score 0.05555555555555555 - nodes in this community are weakly interconnected._
- **Should `Incremental Graph Updates` be split into smaller, more focused modules?**
  _Cohesion score 0.08695652173913043 - nodes in this community are weakly interconnected._
- **Should `Graphify Agent Integration` be split into smaller, more focused modules?**
  _Cohesion score 0.1 - nodes in this community are weakly interconnected._
- **Should `Graph Export Formats` be split into smaller, more focused modules?**
  _Cohesion score 0.14285714285714285 - nodes in this community are weakly interconnected._