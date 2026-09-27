---
title: Análise de módulos previstos ainda não desenvolvidos
description: Inventário de capacidades planejadas sem implementação completa, com evidências, dependências e próximos gates
tags:
  - planning
  - backend
  - roadmap
  - gaps
  - modules
---

# Análise de módulos previstos ainda não desenvolvidos

> Revisão: 2026-09-27 (http_bridge v1 completo; agents+monitor registry; bots catalog via `BotCatalogBackend` (memória ou PG)). Esta análise cruza os SDDs, o roadmap, o catálogo de módulos e o código atual em `backend/src`. “Não desenvolvido” significa que não existe módulo/caminho executável correspondente ou que o design ainda não chegou ao comportamento completo descrito. **`modules/agents`** — registry em memória com write-through e cold-start via PG (`load_agent_identity_snapshot`); auth owner pendente. **`modules/bots`** — catálogo/ranking/HTTP com `PgBotCatalogStore` quando PG disponível; runtime live no monitor fora; seam HTTP promote/demote e snapshot com `promoted_bot_id` feitos. **`modules/orders`** existe como seam fail-closed (`submit_order` + `FailClosedExecutor`); execução real permanece bloqueada.

## Resumo

O backend atual implementa monitor de mercado, backtest, estratégia SMA, risco, TUI, integrações públicas Binance, persistência básica opcional, logging e Jev consultivo. Os módulos abaixo ainda não existem como capacidade completa:

1. Identidade de agentes com **auth owner verificável** no HTTP (persistência PG opcional já wired; sem bootstrap seguro).
2. **Módulo `bots` além da fundação** — acoplamento monitor + promoção autorizada por agentes (fundação + seam HTTP `BotRuntimePort` / `BOT_RUNTIME_ENABLED`; ver [SDD bots](../sdd/bots-module-sdd.md) e [Gate 2 runtime](../sdd/bots-runtime-live-gate2-sdd.md)).
3. Autenticação do owner, autorização por agência e bootstrap seguro.
4. Runtime de execução de agentes, worker, scheduler e recuperação.
5. Gateway de ferramentas, permissões, aprovações e sandbox.
6. Memória de conhecimento, memória entre sessões e grafo.
7. Canais de conversa, voz, aplicações e interface externa.
8. Execução financeira live, saldos privados e ambiente de produção (seam `modules/orders` + `POST /api/v1/orders/submit` validam risco e falham fechado com 503; adapter exchange real ausente).
9. Observabilidade operacional completa.
10. Estado de persistência e recuperação do monitor conforme C17.
11. Round-trip PostgreSQL operacional conforme V18.

Essas capacidades não devem ser tratadas como módulos parcialmente prontos só porque existem tipos auxiliares, flags de configuração ou documentação de design.

## Classificação

