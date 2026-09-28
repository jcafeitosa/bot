---
title: Auditoria ampliada de completude dos módulos do backend
description: Contratos, estado observável, gaps, dependências e sequência sugerida para backend dev/testnet
tags:
  - planning
  - backend
  - modules
  - audit
---

# Auditoria ampliada de completude dos módulos do backend

- **Snapshot documental:** 2026-09-28.
- **Escopo:** 11 módulos em src/modules, mais core e presentation.
- **Método:** critérios vêm dos SDDs existentes; comportamento vem do código/evidência documental. Esta revisão não executou código, testes, Docker, banco, exchange ou CI.
- **Veredito:** completude total não demonstrada. Não há critério único aprovado de “todos os módulos completos”. MVC mínimo mede estrutura/seams, não fechamento do produto. Nenhum módulo é marcado completo.

## Prioridades e gates de entrada

**P0** = pré-condição de isolamento; **P1** = dev/testnet; **P2** = requisito ainda sem decisão. Prioridade é proposta, não requisito aprovado. Cada entrega futura exige SDD, Critic G1 independente, acordo explícito de seam antes dos testes, TDD red/green e Critic da implementação, conforme AGENTS.md. Esta matriz não autoriza comportamento novo nem ordens live.

1. **Banco (P0):** T-DB-ENV G1 foi **APROVADO COM FOLLOW-UP** na revisão `be7c3d47e10bd0e32ddde84e582066e7dad18ea2` (WIP editorial mais recente `763808f949fad08b523437468569fefe356fa743` não é o hash do veredito G1). **G3 offline está APROVADO**, limitado a verificações com fakes; **G4 continua PENDENTE** para Docker/PostgreSQL reais, CI, smoke operacional e exchange. Não há endpoint/secret manager prod; prod permanece bloqueado. Ver [database environment SDD] (../../../sdd/database-environment-sdd.md).
2. **Isolamento (P0):** T-W0-06 recebeu **G1 APROVADO COM FOLLOW-UP** pelo Critic independente na revisão `d797073e4539fcf85c15a777d087788e5248c3a6`; o reconcile OpenKnowledge posterior é `0bb035db2a189cb54f322c12dab0898fa08107eb`. A aprovação é de desenho e não prova implementação. Antes de integrações, G3 ainda deve demonstrar a topologia executável por perfil, deny-egress/egress permitido, identidade dos alvos, proxy Testnet e observação de processos descendentes; runner real somente no gate G4 aplicável, com o W0-02/helper PG aprovado quando for PostgreSQL. Não declarar suíte integral nem executar DB/Docker/exchange por causa do G1.
3. **Evidência:** [test matrix] (../../../reference/test-matrix.md), modules-completeness-evidence.json e [MVC status] (../../../architecture/module-implementation-status.md) contêm números datados (520 testes bin em 2026-09-27, 62 HTTP, manifesto PG estático). Não comprovam execução atual. Nenhum resultado abaixo foi reexecutado.
4. **Dinheiro real:** source backend/src/core/config/mod.rs:381 recusa execução de ordens prod. backend/src/modules/exchanges/rest.rs:31 libera backfill público e opt-ins Spot dev. Produção continua desabilitada até SDD/threat model/ops/autorização próprios.

## Matriz de módulos

