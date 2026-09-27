# Graph Report - docs  (2026-09-26)

## Corpus Check
- 25 files · ~29,718 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 302 nodes · 1346 edges · 14 communities
- Extraction: 21% EXTRACTED · 79% INFERRED · 0% AMBIGUOUS · INFERRED: 1063 edges (avg confidence: 0.65)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 11
- Community 12
- Community 13

## God Nodes (most connected - your core abstractions)
1. `SKILL` - 74 edges
2. `Run /graphify on each local path to produce their graph.json files` - 57 edges
3. `Write merged result back to .graphify_extract.json so Step 4 sees the full graph` - 39 edges
4. `graphify reference: incremental update and cluster-only` - 36 edges
5. `update` - 35 edges
6. `graphify reference: extraction subagent prompt (compact)` - 34 edges
7. `graphify reference: GitHub clone and cross-repo merge` - 33 edges
8. `Load old graph (before update) from backup written before merge` - 29 edges
9. `graphify reference: commit hook and native CLAUDE.md integration` - 28 edges
10. `graphify reference: transcribe video and audio` - 27 edges

## Surprising Connections (you probably didn't know these)
- `Binance REST` --semantically_similar_to--> `SDD — Restringir redirects REST do monitor Binance`  [INFERRED] [semantically similar]
  architecture/integrations.md → sdd/rest-redirect-sdd.md
- `Binance WebSocket` --semantically_similar_to--> `SDD — Restringir redirects REST do monitor Binance`  [INFERRED] [semantically similar]
  architecture/integrations.md → sdd/rest-redirect-sdd.md
- `Escopo` --semantically_similar_to--> `Escopo e gates`  [INFERRED] [semantically similar]
  operations/runbook.md → planning/backend-work-plan.md
- `Pendências e bloqueios` --semantically_similar_to--> `Bloqueios e decisões pendentes`  [INFERRED] [semantically similar]
  planning/current-state-and-roadmap.md → research/agents-capability-research.md
- `root= mirrors the --update runbook (#1361): relativize source_file to the same` --semantically_similar_to--> `Runbook operacional do backend`  [INFERRED] [semantically similar]
  .codex/skills/graphify/SKILL.md → operations/runbook.md