| Capacidade prevista | Situação no código | Evidência | Próximo gate |
|---|---|---|---|
| Identidade de agentes `IdentityOnly` | Módulo `modules/agents` + rotas HTTP v1; `shared_agent_registry`; write-through PG (`PgAgentIdentityStore`) e cold-start hydrate em `serve`; **sem auth owner**. | [SDD agents](../sdd/agents-module-sdd.md) draft G1; pesquisa mantém Gate 1 bloqueado para auth/bootstrap. | Revisão G1, schema PostgreSQL, persistência e autenticação verificável do owner. |
| Módulo `bots` (executores versionados) | **Fundação + seam runtime** — catálogo/PG, `BotRuntimePort`, HTTP `/bots/runtime/*`, snapshot monitor com promoção; supervisor usa `strategy_evaluation_binding` + `BotSignal.bot_id` quando promoção casa com mercado (SMA ainda do `Config` global). | [SDD bots](../sdd/bots-module-sdd.md); [catálogo](../architecture/module-catalog.md). | Gate 1 persistência feito; [Gate 2 runtime draft](../sdd/bots-runtime-live-gate2-sdd.md); registry multi-estratégia e SMA por catálogo. |
| Seam `orders` (fail-closed) | `modules/orders` + `http_bridge/orders` + `POST /api/v1/orders/submit` (422 risk / 503 execution disabled); `client_order_id` com memória + `PgOrderIdempotencyStore` opcional (`0004`). | [SDD orders](../sdd/orders-module-sdd.md), [Gate 2](../sdd/orders-live-execution-gate2-sdd.md). | Adapter exchange real e reconciliação — somente após gates de segurança. |
| Owner, agência e hierarquia | Não existe autenticação confiável nem autorização por agência. | A pesquisa registra que socket Unix e conta do SO não provam a identidade do owner. | Threat model, bootstrap único, autenticação verificável e revisão de segurança. |
| Runtime de agentes | Não existe cérebro, modelo, delegação ou execução de agente. | A pesquisa exclui chamadas LLM, delegação e runtime da etapa `IdentityOnly`. | SDD próprio de runtime e limites de autoridade. |
| Worker e scheduler | Não existe worker durável, agenda, heartbeat, lease ou retry de execução. | A pesquisa classifica rotinas e operação contínua como fase posterior. | SDD de execução durável, fila/outbox, recuperação e SLO. |
| Gateway de ferramentas | Não existe MCP/tool gateway, política por ação ou aprovação. | Pesquisa: ferramentas, sandbox e aprovações estão excluídos da primeira etapa. | Modelo de permissões, política fail-closed e auditoria. |
| Memória e conhecimento | Não existe memória conversacional, memória semântica ou grafo de conhecimento. | Pesquisa separa histórico administrativo de memória de agente e adia essa fase. | Proveniência, revisão, compartilhamento, retenção e aposentadoria. |
| Canais externos | Não existem canais de chat, voz, mobile, navegador ou computador persistente. | Pesquisa marca canais e dispositivos fora da etapa atual. | Contrato de interação, identidade por canal e controles de privacidade. |
| Execução financeira live | Domínio de intenção em `modules/orders` (fail-closed); sem saldo privado, produção ou REST de ordens. | `exchanges/rest` autoriza somente `PublicSpotBackfill` em `dev`; `authorize_rest_use` falha para usos privados. | Adapter verificado, idempotência, reconciliação e revisão de segurança antes de qualquer port real. |
| Observabilidade | Há logging estruturado, mas não há catálogo completo de métricas, SLI/SLO, alertas ou runbook de incidentes. | Roadmap lista WS/REST, persistência, idade de candle, Jev e credenciais como pendências. | Definir métricas, cardinalidade, alertas, dashboards e runbooks. |
| Persistência de runtime | A camada `persistence` grava datasets, mas o estado de recuperação do monitor ainda não está completo. | SDD T-15 marca C17/G4 pendentes: `DEGRADED`, `HEALTHY`, `GAP`, suspeita de commit e recuperação. | Implementar C17 após C14/C15/C16 e revisar G4. |
| Integração PostgreSQL | Existe conexão, migração e persistência básica; a integração operacional completa não foi executada. | V18 está bloqueada por banco descartável; o teste PostgreSQL é ignorado por padrão. | Executar migração, commit, rollback, idempotência e limpeza em `trading_bot` isolado. |

## 1. Identidade persistente de agentes

### O que está previsto

A primeira etapa descrita na pesquisa é `IdentityOnly`, com:

- identificador estável;
- agência;
- nome e papel/nível;
- supervisor tipado;
- associação ao owner;
- estados de ciclo de vida;
- histórico durável de alterações;
- operações autenticadas de owner.

### O que existe

`modules/agents` implementa tipos de identidade (`AgentId`, `AgencyId`, papéis, supervisor), `AgentRegistry` em memória, transições de ciclo de vida, eventos de auditoria em memória e `run_advisory_step` via `core::providers::jev`. `RegistryMonitorAgentHook` integra ao monitor quando `BOT_AGENCY` está definido e compartilha `shared_agent_registry` com a API no mesmo processo. Há espelhamento PostgreSQL opcional e bearer admin opcional (`BOT_HTTP_ADMIN_TOKEN`); **não há** autenticação verificável do owner nem bootstrap seguro. Os tipos de domínio de trading em `market`/`strategy` permanecem separados da identidade administrativa de agentes.

### Bloqueios