| Módulo | Contrato e critério de aceite documentado | Comportamento/fonte observada | Gaps, prioridade, dependências e fatias SDD/TDD sugeridas |
|---|---|---|---|
| **agents** | Identidade/hierarquia/lifecycle, auditoria, advisory Jev opt-in; agente não cria worker ou ordem. Owner humano verificável é requisito distinto. | Models/controllers/adapters/testes em backend/src/modules/agents/. SDD descreve PG mirror e cold-start hydrate. | **P1:** authority-from-client/AGT-SEC-01 e IdP/owner são gaps documentados; PG execution não comprovada aqui. Depende de HTTP auth e DB isolado. SDD threat model/principal; TDD de autorização/negação; G4 cold-start no DB descartável. [agents SDD] (../../../sdd/agents-module-sdd.md), [owner bootstrap] (../../../sdd/agents-owner-bootstrap-g1-sdd.md). |
| **backtest** | SMA offline; sinal em candle fechado e fill no open seguinte; fixture CLI fecha Sell; custos/slippage e aritmética verificados. Não alega retorno real. | CLI gera candles sintéticos em backend/src/modules/backtest/cli.rs; controller tem cálculo de slippage. T-07 registra C12/C13 G3 aprovado, mas não repetido aqui. | **P1:** --persist depende do DB/test harness isolado. **P2:** feed real, outras estratégias/métricas não especificados. Pós-P0, TDD de persistência no PG descartável. Problemas antigos de zero trades/Sell slippage são históricos, não gaps confirmados; [T-07] (../../../sdd/backtest-trades-and-slippage-sdd.md). |
| **bots** | Catálogo versionado strategy×timeframe/símbolo, ranking, runtime fail-closed, promoção autorizada, evaluator no monitor/backtest. Bot != agente. | backend/src/modules/bots/ implementa models/controllers/adapters; SDD descreve catálogo PG e SMA/EMA runtime parcialmente. | **P1:** PG/hydrate isolado, IdP owner e paridade runtime ainda pendentes. Depende de agents/monitor/DB. SDD de autorização; TDD capability/replay/runtime; teste PG descartável. [bots] (../../../sdd/bots-module-sdd.md), [runtime G2] (../../../sdd/bots-runtime-live-gate2-sdd.md), [catalog PG] (../../../sdd/bots-catalog-persistence-gate1-sdd.md). |
| **exchanges** | Account/transport/market tipados; feed governado; REST privado fail-closed exceto opt-in aprovado. Dev usa contas Testnet. | backend/src/modules/exchanges/ tem Binance REST/WS/Testnet adapters. rest.rs libera backfill Spot dev e submit recording/Testnet explícito. Monitor rejeita RunMode Testnet (core/config/mod.rs:390-393). | **P1:** assinatura, redação, redirects, timeout/reconciliação só em Testnet após T-W0-06. Redirect SDD existe; integração não comprovada. **P2:** outras venues sem especificação. Depende de orders/credentials/DB/isolation; TDD offline + integração Testnet opt-in. [redirect] (../../../sdd/rest-redirect-sdd.md), [orders G2] (../../../sdd/orders-live-execution-gate2-sdd.md). |
| **http_bridge** | Facades por domínio; não duplicar política de domínio. | Facades em backend/src/modules/http_bridge/. Auditoria velha registra 62 HTTP integration em 27/09. backend/src/presentation/http/admin_auth.rs:96 retorna erro sem token. | **P1:** revalidar mutações/GETs sensíveis com contrato bearer e DB dev isolado (503 sem config, 401 token inválido). Docs antigos de rota aberta divergem do source. Admin bearer não é IdP. Depende de HTTP auth e DB; TDD offline por rota, PG isolado para rotas persistentes. [HTTP auth] (../../../sdd/http-admin-auth-seam-sdd.md), [core wiring] (../../../sdd/core-services-integration-sdd.md). |
| **market** | OHLCV/timeframes/dataset/resampling; feed REST+WS contíguo, dedup e watermark; adapters em exchanges/persistence. | backend/src/modules/market/ usa models/controllers/adapters. Test matrix lista testes determinísticos mas não foram executados agora. Phase3 SDD descreve layout anterior e está draft. | **P1:** REST/WS/redirect contra código atual e persistência em DB descartável. **P2:** frescor/latência/venues extras não definidos. Depende de exchanges/monitor/persistence; TDD offline + integração após P0. [market Phase3] (../../../sdd/market-module-phase3-sdd.md), [redirect] (../../../sdd/rest-redirect-sdd.md), [monitor persistence] (../../../sdd/monitor-persistence-policy-sdd.md). |
| **monitor** | Loop observe/paper; pause bloqueia avaliação; resume exige backfill REST válido/contíguo; persist health OFF/HEALTHY/DEGRADED/GAP se opt-in; sem ordem real. | backend/src/modules/monitor/ tem supervisor/startup/handle/views. T-10 seams aprovados; C17 persistence registrada; presentation contract segue draft. | **P1:** pausa/retomada, headless/serve e PG opt-in sem evidência atual. **P2:** SLO/alerta/runbook sem contrato. Depende de market/strategy/risk/portfolio/DB/UI; finalizar SDD de apresentação, TDD offline de controle/lag/shutdown, depois integração. [T-10] (../../../sdd/monitor-pause-resume-sdd.md), [T-15] (../../../sdd/monitor-persistence-policy-sdd.md), [presentation] (../../../sdd/monitor-presentation-contract-sdd.md), [core] (../../../sdd/core-completeness-sdd.md). |
| **orders** | OrderIntent validado por risk antes do port; fail-closed default; paper ledger/idempotência/reconciliação; Testnet opt-in dev; prod desabilitado. | backend/src/modules/orders/ tem submit/paper/recording/Testnet, PG stores/poll. G2 SDD marca Testnet/reconciliação parcial e Critic/threat model pendentes. | **P1:** PG idempotência/reconciliação isolada; então submit mínimo Testnet opt-in e redação/ambiguidade/async review. **P2:** prod não especificada/autorizada. Depende de risk/exchanges/credentials/DB/isolation; TDD replay/errors offline, depois G4 Testnet. [orders] (../../../sdd/orders-module-sdd.md), [G2] (../../../sdd/orders-live-execution-gate2-sdd.md), [ambiguous claim] (../../../sdd/wave0-12-order-ambiguous-claim-sdd.md), [async] (../../../sdd/wave0-13-orders-block-on-sdd.md). |
| **portfolio** | Evidência documentada limita-se a paper snapshot/positions alimentados por paper fills; não achei contrato de portfolio financeiro completo. | backend/src/modules/portfolio/ tem models/controllers e paper snapshot; auditoria anterior cita HTTP test histórico. | **P1:** provar paper submit→snapshot em dev. Persistir somente se requisito de sobreviver restart for decidido. **P2 decisão:** valuation, balances, fees, reconciliação, multi-moeda e ledger durável sem spec. Depende de orders; decisão primeiro, depois SDD/TDD. Sem SDD específico localizado. |
| **risk** | Limites/perfis e gate de sinais/intents; orders valida antes do executor. Risk não executa exchange/UI. | backend/src/modules/risk/ contém models/controllers/tests; Orders SDD exige validação pre-port. | **P1:** revalidar invariantes e que intenção inválida não chama executor. **P2 decisão:** política financeira de portfolio/volatilidade/gap/prod não definida. Depende de orders; TDD offline dos contratos atuais, SDD só para nova política decidida. |
| **strategy** | Avaliação determinística de candles fechados; SMA/EMA mencionados em runtime bots; Signal compartilhado, sem ordem. | backend/src/modules/strategy/ contém models/controllers; bots SDD registra evaluator no monitor/backtest. Testes da matriz não rodados agora. | **P1:** provar paridade monitor/backtest e binding por fixtures offline. **P2 decisão:** catálogo/versionamento/treino/performance além do atual não especificados. Depende de market/contracts; TDD fixtures e SDD só após decisão. |

