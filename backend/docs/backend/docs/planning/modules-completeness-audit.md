---
title: Auditoria ampliada de completude dos módulos do backend
description: Contratos documentados, comportamento observado, lacunas, dependências e fatias sugeridas para backend dev/testnet
tags:
  - planning
  - backend
  - modules
  - audit
---

# Auditoria ampliada de completude dos módulos do backend

- **Snapshot:** 2026-09-28. Escopo: 11 diretórios em src/modules, mais core e presentation.
- **Método:** contratos e critérios de aceite vêm de SDDs existentes; comportamento vem do código e das evidências documentadas. Esta revisão não executou testes, binário, Docker, banco, exchange ou CI.
- **Veredito:** completude total não demonstrada. Não há critério único aprovado para “todos os módulos completos”. O MVC mínimo mede existência de camadas/seams e não fecha as capacidades de produto. Nenhum módulo recebe aqui o rótulo completo.

## Prioridade proposta

**P0** são gates de isolamento necessários antes de executar integração; **P1** é o necessário para iniciar testes dev/testnet; **P2** requer decisão de produto. São prioridades sugeridas, não requisitos já aprovados. Cada fatia precisa de SDD, Critic G1 independente, acordo explícito dos seams antes de escrever testes, TDD red/green e revisão independente da implementação, conforme AGENTS.md. Esta matriz não autoriza ampliar produto nem ativar ordens live.

## Gates transversais

1. **Banco por ambiente e isolamento (P0).** T-DB-ENV recebeu G1 com follow-ups e especifica seletor de ambiente, URLs separadas e isolamento local. A implementação G3 e a revisão Critic ainda são pendentes; nenhum G4 PostgreSQL é afirmado aqui. Produção ainda não tem endpoint nem secret manager. Fonte: [database-environment-sdd](../sdd/database-environment-sdd.md), WIP 763808f949fad08b523437468569fefe356fa743.
2. **Isolamento dos testes (P0).** T-W0-06 trata contenção de alvos, lock/opt-in Testnet e ausência de efeitos por padrão; status proposto/G1 pendente. Conteúdo mais recente disponível: 0bb035db2a189cb54f322c12dab0898fa08107eb. Não declarar suíte integral nem rodar integrações DB/Docker/exchange antes de Critic aprovar o documento vigente e o runner demonstrar os contratos.
3. **Evidência.** [Matriz de testes](../reference/test-matrix.md), [evidência machine-readable](./modules-completeness-evidence.json) e [status MVC](../architecture/module-implementation-status.md) registram resultados datados, não execução atual. Números históricos incluem 520 testes do bin em 2026-09-27, 62 HTTP e manifesto PG como contagem estática.
4. **Sem dinheiro real.** O código recusa execução de ordem em prod em [core/config/mod.rs](../../src/core/config/mod.rs:381). [exchanges/rest.rs](../../src/modules/exchanges/rest.rs:31) só libera backfill público e opt-ins Spot dev. Execução de produção exige SDD/threat model/ops/autorização próprios e fica desabilitada.

## Matriz dos módulos

### agents

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Identidade administrativa separada de bots; hierarquia/lifecycle válidos; auditoria em memória; advisory Jev opcional; sem worker, ferramenta ou ordem. Owner humano verificável é requisito separado. | Models/controllers/adapters/testes em [agents/mod.rs](../../src/modules/agents/mod.rs:6). SDD descreve espelhamento PG e hydrate no boot; execução PG não comprovada nesta revisão. | **P1:** authority-from-client/AGT-SEC-01 e IdP/owner humano continuam gaps documentados; bootstrap PG não equivale a IdP. Depende de auth e DB isolado. Fatias: SDD de principal/owner e threat model; TDD de autorização/negação; PG cold-start no runner descartável. [agents-module-sdd](../sdd/agents-module-sdd.md), [owner bootstrap G1](../sdd/agents-owner-bootstrap-g1-sdd.md). |

### backtest

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| SMA offline; candle fechado e preenchimento no próximo open; fixture CLI produz trade Sell fechado; slippage/taxas calculados e tamanho/timestamp verificados. Dados sintéticos não representam mercado. | CLI gera fixture em [backtest/cli.rs](../../src/modules/backtest/cli.rs:184); controller calcula slippage em simulation.rs:148,178,224,321-346. SDD relata C12/C13 G3 aprovado e testes nas combinações permitidas, mas não houve repetição nesta auditoria. | **P1:** persistência opcional depende do DB/test harness isolado. **P2:** fontes reais, estratégias adicionais e métricas/requisitos de validade não especificados. Primeiro fechar P0; depois SDD/TDD de persistência no DB descartável. [backtest trade/slippage SDD](../sdd/backtest-trades-and-slippage-sdd.md), [database environment SDD](../sdd/database-environment-sdd.md). |