- fonte confiável da identidade do owner;
- bootstrap inicial único e recuperável;
- constraints de hierarquia e ausência de ciclos;
- permissões PostgreSQL;
- contrato público das transições;
- revisão independente de segurança.

### Distinção explícita: agents, bots (futuro), backtest, monitor

| | `modules/agents` | `modules/bots` | `backtest::BotId` | `modules/monitor` |
|---|---|---|---|---|
| Status | Implementado (memória) | Fundação (memória + HTTP catalog/ranking/persist) | Reexport de `bots::BotId`; simulação em `backtest` | Implementado (mercado live/paper) |
| Papel | Governança e identidade `IdentityOnly` | Executor strategy×timeframe versionado (sem runtime live) | Chave canônica compartilhada com `bots` | Supervisor operacional de candles/sinais |

Cadastrar um **agente** não cria um **bot** executor. Rodar backtest com um **BotId** não registra agente nem bot futuro. O monitor não substitui nenhum dos dois cadastros.

## 1b. Módulo `bots` (fundação — não confundir com agents)

### O que está previsto (completo)

Bots especializados como **artefatos versionados**, com limites de autoridade, métricas, avaliação, ciclo de promoção e runtime live — separados da hierarquia administrativa de agentes.

### O que existe hoje

`src/modules/bots/` com models/controllers/adapters: `BotIdentity`, catálogo `build_catalog_from_config`, `full_ranking`/`rank_bots`, `BotCatalogStore` noop, testes em `modules/bots/tests.rs`, `ApiState` com `InMemoryBotCatalogStore`; rotas `GET /catalog`, `POST /ranking`, `POST /catalog/persist`, `GET /catalog/snapshot`. `backtest` delega tipos e ranking ao módulo `bots`. **Há** `PgBotCatalogStore` + tabela `bot_catalog_entries`; **ainda não há** auth owner, promoção automática nem executor live no monitor (seam HTTP `BotRuntimePort` + snapshot enriquecido + publicação headless; loop de estratégia do supervisor ainda não usa `BotId` promovido).

### Decisão

Não tratar `modules/bots` como alias de `modules/agents`. `BotId` é domínio de produto para executor versionado (chave `strategy@version:timeframe:symbol`), não `AgentId`. Definir mapeamento explícito com agentes autorizadores antes de runtime live.

## 2. Controle, autenticação e autorização

### O que está previsto

Toda operação de agente deve validar owner, agência, alvo, ação e estado no lado servidor. A hierarquia owner → CEO → Level B → Level A → especialistas/workers precisa ser uma regra de domínio auditável.

### O que existe

O backend possui gates de configuração e de uso REST da exchange, mas não possui autenticação de usuário, sessões, tokens, autorização por agência ou auditoria de operações administrativas.

### Decisão

Não reutilizar `Credentials` da exchange para autenticar o owner. São domínios diferentes: credenciais de exchange autorizam acesso técnico à Binance, enquanto owner/agência exigem identidade do produto.

## 3. Runtime, worker e scheduler

### O que está previsto

Uma fase posterior precisa suportar execução durável, scheduler, heartbeat, leases, retries limitados, idempotência, recuperação e observabilidade.

### O que existe

`app` possui tarefas do monitor e cancelamento por geração, mas isso não é um runtime geral de agentes. Não há fila de tarefas de agente, scheduler, worker persistente, lease ou recuperação de jobs.

### Risco

Confundir as tasks do monitor com um worker de agentes criaria autoridade implícita e dificultaria separar sinais de mercado de execução autônoma.

## 4. Ferramentas, aprovações e sandbox

### O que está previsto

A pesquisa prevê política por ferramenta/ação, negação por padrão, auditoria, aprovação humana e isolamento de recursos.

### O que existe

Jev recebe um snapshot consultivo e retorna observações textuais. Ele não chama ferramentas, não escolhe conta, não acessa exchange e não autoriza execução. Também não há gateway MCP, catálogo de ferramentas, policy engine ou sandbox.

### Dependências

Esse módulo só pode ser desenhado depois da identidade e autorização. A política precisa definir ator, alvo, ferramenta, argumentos, risco, aprovação, expiração e resultado auditado.

