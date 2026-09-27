# Graph Report - docs  (2026-09-27)

## Corpus Check
- 86 files · ~130,881 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1002 nodes · 1276 edges · 74 communities (72 shown, 2 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `35c45a93`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- SKILL
- unimplemented-modules-analysis
- core-extraction-phase1-sdd
- monitor-presentation-contract-sdd
- agents-module-sdd
- update
- modules-completeness-audit
- index
- current-state-and-roadmap
- modules-mvc-convention-sdd
- cli-and-config
- test-matrix
- 3. Avaliação por módulo
- core-providers-nim-sdd
- agents-pg-registry-sdd
- module-catalog
- backtest-trades-and-slippage-sdd
- query
- Threat model — módulo `org`
- backend-corrections-sdd
- core-completeness-sdd
- rest-redirect-sdd
- exports
- github-and-merge
- SDD — Módulo `org` para estrutura e governança de agentes
- Estratégia de grafo unificado (Neo4j)
- orders-live-execution-gate2-sdd
- core-services-integration-sdd
- Slices
- transcribe
- add-watch
- hooks
- extraction-spec
- monitor-persistence-policy-sdd
- 0001-backend-core-modules-mvc
- Análise do módulo `org` para o sistema de agentes
- backend-module-map-sdd
- agents-capability-research
- bots-runtime-live-gate2-sdd
- monitor-pause-resume-sdd
- integrations
- module-implementation-status
- bots-module-sdd
- market-module-phase3-sdd
- provider-credentials-db-sdd
- runbook
- SDD — Outbox PG para projeção Neo4j (fatia F2.1)
- persistence-agents-bots-scaffold-sdd
- SDD — Projeção Neo4j hierarquia de agents (fatia F1)
- SDD — Projeção Neo4j bots (fatia F2)
- core-database-sdd
- SDD — Projeção Neo4j OrderIntent (fatia F3)
- http-admin-auth-seam-sdd
- bots-catalog-persistence-gate1-sdd
- grok-bot-overview
- master-plan.md
- SDD — Product owner bootstrap (G1 fatia mínima)
- Threat model — orders Gate 2
- Threat model — autenticação do owner / IdP (P1)
- SDD W0-02 — CI verde + PG fail-loud + DB de teste isolado (stub)
- SDD W0-03 — monitor com persistência desligada não abre nem migra PG [SEGURANÇA]
- SDD W0-12 — resultado ambíguo de ordem não libera o claim [SEGURANÇA]
- legacy-file-cleanup-sdd
- SDD W0-13 — orders sem `block_on` dentro do runtime Tokio
- orders-module-sdd
- Achado F-ADM-01 — auth admin HTTP "fail-closed" é, na prática, opcional (fail-open)
- graph-query-port-f3-sdd.md
- monitor-persistence-c17-sdd.md
- Addendum V18 — persistência de mercado (T-15)
- backend-work-plan
- postgres-and-graph-dev
- Fatia 2 — TUI/presentation + REST do estado de arquivo (T-15 runtime)
- Achado F-CRED — credenciais de provider/exchange em texto claro

## God Nodes (most connected - your core abstractions)
1. `SKILL` - 74 edges
2. `index` - 56 edges
3. `unimplemented-modules-analysis` - 50 edges
4. `modules-mvc-convention-sdd` - 41 edges
5. `module-catalog` - 38 edges
6. `modules-completeness-audit` - 37 edges
7. `update` - 35 edges
8. `current-state-and-roadmap` - 35 edges
9. `agents-module-sdd` - 33 edges
10. `cli-and-config` - 32 edges

## Surprising Connections (you probably didn't know these)
- `module-implementation-status` --references--> `postgres-and-graph-dev`  [EXTRACTED]
  architecture/module-implementation-status.md → operations/postgres-and-graph-dev.md
- `index` --references--> `postgres-and-graph-dev`  [EXTRACTED]
  index.md → operations/postgres-and-graph-dev.md
- `backend-work-plan` --references--> `postgres-and-graph-dev`  [EXTRACTED]
  planning/backend-work-plan.md → operations/postgres-and-graph-dev.md
- `unimplemented-modules-analysis` --references--> `postgres-and-graph-dev`  [EXTRACTED]
  planning/unimplemented-modules-analysis.md → operations/postgres-and-graph-dev.md
- `cli-and-config` --references--> `postgres-and-graph-dev`  [EXTRACTED]
  reference/cli-and-config.md → operations/postgres-and-graph-dev.md

## Communities (74 total, 2 thin omitted)

### Community 0 - "SKILL"
Cohesion: 0.03
Nodes (75): 1. uv tool installs — most reliable on modern Mac/Linux, 2. Read shebang from graphify binary (pipx and direct pip installs), 3. Fall back to python3, a detected file whose chunk failed or was omitted must stay unstamped so the, a stale semantic_hash from a prior run; clear it so detect_incremental re-queues, Always (re)write the cache file: write hits, else DELETE any leftover from a prior, base so the full build and incremental --update never drift apart on re-extract., by the AST pass (Part A); flattening every category here makes subagents re-read (+67 more)

### Community 1 - "unimplemented-modules-analysis"
Cohesion: 0.05
Nodes (40): 1. Identidade persistente de agentes, 1b. Módulo `bots` (fundação — não confundir com agents), 2. Controle, autenticação e autorização, 3. Runtime, worker e scheduler, 4. Ferramentas, aprovações e sandbox, 5. Memória, conhecimento e canais, 6. Execução financeira e produção, 7. Persistência e integração ainda incompletas (+32 more)

### Community 2 - "core-extraction-phase1-sdd"
Cohesion: 0.12
Nodes (17): Alternativas consideradas, Contexto, Critérios de aceite, Decisões, core-extraction-phase1-sdd, Estratégia de imports, Matriz de dependência (fase 1), Não-objetivos (fora desta fatia) (+9 more)

### Community 3 - "monitor-presentation-contract-sdd"
Cohesion: 0.13
Nodes (15): Alternativas, Comandos e resultado de envio, Contexto, objetivos e limites, Decimal canônico, Decisão de ownership e tipos públicos, monitor-presentation-contract-sdd, Gates de CI e aviso inventory, Migração concreta da TUI (+7 more)

### Community 4 - "agents-module-sdd"
Cohesion: 0.15
Nodes (13): 1. Contexto e objetivo, 2. Convenção MVC, 3. Seams públicos, 4. Regras de domínio (IdentityOnly), 5. Alternativas, 6. Riscos e validação, 7. Referências, Critérios de fechamento G1 (checklist) (+5 more)

### Community 5 - "update"
Cohesion: 0.06
Nodes (36): are dropped rather than masquerading as deletions; untouched rows preserved (#1908)., as the freshly merged nodes and would DELETE the re-extracted content (#1178 is moot, cached files instead of missing every one after a move (#1417)., Changed semantic files dispatched this run but NOT stamped had their chunk fail, (cli._stamped_manifest_files + clear_semantic + scan_corpus)., directed=IS_DIRECTED: replace IS_DIRECTED with True if --directed was given, else, Do NOT add `changed` here: with root= passed, prune_set relativizes to the same base, update (+28 more)

### Community 6 - "modules-completeness-audit"
Cohesion: 0.07
Nodes (28): Artefatos verificados, Auditoria completa de `docs/`, Auditoria do backend (2026-09-27), Correções aplicadas nesta auditoria, Critério de saúde, docs-audit, Duplicidade física (resolvida), Estrutura canônica (+20 more)

### Community 7 - "index"
Cohesion: 0.11
Nodes (19): AGENTS, graphify, grok-bot-teams-and-enterprises, Grok Bot for teams and enterprises, Security model, meta-model-api-overview, openbot-readme, OpenBot (+11 more)

### Community 8 - "current-state-and-roadmap"
Cohesion: 0.13
Nodes (15): Com DATABASE_URL → trading_bot:, Designs e correções já registrados, current-state-and-roadmap, Estado atual e planejamento do backend, Evidência existente, Gates de aceitação, O que já está feito, P0 — Fechado (+7 more)

### Community 9 - "modules-mvc-convention-sdd"
Cohesion: 0.07
Nodes (30): 10. Referências, 1. Contexto e objetivo, 2.1 Mapeamento de camadas, 2.2 `core/` — MVC ou só infra?, 2.3 `presentation/terminal` — só View?, 2.4 Contratos neutros entre módulos, 2. Convenção MVC Rust (definição normativa), 3.1 `modules/market` (+22 more)

### Community 10 - "cli-and-config"
Cohesion: 0.17
Nodes (12): Comandos, Configuração em camadas, Contratos de segurança, cli-and-config, Opções do monitor, Persistência, PG orders retention (Gate 2), Referência de CLI e configuração (+4 more)

### Community 11 - "test-matrix"
Cohesion: 0.18
Nodes (11): Bot runtime no `serve` vs testes HTTP (G2 parcial), Critério de atualização, test-matrix, equivale a:, Lacunas explícitas, Matriz de testes do backend, Rotas mutantes com `BOT_HTTP_ADMIN_TOKEN`, Testes de integração (+3 more)

### Community 12 - "3. Avaliação por módulo"
Cohesion: 0.05
Nodes (39): 1. Inventário real do código, 2. Contradições doc×código relevantes, 3.10 orders, 3.11 bots, 3.12 agents, 3.13 presentation/http + http_bridge, 3.14 providers / jev / credentials, 3.15 notifications / canais (+31 more)

### Community 13 - "core-providers-nim-sdd"
Cohesion: 0.11
Nodes (18): Contexto, core-providers-jev-sdd, Env vars, Não-objetivos, Objetivo, SDD — `core::providers` e migração do Jev, Seams públicos, Contexto (+10 more)

### Community 14 - "agents-pg-registry-sdd"
Cohesion: 0.22
Nodes (9): 1. Contexto, 2. Objetivo, 3. Mapeamento coluna ↔ modelo, 4. Riscos, 5. Validação, 6. Rollback, 7. Próximo, agents-pg-registry-sdd (+1 more)

### Community 15 - "module-catalog"
Cohesion: 0.05
Nodes (37): Arquitetura do backend, Camadas e módulos, Cobertura de testes, backend-module-reference, Fluxo do backtest, Fluxo do monitor, Interfaces e seams importantes, Limites conhecidos (+29 more)

### Community 16 - "backtest-trades-and-slippage-sdd"
Cohesion: 0.14
Nodes (14): Alternativas e decisões, Contexto e problema, Desenho, backtest-trades-and-slippage-sdd, Evidência de C12 (G3 aprovado por /root/c12_critic), Evidência de C13 (G3 aprovado com follow-up documental por /root/c13_critic), Fixture em barras agregadas, Objetivos e critérios de aceite (+6 more)

### Community 17 - "query"
Cohesion: 0.18
Nodes (11): query, Find best matching node, Find best-matching start nodes, For /graphify explain, For /graphify path, graphify reference: query, path, explain, or: graphify query "QUESTION" --dfs --budget 3000, Score each node by term overlap for ranked output (+3 more)

### Community 18 - "Threat model — módulo `org`"
Cohesion: 0.11
Nodes (18): 0. Entradas revisadas, 10. Próximos passos, 1. Escopo e premissas, 2. Ativos, 3. Atores, 4. Fronteiras de confiança, 5.a Escalada de autoridade, 5.b Tenant spoofing (+10 more)

### Community 19 - "backend-corrections-sdd"
Cohesion: 0.20
Nodes (10): Contexto e limites, Contratos que o código atual procura implementar, backend-corrections-sdd, Entregas possíveis após revisão, Não objetivos, Objetivos propostos, SDD — Correções de configuração, mercado e organização do backend, Segurança e limites conhecidos (+2 more)

### Community 20 - "core-completeness-sdd"
Cohesion: 0.20
Nodes (10): Contexto, core-completeness-sdd, Escopo — o que entra no `core`, Matriz de dependência, Não-objetivos, Objetivo, Rollback, SDD — Completude transversal do `core` (+2 more)

### Community 21 - "rest-redirect-sdd"
Cohesion: 0.20
Nodes (10): Alternativas e decisão, Contexto e objetivo, Design e seams propostos para acordo do usuário, rest-redirect-sdd, Entregas, TDD e verificação, Evidência de C10 — 2026-09-27, Evidência de C9 — 2026-09-27, Preflight T-17 (somente leitura) (+2 more)

### Community 22 - "exports"
Cohesion: 0.22
Nodes (9): exports, graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag) (+1 more)

### Community 23 - "github-and-merge"
Cohesion: 0.22
Nodes (9): Add --backend gemini|kimi|openai|deepseek|claude-cli depending on which API key you have set, Clone each repo, run the full pipeline on each, then merge, github-and-merge, graphify reference: GitHub clone and cross-repo merge, Run /graphify on each local path to produce their graph.json files, Step 0 - Clone GitHub repo(s) (only if a GitHub URL was given), Then merge:, Then merge at the project root: (+1 more)

### Community 24 - "SDD — Módulo `org` para estrutura e governança de agentes"
Cohesion: 0.07
Nodes (29): 10. API de domínio, 11. Persistência PostgreSQL, 12. Auditoria e privacidade, 13. Coexistência e cutover da hierarquia atual, 14. Dependências externas (P1, P2, P3), 15. Seções sensíveis à segurança (threat model normativo), 16. Riscos principais, 17. Observabilidade e operação (+21 more)

### Community 25 - "Estratégia de grafo unificado (Neo4j)"
Cohesion: 0.08
Nodes (25): 10. Fases de rollout, 11. Anti-patterns (técnicos e ops), 12. Diagrama — fluxo de dados alvo (F1+), 13. Integração com operações existentes, 14. Próximo passo implementável (fatia vertical recomendada), 15. Referências, 16. F2 entregue (bots write-only, best-effort), 1. Contexto e objetivo (+17 more)

### Community 26 - "orders-live-execution-gate2-sdd"
Cohesion: 0.14
Nodes (14): Contexto, Critérios de fechamento G2 (checklist), Critérios para sair de “rascunho” (threat model), orders-live-execution-gate2-sdd, Não-objetivos, Objetivo (quando aprovado), Pendências de decisão, Próximo slice: poller de reconciliação (design) (+6 more)

### Community 27 - "core-services-integration-sdd"
Cohesion: 0.33
Nodes (6): Bootstrap, core-services-integration-sdd, Gaps, Matriz módulo × core, SDD — Integração de módulos com serviços do `core`, Validação

### Community 28 - "Slices"
Cohesion: 0.06
Nodes (35): Approval, Decisions required from the owner (block only their dependents), Dependency table, Global constraints, Open design inputs, Org Complete System Implementation Plan, P1 — Owner authentication / IdP and bootstrap (human principals only), P2 — DB roles, migrations and isolated PostgreSQL (+27 more)

### Community 29 - "transcribe"
Cohesion: 0.40
Nodes (5): transcribe, graphify reference: transcribe video and audio, print progress to stdout, which would otherwise corrupt the JSON file (#1392)., Step 2.5 - Transcribe video / audio files (only if video files detected), Write the JSON from Python (NOT a shell '>' redirect): transcribe_all/Whisper

### Community 30 - "add-watch"
Cohesion: 0.50
Nodes (4): add-watch, For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 31 - "hooks"
Cohesion: 0.50
Nodes (4): hooks, For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration

### Community 33 - "monitor-persistence-policy-sdd"
Cohesion: 0.15
Nodes (13): Alternativas e decisão, Contexto, objetivo e limites, Contratos e seams propostos para acordo do usuário antes de testes, Desenho, monitor-persistence-policy-sdd, Escritas, retries e reconciliação, Evidência C16 (startup), Evidência C17 (runtime; revisão independente aprovada com follow-up) (+5 more)

### Community 34 - "0001-backend-core-modules-mvc"
Cohesion: 0.14
Nodes (14): Alternatives, Configuração e persistência, Critério de escolha: reorganizar ou manter, Design, Destino e seams de tipos, Direção e admissão observável do core, 0001-backend-core-modules-mvc, Drawbacks e riscos (+6 more)

### Community 35 - "Análise do módulo `org` para o sistema de agentes"
Cohesion: 0.15
Nodes (12): Alternativas, Análise do módulo `org` para o sistema de agentes, Dependências e módulos auxiliares, Estado, Fases sugeridas, Modelo conceitual, O que existe hoje, Objetivo e estado (+4 more)

### Community 36 - "backend-module-map-sdd"
Cohesion: 0.17
Nodes (12): Alternativas e decisão, Contexto e objetivo, backend-module-map-sdd, SDD T-16 — Mapa de módulos do backend no README, Superfície e desenho documental, Validação e gates, Addendum (2026-09-27 sessão 2), modules-relocation-phase4-5-sdd (+4 more)

### Community 37 - "agents-capability-research"
Cohesion: 0.17
Nodes (12): Bloqueios e decisões pendentes, Capacidades recomendadas por fase, Comparação de capacidades e decisão, agents-capability-research, Etapa 1 — identidade `IdentityOnly`, Fases posteriores, com SDD e gates próprios, Fontes primárias, Implementação parcial no backend (não fecha Etapa 1) (+4 more)

### Community 38 - "bots-runtime-live-gate2-sdd"
Cohesion: 0.17
Nodes (12): Contexto, Critérios de fechamento G2 (checklist), bots-runtime-live-gate2-sdd, Não-objetivos, Objetivo (quando aprovado), Pendências de decisão, Rollout / rollback, SDD — Gate 2: runtime bots (executor live) (+4 more)

### Community 39 - "monitor-pause-resume-sdd"
Cohesion: 0.17
Nodes (12): Alternativas consideradas, Contexto e problema, Contratos e seams para acordo do usuário antes dos testes, Dados, segurança e observabilidade, Design, monitor-pause-resume-sdd, Entregas, TDD e validação, Estado e transições (+4 more)

### Community 40 - "integrations"
Cohesion: 0.20
Nodes (10): Binance REST, Binance WebSocket, ccxt vendorizado, integrations, Integrações do backend, Mapa de integrações, Matriz de risco de integração, PostgreSQL (+2 more)

### Community 41 - "module-implementation-status"
Cohesion: 0.20
Nodes (10): `core/`, Critério de linha, module-implementation-status, Gates (G4), Lacunas conhecidas (não bloqueiam o objetivo literal), `modules/`, `presentation/`, Restrições do objetivo literal (+2 more)

### Community 42 - "bots-module-sdd"
Cohesion: 0.20
Nodes (10): 1. Contexto e objetivo, 2. Convenção MVC, 3. Seams públicos, 4. Migração desde backtest, 5. Validação e rollout, 6. Pendências, bots-module-sdd, Não objetivos (+2 more)

### Community 43 - "market-module-phase3-sdd"
Cohesion: 0.20
Nodes (10): Contexto, Critérios de aceite, market-module-phase3-sdd, Mapeamento MVC (esta entrega), Objetivo, Próxima fatia (F4 — monitor), Riscos, Rollback (+2 more)

### Community 44 - "provider-credentials-db-sdd"
Cohesion: 0.20
Nodes (10): Dev seed (ops), provider-credentials-db-sdd, Fail-closed, HTTP admin CRUD (stub, fail-closed), Public seam, Rotation, Schema, SDD: Provider credentials in PostgreSQL (+2 more)

### Community 45 - "runbook"
Cohesion: 0.22
Nodes (9): Diagnóstico rápido, runbook, Encerramento e recuperação, Escopo, Inicialização, Operação durante a execução, Pré-voo, Runbook operacional do backend (+1 more)

### Community 46 - "SDD — Outbox PG para projeção Neo4j (fatia F2.1)"
Cohesion: 0.22
Nodes (9): 1. Contexto, 2. Objetivo (F2.1), 3. Schema, 4. Fail-closed, 5. Validação, 6. F2.1.2 (*implemented*), 7. F2.1.3 (*partial* — CLI drain), 8. F2.1.3+ (*partial* — enqueue na mesma TX) (+1 more)

### Community 47 - "persistence-agents-bots-scaffold-sdd"
Cohesion: 0.22
Nodes (9): 1. Contexto, 2. Objetivo, 3. Seams públicos (esta fatia), 4. Mapeamento modelo → coluna, 5. Riscos e mitigação, 6. Validação, 7. Próximo gate, persistence-agents-bots-scaffold-sdd (+1 more)

### Community 48 - "SDD — Projeção Neo4j hierarquia de agents (fatia F1)"
Cohesion: 0.25
Nodes (8): 1. Contexto, 2. Objetivo, 3. Modelo de grafo (F1), 4. Configuração, 5. Validação, 6. Rollback, 7. F2 (bots — SDD separado), SDD — Projeção Neo4j hierarquia de agents (fatia F1)

### Community 49 - "SDD — Projeção Neo4j bots (fatia F2)"
Cohesion: 0.25
Nodes (8): 1. Contexto, 2. Objetivo, 3. Modelo de grafo (F2), 4. Configuração, 5. Validação, 6. Rollback, 7. F2.1 (pendente), SDD — Projeção Neo4j bots (fatia F2)

### Community 50 - "core-database-sdd"
Cohesion: 0.20
Nodes (10): Contexto e objetivo, core-database-sdd, Env, Migrações, Próximo, SDD — `core::database`, Seams, Validação (+2 more)

### Community 51 - "SDD — Projeção Neo4j OrderIntent (fatia F3)"
Cohesion: 0.25
Nodes (8): 1. Contexto, 2. Objetivo, 3. Modelo de grafo, 4. Configuração, 5. Validação, 6. Rollback, 7. Próxima fatia, SDD — Projeção Neo4j OrderIntent (fatia F3)

### Community 52 - "http-admin-auth-seam-sdd"
Cohesion: 0.29
Nodes (7): Comportamento, Contexto, http-admin-auth-seam-sdd, Fora de escopo, Observabilidade (read-only), SDD — HTTP admin bearer seam, Validação

### Community 53 - "bots-catalog-persistence-gate1-sdd"
Cohesion: 0.33
Nodes (6): bots-catalog-persistence-gate1-sdd, Implementado, Pendente, Rollback, SDD — Gate 1: PostgreSQL para agents e catálogo bots, Validação

### Community 55 - "grok-bot-overview"
Cohesion: 0.40
Nodes (5): grok-bot-overview, FAQ excerpts, Grok Bot, Shared computer, What makes Grok Bot different

### Community 56 - "master-plan.md"
Cohesion: 0.32
Nodes (3): Correção normativa, Errata — plano de implementação de org, Ordem normativa

### Community 57 - "SDD — Product owner bootstrap (G1 fatia mínima)"
Cohesion: 0.50
Nodes (4): Escopo desta fatia, Fora de escopo (permanece pendente G1 produto), SDD — Product owner bootstrap (G1 fatia mínima), Validação

### Community 58 - "Threat model — orders Gate 2"
Cohesion: 0.17
Nodes (12): 0. Entradas revisadas, 10. Bloqueios e próximos passos, 1. Contexto resumido (estado real do código), 2. Ativos, 3. Atores, 4. Fronteiras de confiança, 5. Casos de abuso, 6. Mitigações e critérios de aceite testáveis (SEC-ORD) (+4 more)

### Community 59 - "Threat model — autenticação do owner / IdP (P1)"
Cohesion: 0.17
Nodes (12): 0. Fontes lidas, 10. Não verificado, 1. Estado atual em uma frase, 2. Ativos, 3. Atores, 4. Fronteiras de confiança, 5. Casos de abuso, 6. Achados (+4 more)

### Community 60 - "SDD W0-02 — CI verde + PG fail-loud + DB de teste isolado (stub)"
Cohesion: 0.20
Nodes (10): Contexto (evidência no código, HEAD `b2001a7c`), Contradições doc × código, Critérios de aceite (conclusão da Onda 0), Decisão, Dependências, Riscos, Rollout / rollback, SDD W0-02 — CI verde + PG fail-loud + DB de teste isolado (stub) (+2 more)

### Community 61 - "SDD W0-03 — monitor com persistência desligada não abre nem migra PG [SEGURANÇA]"
Cohesion: 0.20
Nodes (10): Contexto (evidência no código, HEAD `b2001a7c`), Contradições doc × doc × código, Critérios de aceite, Decisão, Dependências, Riscos, Rollout / rollback, SDD W0-03 — monitor com persistência desligada não abre nem migra PG [SEGURANÇA] (+2 more)

### Community 62 - "SDD W0-12 — resultado ambíguo de ordem não libera o claim [SEGURANÇA]"
Cohesion: 0.20
Nodes (10): Contexto (evidência no código, HEAD `b2001a7c`), Contradição doc × código, Critérios de aceite, Decisão (correção mais simples correta), Dependências, Riscos, Rollout / rollback, SDD W0-12 — resultado ambíguo de ordem não libera o claim [SEGURANÇA] (+2 more)

### Community 63 - "legacy-file-cleanup-sdd"
Cohesion: 0.22
Nodes (9): Alternativas, Contexto e objetivo, legacy-file-cleanup-sdd, Evidência de uso e decisão por arquivo, Riscos e controles, Rollback e gate, SDD — Remoção de arquivos legados de configuração e migração, Seams públicos e comportamento preservado (+1 more)

### Community 64 - "SDD W0-13 — orders sem `block_on` dentro do runtime Tokio"
Cohesion: 0.22
Nodes (9): Contexto (evidência no código, HEAD `b2001a7c`; análise estática), Critérios de aceite, Decisão, Dependências e arquivos compartilhados, Riscos, Rollout / rollback, SDD W0-13 — orders sem `block_on` dentro do runtime Tokio, Seams públicos para acordo antes do TDD (+1 more)

### Community 65 - "orders-module-sdd"
Cohesion: 0.25
Nodes (8): Contexto, orders-module-sdd, Não-objetivos, Objetivo, Rollback, SDD — Módulo `modules/orders`, Seams públicos, Validação

### Community 66 - "Achado F-ADM-01 — auth admin HTTP "fail-closed" é, na prática, opcional (fail-open)"
Cohesion: 0.25
Nodes (8): 1. Evidência (estática), 2. Severidade, 3. Correção recomendada, 4. Critérios de aceite testáveis (SEC-ADM), 5. Itens não verificados, Achado F-ADM-01 — auth admin HTTP "fail-closed" é, na prática, opcional (fail-open), Cobertura das rotas (`routes/mod.rs:25-93`), Documentação que afirma "fail-closed"

### Community 67 - "graph-query-port-f3-sdd.md"
Cohesion: 0.29
Nodes (6): Contexto, Critérios de fechamento (fatia F3 parcial), HTTP admin read-only (F3 fatia 1), SDD — GraphQueryPort (F3 read-only), Seams públicos, Validação

### Community 68 - "monitor-persistence-c17-sdd.md"
Cohesion: 0.29
Nodes (5): C17 — persistência do monitor, Escopo, Fatia 1 — metadata opcional do supervisor em PG, Semântica, Testes

### Community 69 - "Addendum V18 — persistência de mercado (T-15)"
Cohesion: 0.29
Nodes (6): Addendum V18 — persistência de mercado (T-15), Comportamento comprovado, Entrega, Estado, Seam público exercitado, Validação

### Community 70 - "backend-work-plan"
Cohesion: 0.33
Nodes (6): backend-work-plan, Escopo e gates, Plano de execução das correções pendentes do backend, Próximas ações do Orquestrador, Trilha paralela — completude bots / orders / agents / HTTP, Verificação e limites

### Community 71 - "postgres-and-graph-dev"
Cohesion: 0.40
Nodes (5): DATABASE_URL (market / migrations), postgres-and-graph-dev, Neo4j (opcional), PostgreSQL 18+ e Neo4j — desenvolvimento, Provider credentials (LLM API keys)

### Community 72 - "Fatia 2 — TUI/presentation + REST do estado de arquivo (T-15 runtime)"
Cohesion: 0.40
Nodes (5): Escopo, Fatia 2 — TUI/presentation + REST do estado de arquivo (T-15 runtime), Fora de escopo (fatia 2), Semântica, Testes (unit)

### Community 73 - "Achado F-CRED — credenciais de provider/exchange em texto claro"
Cohesion: 0.40
Nodes (5): 1. Onde e como as credenciais estão, 2. Achados, 3. Critérios de aceite testáveis (SEC-CRED), 4. Itens não verificados, Achado F-CRED — credenciais de provider/exchange em texto claro

## Knowledge Gaps
- **881 isolated node(s):** `1. Contexto e objetivo`, `2. PostgreSQL — função (SoT autoritativo)`, `3. Neo4j — função (grafo único complementar)`, `4.1 Contrato em `AppDatabases``, `4.2 Fluxos` (+876 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 882 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `index` connect `index` to `unimplemented-modules-analysis`, `monitor-presentation-contract-sdd`, `agents-module-sdd`, `modules-completeness-audit`, `current-state-and-roadmap`, `cli-and-config`, `test-matrix`, `module-catalog`, `backtest-trades-and-slippage-sdd`, `backend-corrections-sdd`, `rest-redirect-sdd`, `orders-live-execution-gate2-sdd`, `monitor-persistence-policy-sdd`, `0001-backend-core-modules-mvc`, `backend-module-map-sdd`, `agents-capability-research`, `bots-runtime-live-gate2-sdd`, `monitor-pause-resume-sdd`, `integrations`, `module-implementation-status`, `bots-module-sdd`, `provider-credentials-db-sdd`, `runbook`, `core-database-sdd`, `http-admin-auth-seam-sdd`, `bots-catalog-persistence-gate1-sdd`, `unified-neo4j-graph-strategy.md`, `grok-bot-overview`, `master-plan.md`, `legacy-file-cleanup-sdd`, `orders-module-sdd`, `graph-query-port-f3-sdd.md`, `backend-work-plan`, `postgres-and-graph-dev`?**
  _High betweenness centrality (0.143) - this node is a cross-community bridge._
- **Why does `modules-mvc-convention-sdd` connect `modules-mvc-convention-sdd` to `unimplemented-modules-analysis`, `0001-backend-core-modules-mvc`, `core-extraction-phase1-sdd`, `agents-module-sdd`, `backend-module-map-sdd`, `monitor-presentation-contract-sdd`, `bots-module-sdd`, `market-module-phase3-sdd`, `module-catalog`, `core-completeness-sdd`, `master-plan.md`?**
  _High betweenness centrality (0.091) - this node is a cross-community bridge._
- **Why does `Org Complete System Implementation Plan` connect `Slices` to `master-plan.md`?**
  _High betweenness centrality (0.089) - this node is a cross-community bridge._
- **What connects `1. Contexto e objetivo`, `2. PostgreSQL — função (SoT autoritativo)`, `3. Neo4j — função (grafo único complementar)` to the rest of the system?**
  _881 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `SKILL` be split into smaller, more focused modules?**
  _Cohesion score 0.02666666666666667 - nodes in this community are weakly interconnected._
- **Should `unimplemented-modules-analysis` be split into smaller, more focused modules?**
  _Cohesion score 0.04878048780487805 - nodes in this community are weakly interconnected._
- **Should `core-extraction-phase1-sdd` be split into smaller, more focused modules?**
  _Cohesion score 0.11764705882352941 - nodes in this community are weakly interconnected._