### bots

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Catálogo versionado de estratégia×timeframe/símbolo, métricas/ranking determinísticos; runtime fail-closed por padrão; promoção manual e binding de evaluator; agente e bot são entidades distintas. | Models/controllers/adapters em [bots/mod.rs](../../src/modules/bots/mod.rs:6); SDDs registram catálogo PG e runtime de SMA/EMA como parcial. | **P1:** provar persistência/hydrate em PG isolado; owner/IdP e paridade runtime serve/testes não fechados. **P2:** política de promoção/auditoria além do seam atual precisa decisão. Depende de agents, monitor e DB. SDD de auth, TDD de capability/replay e teste PG descartável. [bots SDD](../sdd/bots-module-sdd.md), [bots runtime G2](../sdd/bots-runtime-live-gate2-sdd.md), [catalog PG G1](../sdd/bots-catalog-persistence-gate1-sdd.md). |

### exchanges

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Tipos de conta/transporte/mercado; REST/WS governados por timeframe; REST privado fail-closed, exceto opt-ins aprovados. Dev usa contas de teste. | Registry/bootstrap/Binance REST/WS/Testnet adapters em [exchanges/mod.rs](../../src/modules/exchanges/mod.rs:1). [rest.rs](../../src/modules/exchanges/rest.rs:31) permite backfill Spot dev e submit só via recording/Testnet opt-in. Config rejeita modo Testnet no monitor (core/config/mod.rs:390-393). | **P1:** provar assinatura/redação/redirect/timeouts/reconciliação só em Testnet após T-W0-06. Redirect SDD e tests external pendentes. **P2:** outras venues/tipos de mercado sem critérios aceitos. Depende de credentials, orders, risk, DB e isolamento. TDD offline com fake; integração opt-in somente Testnet. [REST redirect SDD](../sdd/rest-redirect-sdd.md), [orders G2](../sdd/orders-live-execution-gate2-sdd.md). |

### http_bridge

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Facades finas por domínio entre rotas e use cases; sem duplicar política de domínio. | Facades por domínio em [http_bridge/mod.rs](../../src/modules/http_bridge/mod.rs:8). Auditoria anterior registra 62 http_integration em 2026-09-27; não reexecutado. [admin_auth.rs](../../src/presentation/http/admin_auth.rs:96) retorna erro quando token não configurado. | **P1:** cobrir as rotas mutantes/sensíveis com DB dev isolado e bearer contract (503 configuração ausente, 401 token inválido). Docs antigos que dizem “rotas abertas” divergiram do source atual. Auth admin não prova identidade do owner humano. Depende de HTTP auth e DB; TDD offline por rota, PG isolado para endpoints persistentes. [HTTP auth SDD](../sdd/http-admin-auth-seam-sdd.md), [core integration SDD](../sdd/core-services-integration-sdd.md). |

### market

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| OHLCV/timeframes/datasets/resampling; feed híbrido REST+WS com contiguidade/dedupe/watermark; I/O em exchanges e persistência por adapter. | Estrutura models/controllers/adapters em [market/mod.rs](../../src/modules/market/mod.rs:3); matriz lista testes determinísticos, sem execução atual. Phase3 SDD é draft e descreve layout anterior. | **P1:** validar feed REST/WS e redirect com comportamento atual; persistência só em DB descartável. **P2:** frescor/latência/venues adicionais não especificados. Depende de exchanges, monitor e persistence. TDD offline de feed, fake HTTP redirect e integração PG após P0. [market phase3 SDD](../sdd/market-module-phase3-sdd.md), [redirect SDD](../sdd/rest-redirect-sdd.md), [monitor persistence SDD](../sdd/monitor-persistence-policy-sdd.md). |

### monitor

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Loop observe/paper; pausa interrompe avaliação; retomada exige backfill REST atual/contíguo; estado de persistência OFF/HEALTHY/DEGRADED/GAP quando opt-in. Monitor não envia ordens reais. | Supervisor/startup/handle/views em [monitor/mod.rs](../../src/modules/monitor/mod.rs:5). T-10 seams aprovados; persistência C17 registrada; contrato de apresentação watch/broadcast/shutdown ainda draft. | **P1:** executar fluxo de pausa/retomada, headless/serve e persistência em PG isolado. **P2:** SLI/SLO/alertas/runbook não definidos; core SDD deixa observabilidade avançada fora de escopo. Depende de market/strategy/risk/portfolio/DB/presentation. SDD apresentação e TDD offline de controle/lag/shutdown antes de integração. [pause/resume](../sdd/monitor-pause-resume-sdd.md), [persistence](../sdd/monitor-persistence-policy-sdd.md), [presentation contract](../sdd/monitor-presentation-contract-sdd.md), [core completeness](../sdd/core-completeness-sdd.md). |