## 5. Memória, conhecimento e canais

Não existem módulos para:

- memória conversacional;
- memória semântica;
- grafo de conhecimento;
- proveniência e revisão de conhecimento;
- canais de chat, voz ou mobile;
- navegador ou computador persistente;
- mensagens entre agentes;
- delegação multiagente.

A pesquisa classifica todos esses itens como fases posteriores ou fora do escopo de `IdentityOnly`. Cada capacidade exige SDD próprio; não deve ser adicionada ao cadastro de identidade como campo executável ou prompt implícito.

## 6. Execução financeira e produção

O caminho de execução está deliberadamente ausente:

- sem criação de ordem;
- sem consulta ou movimentação de saldo privado;
- sem assinatura de ordens;
- sem account trading;
- sem ambiente de produção;
- sem estratégia que possa conceder autoridade financeira.

O módulo `exchanges/rest` autoriza somente backfill público Spot em `dev`. A existência de `RiskLimits`, `PortfolioSnapshot` ou sinais não representa execução financeira. Qualquer implementação futura precisa de autorização explícita, threat model, limites, aprovação humana, idempotência, reconciliação e rollback operacional.

## 7. Persistência e integração ainda incompletas

### C17 — estado de runtime e recuperação

O SDD T-15 ainda prevê:

- estado inicial `DEGRADED`;
- distinção `HEALTHY`, `DEGRADED` e `GAP`;
- suspeita de perda por overflow, pausa, desconexão ou commit incerto;
- recuperação somente após janela REST contígua e commit confirmado;
- descarte de resultados de gerações antigas;
- comportamento de timeout e retry.

C17 depende de C14, C15 e C16 e permanece pendente.

### V18 — PostgreSQL

A persistência base existe, mas a prova operacional completa não foi executada. V18 requer um banco `trading_bot` descartável e isolado para observar migração, commit, rollback após erro e idempotência. O teste ignorado não deve ser promovido a aprovação.

## 8. Observabilidade operacional

O logging atual não fecha a observabilidade necessária. Ainda precisam ser definidos:

- contadores de mensagens WS aceitas, rejeitadas e descartadas;
- latência e erro de REST;
- idade do último candle;
- estado e idade da persistência;
- tamanho e saturação da fila;
- falhas, timeout e latência de Jev;
- métricas de pausa, retomada, gaps e reconciliação;
- SLI/SLO;
- alertas;
- dashboards;
- runbooks de credenciais, banco e degradação.

## Ordem recomendada de desenvolvimento

1. Fechar C10 e o contrato/fallback do adapter REST.
2. Fechar C15 e confirmar o comportamento de backpressure do WS.
3. Fechar C16/C17 e executar V18 em banco isolado.
4. Definir observabilidade operacional e runbooks.
5. Concluir revisão G1 do [SDD agents](../sdd/agents-module-sdd.md) e persistência PostgreSQL.
6. Implementar autenticação, bootstrap e autorização do owner.
7. Só então criar runtime, ferramentas, memória, canais e execução financeira em projetos separados.

## Critérios para considerar um módulo desenvolvido

Um módulo previsto só deve sair desta lista quando houver:

- código presente e integrado ao fluxo correto;
- contrato público documentado;
- teste comportamental determinístico;
- tratamento de erro e falha;
- revisão independente registrada;
- evidência de integração quando houver dependência externa;
- atualização do catálogo, roadmap e matriz de testes.

## Referências

- [Catálogo completo de módulos](../architecture/module-catalog.md)
- [Matriz de testes](../reference/test-matrix.md)
- [Estado atual e planejamento](./current-state-and-roadmap.md)
- [Plano de execução](./backend-work-plan.md)
- [Pesquisa de capacidades de agentes](../research/agents-capability-research.md)
- [SDD — módulo agents (IdentityOnly)](../sdd/agents-module-sdd.md)
- [SDD T-10 — pausa e retomada](../sdd/monitor-pause-resume-sdd.md)
- [SDD T-15 — persistência opcional](../sdd/monitor-persistence-policy-sdd.md)
- [SDD T-05 — redirects REST](../sdd/rest-redirect-sdd.md)