## Cross-cutting core e presentation

| Área | Contrato/comportamento observado | Gaps e fatias |
|---|---|---|
| **core/config** | Parser/validação; prod recusa order execution (backend/src/core/config/mod.rs:381). T-DB-ENV requer resolver por ambiente e URL separada. | **P0:** G3 do alvo local + revisão; garantir todos callers pelo resolver. **P2:** prod sem endpoint/secret manager. [database environment SDD] (../../../sdd/database-environment-sdd.md). |
| **core/database/persistence** | Bootstrap PG + Neo4j opcional, migrations/transações/projeções em backend/src/core/database/. Sem execução PG comprovada aqui. | **P0:** zero SQL em mismatch, alvo dev identificado, runner isolado, backup antes de mutação/limpeza. Neo4j só com alvo local reconhecido. T-DB-ENV/T-W0-02b/T-W0-06. |
| **core/providers** | Provider clients e credential cache/CRUD. backend/src/core/providers/credentials/mod.rs declara encryption mode none/plaintext PG. | **P1 segurança:** W0-07 precisa de SDD/revisão, envelope encryption e migração antes de credenciais seguras. Sem secrets reais em logs/fixtures. [W0-07] (../../../sdd/wave0-07-credentials-at-rest-sdd.md). |
| **core/health/logging/notifications** | Liveness/readiness e LogNotifier em [core completeness SDD] (../../../sdd/core-completeness-sdd.md). | Métricas, SLI/SLO, alertas, dashboards/runbooks estão explicitamente fora do SDD atual. **P2 decisão:** requisitos operacionais antes de alegar prontidão prod. |
| **presentation/http** | Serve, routes, OpenAPI, bearer e ApiState. Código admin auth em backend/src/presentation/http/admin_auth.rs:96. | **P1:** smoke atual e rotas com DB depois dos gates; bearer não equivale IdP. Reconciliar docs antigos de auth com source atual. |
| **presentation/terminal** | TUI Ratatui consumidora de monitor. | Se TUI faz parte do aceite: aprovar contrato e TDD snapshot/commands/lag/shutdown. Para HTTP-first, priorizar após API dev. Depende de monitor. [presentation contract SDD] (../../../sdd/monitor-presentation-contract-sdd.md). |