### orders

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| OrderIntent validado por risk antes do executor; default fail-closed; paper ledger, client_order_id idempotente e reconciliação; Testnet apenas opt-in dev; prod desabilitado. | Submit/paper/recording/Testnet, stores PG e polling em [orders/mod.rs](../../src/modules/orders/mod.rs:8). G2 SDD chama execução/reconciliação parciais e Critic/threat model pendentes. Config recusa prod em [config/mod.rs](../../src/core/config/mod.rs:381). | **P1:** provar idempotência/reconciliação em PG isolado; depois submit mínimo Testnet sob opt-in após isolamento; revisar ambiguidade de resposta, async e redação. **P2:** produção não especificada nem autorizada. Depende de risk/exchanges/credentials/DB/harness. TDD offline de replay/erro; integração Testnet independente. [orders SDD](../sdd/orders-module-sdd.md), [orders G2](../sdd/orders-live-execution-gate2-sdd.md), [ambiguous claim SDD](../sdd/wave0-12-order-ambiguous-claim-sdd.md), [async SDD](../sdd/wave0-13-orders-block-on-sdd.md). |

### portfolio

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Superfície comprovada é snapshot/posições paper alimentados por paper fills; não localizei contrato para portfolio financeiro completo. | Models/controllers de asset/paper fill em [portfolio/mod.rs](../../src/modules/portfolio/mod.rs:1); snapshot HTTP/paper test descritos na auditoria/matriz datadas. | **P1:** provar paper submit→snapshot no processo dev. Persistência só se sobrevivência a restart for desejada. **P2 decisão:** valuation dinâmica, balances, fees, reconciliação exchange, múltiplas moedas e ledger durável sem spec aprovada. Depende de orders; decidir escopo antes de SDD e testes. Sem SDD específico de completude localizado. |

### risk

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Limites/perfis e gate de sinais/intents; orders chama validação antes do port; risk não executa exchange nem UI. | Models/controllers/tests em [risk/mod.rs](../../src/modules/risk/mod.rs:1); orders SDD exige validação pré-executor. Test matrix lista testes de limites e sizing, não executados nesta revisão. | **P1:** revalidar invariantes e provar que intenção inválida não chama executor. **P2 decisão:** política financeira real, limites por portfolio/volatilidade/gap/produção não definidos; não inferir. TDD offline dos contratos existentes; SDD só após decisão para política adicional. |

### strategy

| Contrato e aceite documentado | Estado observado | Gaps, dependências e fatias sugeridas |
|---|---|---|
| Avaliação determinística de candles fechados; SMA/EMA no runtime de bots; Signal compartilhado, sem envio de ordem. | Models/controllers/reexports em [strategy/mod.rs](../../src/modules/strategy/mod.rs:1); bots runtime SDD cita evaluate_for_kind em monitor/backtest. Matriz lista testes, sem execução atual. | **P1:** provar paridade evaluator monitor/backtest e binding de bots com fixtures offline. **P2 decisão:** catálogo, versionamento além dos evaluators atuais, otimização/treino e critérios de performance não definidos. Depende de market e application contracts; TDD por fixtures/propriedades e SDD apenas para estratégias aprovadas. |

## Cross-cutting: core e presentation