## Communities (14 total, 0 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.11
Nodes (65): Clone each repo, run the full pipeline on each, then merge, github-and-merge, Step 0 - Clone GitHub repo(s) (only if a GitHub URL was given), Then merge:, Then merge at the project root:, Use LOCAL_PATH as the target for all subsequent steps, transcribe, print progress to stdout, which would otherwise corrupt the JSON file (#1392). (+57 more)

### Community 1 - "Community 1"
Cohesion: 0.08
Nodes (51): Arquitetura do backend, backend-module-reference, Fluxo do backtest, Fluxo do monitor, Módulos de raiz, Submódulos de exchanges, Visão do sistema, Binance REST (+43 more)

### Community 2 - "Community 2"
Cohesion: 0.23
Nodes (39): Step 2.5 - Transcribe video / audio files (only if video files detected), are dropped rather than masquerading as deletions; untouched rows preserved (#1908)., as the freshly merged nodes and would DELETE the re-extracted content (#1178 is moot, cached files instead of missing every one after a move (#1417)., Changed semantic files dispatched this run but NOT stamped had their chunk fail, (cli._stamped_manifest_files + clear_semantic + scan_corpus)., directed=IS_DIRECTED: replace IS_DIRECTED with True if --directed was given, else, Do NOT add `changed` here: with root= passed, prune_set relativizes to the same base (+31 more)

### Community 3 - "Community 3"
Cohesion: 0.62
Nodes (29): AGENTS, graphify, add-watch, For /graphify add, For --watch, graphify reference: add a URL and watch a folder, graphify reference: extra exports and benchmark, extraction-spec (+21 more)

### Community 4 - "Community 4"
Cohesion: 0.17
Nodes (19): exports, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag), Step 8 - Token reduction benchmark (only if total_words > 5000) (+11 more)

### Community 5 - "Community 5"
Cohesion: 0.35
Nodes (17): Cobertura de testes, Interfaces e seams importantes, Verificação antes de aceitar uma alteração, Contratos que o código atual procura implementar, backend-corrections-sdd, Entregas possíveis após revisão, Não objetivos, Objetivos propostos (+9 more)

### Community 6 - "Community 6"
Cohesion: 0.31
Nodes (16): Bloqueios e decisões pendentes, Capacidades recomendadas por fase, Comparação de capacidades e decisão, agents-capability-research, Etapa 1 — identidade `IdentityOnly`, Fontes primárias, Pesquisa de capacidades para o módulo `agents`, Recursos filtrados para a primeira etapa (+8 more)

### Community 7 - "Community 7"
Cohesion: 0.29
Nodes (12): P1 — Fechar contratos do monitor, Comandos, Contratos de segurança, cli-and-config, Opções do monitor, Persistência, Referência de CLI e configuração, Timeframes por operação (+4 more)

### Community 8 - "Community 8"
Cohesion: 0.20
Nodes (12): Contexto e objetivo, backend-module-map-sdd, Superfície e desenho documental, Contexto e problema, Desenho, backtest-trades-and-slippage-sdd, Evidência de C12 (G3 aprovado por /root/c12_critic), Evidência de C13 (G3 aprovado com follow-up documental por /root/c13_critic) (+4 more)

### Community 9 - "Community 9"
Cohesion: 0.36
Nodes (11): Escopo e gates, Gates de aceitação, Fases posteriores, com SDD e gates próprios, Validação e gates, Desenho, monitor-persistence-policy-sdd, Escritas, retries e reconciliação, Inicialização e autoridade do estado (+3 more)

### Community 10 - "Community 10"
Cohesion: 0.61
Nodes (9): Limites conhecidos, backend-work-plan, Próximas ações do Orquestrador, Verificação e limites, Limites e método, Contexto e limites, Segurança e limites conhecidos, Riscos, limites e custos (+1 more)

### Community 11 - "Community 11"
Cohesion: 0.25
Nodes (8): Contexto e problema, Dados, segurança e observabilidade, Design, monitor-pause-resume-sdd, Entregas, TDD e validação, Estado e transições, Pressão no canal e heartbeat, Questões abertas e gate

### Community 12 - "Community 12"
Cohesion: 0.29
Nodes (7): Verificação executada em 2026-09-27, Contexto e objetivo, rest-redirect-sdd, Entregas, TDD e verificação, Evidência de C9 — 2026-09-27, Preflight T-17 (somente leitura), Riscos, operação e reversão

### Community 13 - "Community 13"
Cohesion: 0.33
Nodes (7): Contexto e objetivo, legacy-file-cleanup-sdd, Riscos e controles, Rollback e gate, SDD — Remoção de arquivos legados de configuração e migração, Validação proporcional e plano de CL, Riscos e rollout/rollback

## Knowledge Gaps
- **44 isolated node(s):** `What You Must Do When Invoked`, `1. uv tool installs — most reliable on modern Mac/Linux`, `3. Fall back to python3`, `Placeholder questions - regenerated with real labels in Step 5`, `Step 5 - Label communities` (+39 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 44 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `graphify reference: commit hook and native CLAUDE.md integration` connect `Community 3` to `Community 1`, `Community 5`?**
  _High betweenness centrality (0.219) - this node is a cross-community bridge._
- **Why does `SKILL` connect `Community 0` to `Community 1`, `Community 2`, `Community 3`, `Community 4`?**
  _High betweenness centrality (0.206) - this node is a cross-community bridge._
- **Why does `Add --backend gemini|kimi|openai|deepseek|claude-cli depending on which API key you have set` connect `Community 1` to `Community 0`, `Community 3`?**
  _High betweenness centrality (0.184) - this node is a cross-community bridge._
- **Are the 56 inferred relationships involving `Run /graphify on each local path to produce their graph.json files` (e.g. with `For /graphify add` and `graphify reference: add a URL and watch a folder`) actually correct?**
  _`Run /graphify on each local path to produce their graph.json files` has 56 INFERRED edges - model-reasoned connections that need verification._
- **Are the 38 inferred relationships involving `Write merged result back to .graphify_extract.json so Step 4 sees the full graph` (e.g. with `For /graphify add` and `graphify reference: add a URL and watch a folder`) actually correct?**
  _`Write merged result back to .graphify_extract.json so Step 4 sees the full graph` has 38 INFERRED edges - model-reasoned connections that need verification._
- **Are the 35 inferred relationships involving `graphify reference: incremental update and cluster-only` (e.g. with `For /graphify add` and `graphify reference: add a URL and watch a folder`) actually correct?**
  _`graphify reference: incremental update and cluster-only` has 35 INFERRED edges - model-reasoned connections that need verification._
- **What connects `What You Must Do When Invoked`, `1. uv tool installs — most reliable on modern Mac/Linux`, `3. Fall back to python3` to the rest of the system?**
  _44 weakly-connected nodes found - possible documentation gaps or missing edges._