## Application/CLI wiring e contratos neutros

| Contrato e critério baseado no desenho existente | Estado observado | Gap, dependência e fatia sugerida |
|---|---|---|
| Composition root escolhe comando, lê configuração e propaga Environment aos callers; módulos compartilham tipos neutros sem acoplar domínio a presentation. | `backend/src/main.rs:1-4,21-47,71-120` declara módulos/comandos e despacha serve, backtest, graph projection, graph e orders; `backend/src/modules/application_contracts.rs:1-30` define Signal/BotSignal sem política; `backend/src/modules/config_api.rs:1-3` reexporta tipos tipados para adapters. | **P1:** consolidar inventory dos callers transitivos do resolver DB e provar seletor Environment consistente para cada comando; CLI graph read, serve e subcomandos que ainda têm resolver próprio devem aparecer explicitamente nos acceptance tests. Dependências: T-DB-ENV/T-W0-02b. Fatia TDD offline com parse/dispatch públicos e fakes que afirmem que comando offline não conecta, comando persistente propaga o mesmo Environment e rejeições acontecem antes de efeito. **P2 decisão:** não foi localizado contrato aprovado para expandir a CLI, trocar formatos ou dar semântica de produto adicional a Signal/BotSignal/config_api.

## Reconciliação e decisões necessárias

- Auditoria prévia cobria bots/orders/agents/HTTP; esta amplia sem tratar seus testes datados como atuais.
- [T-16 module map] (../../../sdd/backend-module-map-sdd.md) é superseded e representa árvore anterior.
- [MVC status] (../../../architecture/module-implementation-status.md) separa MVC mínimo do goal de completude parcial; seus números são do snapshot 2026-09-27.
- Docs antigos dizem que admin routes ficam abertas sem token; source atual retorna erro de configuração ausente. Revalidar em gate permitido.
- T-16 relatou zero trades/Sell slippage ausente; [T-07] (../../../sdd/backtest-trades-and-slippage-sdd.md) registra correções C12/C13 e aprovação; são itens históricos, não gaps atuais confirmados.
- SDD draft/partial e existência de implementação são eixos diferentes; nenhum, isoladamente, atesta completude.

Decisões humanas: (1) dev é API+paper+persistência ou inclui Binance Testnet opt-in? (2) prod significa preparação/ops ou ordens reais? Live production permanece bloqueada. (3) qual IdP/owner verificável? (4) portfolio é paper ou contabilidade/reconciliação completa? (5) política risk além das regras atuais? (6) SLI/SLO, backups, retenção e runbook? Não inventar respostas.

## Sequência proposta

| Etapa | Entrega | Evidência de saída |
|---|---|---|
| 0 | Critic aprova T-W0-02b, T-DB-ENV, T-W0-06; preservar DB existente | Contratos vigentes revisados; nenhum efeito externo prematuro |
| 1 | G3/G4 PostgreSQL dev + test runner | mismatch rejeita antes de SQL; alvo identificado/descartável; log/exit codes registrados |
| 2 | HTTP dev offline | serve loopback, health/ready/OpenAPI/auth/404 e testes determinísticos observados |
| 3 | Fluxos dev persistidos | SDD e testes isolados para agents/bots/orders/monitor/backtest; Critic por fatia |
| 4 | Binance Spot Testnet opt-in | credenciais locais sem imprimir; submit/reconcile no alvo de teste; sem produção |
| 5 | Decisões e desenho operacional prod | IdP, encryption, SLO, backup, runbook/rollback e review; ordens live continuam bloqueadas |

## ENTREGA — matriz documental

- **Builder:** Orquestrador; síntese source/SDD; atualização exclusivamente por OpenKnowledge.
- **Artefato:** esta página substitui a matriz parcial anterior; história OpenKnowledge preserva revisão anterior.
- **Teste/evidência:** nenhum código/teste executado nesta entrega; evidências referenciadas são históricas e marcadas.
- **Revisão:** PENDENTE — Critic independente /root/test_safety_sdd_critic.
- **Riscos:** DB/test isolation pendentes, IdP e credenciais provider em plaintext, decisões de produto acima.
- **Veredito:** PENDENTE até revisão independente.