| Área | Contrato e comportamento observado | Gaps e fatias sugeridas |
|---|---|---|
| **core/config** | Parsing/validação; configuração atual recusa execução prod [config/mod.rs](../../src/core/config/mod.rs:381). | **P0:** concluir resolver único por ambiente e caller inventory. **P2:** prod depende de endpoint/secret manager inexistentes. SDD T-DB-ENV vigente. |
| **core/database/persistence** | Bootstrap PG + Neo4j opcional, migrações/transações/projeções em [database/mod.rs](../../src/core/database/mod.rs:5). Execução PG não comprovada aqui. | **P0:** G3 target dev + Critic, mismatch sem conexão e runner isolado antes de G4; snapshot do banco existente antes de qualquer limpeza. Neo4j apenas se target local identificável e opt-in. T-DB-ENV/T-W0-02b/T-W0-06. |
| **core/providers** | Providers e credential cache/CRUD. [credentials/mod.rs](../../src/core/providers/credentials/mod.rs:1) declara encryption mode none e plaintext em PG. | **P1 segurança:** W0-07 precisa de SDD/revisão, encryption envelope e migração/backfill antes de alegar credenciais seguras. Sem secrets reais em fixtures/logs. |
| **core/health/logging/notifications** | Liveness/readiness e LogNotifier descritos em [core-completeness-sdd](../sdd/core-completeness-sdd.md). | Metrics, SLI/SLO, alertas, dashboards e runbooks são explicitamente fora do escopo atual. **P2 decisão:** escolher requisitos operacionais antes de alegar prontidão prod. |
| **presentation/http** | Serve, rotas, OpenAPI, bearer e ApiState; código admin auth em [admin_auth.rs](../../src/presentation/http/admin_auth.rs:96). | **P1:** smoke HTTP atual e rotas persistentes após gates; auth admin não equivale IdP/owner. Docs antigos de falha aberta estão superseded pelo source. |
| **presentation/terminal** | View Ratatui consumidora do monitor. | Se TUI for parte do aceite: aprovar contrato apresentação e TDD snapshot/commands/lag/shutdown; depende de monitor. Caso HTTP-first, priorizar depois do dev API. [monitor presentation contract](../sdd/monitor-presentation-contract-sdd.md). |

## Reconciliação e decisões pendentes

- A auditoria anterior cobria bots/orders/agents/HTTP; esta amplia para 11 módulos sem tratar contagem histórica como resultado atual.
- [Mapa de módulos T-16](../sdd/backend-module-map-sdd.md) está superseded e representa layout anterior.
- [Status MVC](../architecture/module-implementation-status.md) separa MVC mínimo literal do goal de completude parcial. Números e vereditos nele são do snapshot 2026-09-27, não execução deste turno.
- Docs antigos dizem que rotas admin ficam abertas sem token; source atual retorna erro de configuração se bearer não foi configurado. Revalidar no gate, mas reconciliar docs antes de usá-los como estado atual.
- T-16 também relata zero trades/slippage ausente no backtest; [T-07](../sdd/backtest-trades-and-slippage-sdd.md) registra C12/C13 como corrigidos e aprovados. Esses pontos são históricos, não gaps confirmados.
- Draft/partial do SDD e presença de implementação são dimensões separadas; implementação ou status documental, isolados, não fecham completude.

Decisões humanas necessárias: (1) escopo dev = API+paper+persistência ou inclui Binance Spot Testnet opt-in; (2) prod = configuração/ops ou ordens reais; execução real continua bloqueada; (3) provider IdP/owner humano; (4) portfolio paper ou contabilidade/reconciliação completas; (5) modelo de risk além das regras atuais; (6) SLI/SLO, backup/restore, retenção e resposta operacional. Não inventar essas políticas.

## Sequência proposta

| Ordem | Fatia | Critério observável |
|---|---|---|
| 0 | Critic aprova SDDs T-W0-02b, T-DB-ENV e T-W0-06 vigentes; banco preservado | Contratos de isolamento aprovados; sem execução externa prematura |
| 1 | G3/G4 DB dev + runner de testes | mismatch rejeitado antes de SQL; alvo identificado/descartável; execuções registradas |
| 2 | HTTP dev sem efeitos externos | serve loopback; health/ready/OpenAPI/auth/404 e testes determinísticos passam |
| 3 | Fluxos dev persistidos | agents/bots/orders/monitor/backtest conforme SDD; PG isolado; cada fatia revisada |
| 4 | Binance Spot Testnet opt-in | credenciais verificadas sem expor valores; submit/reconcile em alvo de teste; sem prod |
| 5 | Decisões e desenho operacional de produção | owner/IdP, credenciais cifradas, SLO, backup, runbook, rollback e revisão; ordens live continuam fora até autorização específica |

## ENTREGA — matriz documental

- **Builder:** Orquestrador; síntese documental e source, Markdown atualizado exclusivamente por OpenKnowledge.
- **Artefato:** esta página; o histórico OpenKnowledge preserva a auditoria parcial anterior.
- **Teste/evidência:** nenhum código/teste executado nesta entrega. Referências de teste são históricas e identificadas como tal.
- **Achados/revisão:** PENDENTE — Critic independente /root/test_safety_sdd_critic.
- **Riscos/pendências:** gates DB/test isolation, IdP, credenciais de provider em plaintext e decisões de negócio acima.
- **Veredito:** PENDENTE até revisão independente.
