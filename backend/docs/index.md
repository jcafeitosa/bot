---
title: Documentação do backend
description: Índice da documentação técnica, operacional e de design do backend
tags:
  - backend
  - documentation
  - index
---
# Documentação do backend

Esta é a entrada principal da documentação do backend. Use as referências por tipo para encontrar operação, configuração, pesquisa, planejamento e design técnico.

## Comece aqui

- [Guia de execução e operação](./operations/runbook.md) — pré-requisitos, inicialização, modos suportados, persistência, diagnóstico e `./scripts/verify-backend-gates.sh`.
- [PostgreSQL e grafo (dev)](./operations/postgres-and-graph-dev.md) — `DATABASE_URL`, PG **21/21**, retenção orders G2 ([cli-and-config § retention](./reference/cli-and-config.md#pg-orders-retention-gate-2)).
- [Referência de CLI e configuração](./reference/cli-and-config.md) — comandos, opções, variáveis de ambiente (`backend/.env.example`), validações e [verificação local (gates)](./reference/cli-and-config.md#verificação-local-gates).
- [API HTTP (OpenAPI + Scalar)](./reference/cli-and-config.md#subcomando-serve-http) — subcomando `serve`, `/openapi.json`, `/docs`, `GET /meta` (`http_seams`), agents e `--with-monitor`.
- [Referência de módulos do backend](./architecture/backend-module-reference.md) — visão consolidada, interfaces, seams, fluxos e limites.
- [Catálogo completo de módulos](./architecture/module-catalog.md) — todos os módulos Rust, facades HTTP (`§3d` `http_bridge`), submódulos de exchange, contratos e invariantes.
- [Matriz de testes](./reference/test-matrix.md) — cobertura por módulo, integração, evidências e lacunas (incl. [runtime bots vs `serve` (G2)](./reference/test-matrix.md#bot-runtime-no-serve-vs-testes-http-g2-parcial)).
- [Mapeamento de camadas](./architecture/layer-mapping.md) — domain, application, infrastructure, presentation.
- [Integrações do backend](./architecture/integrations.md) — Binance, ccxt, PostgreSQL, Jev, terminal e controles.
- [Estratégia Neo4j unificado (dual-store)](./architecture/unified-neo4j-graph-strategy.md) — PG SoT transacional + Neo4j grafo complementar pareado em produção alvo.
- [Estado atual e planejamento](./planning/current-state-and-roadmap.md) — feito, pendências, bloqueios, gates e roadmap.
- [Auditoria de completude de módulos](./planning/modules-completeness-audit.md) — bots/orders/agents/HTTP; baseline **469**/**0** ignored; PG **21/21**; goal **parcial** (auth owner + Critic).
- README do backend — visão geral, arquitetura e comandos rápidos.

## Pesquisa

- [Pesquisa de capacidades para o módulo `agents`](./research/agents-capability-research.md) — comparação de referências e limites para a primeira etapa `IdentityOnly`.
- [Análise de capacidades do módulo `org`](./research/org-module-capability-analysis.md) — escopo provisório de organizações, cargos, posições, ocupantes e governança; ainda não é SDD aprovado.

## Fontes preservadas

- [Grok Bot — overview](./external-sources/grok-bot-overview.md)
- [Grok Bot — teams and enterprises](./external-sources/grok-bot-teams-and-enterprises.md)
- [OpenBot — README](./external-sources/openbot-readme.md)
- [OpenClaw — Security](./external-sources/openclaw-security.md)
- [Meta Model API — overview](./external-sources/meta-model-api-overview.md)

## Planejamento

- [Auditoria de completude — bots, orders, agents, HTTP](./planning/modules-completeness-audit.md) — snapshot JSON: [modules-completeness-evidence.json](./planning/modules-completeness-evidence.json) — baseline `./scripts/verify-backend-gates.sh` (**469** passed, **0** ignored; PG **21/21** no CI `postgres-integration` ou local com `DATABASE_URL`); [ENTREGA G4 Builder](./planning/modules-completeness-audit.md#entrega-pacote-completude-módulos--g4-builder) (**PENDENTE** Critic); [pacote Critic (handoff)](./planning/modules-completeness-audit.md#pacote-para-revisão-critic-handoff); checklists [agents G1](./sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist), [orders G2](./sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](./sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist)
- [Status de implementação MVC mínimo](./architecture/module-implementation-status.md) — checklist `core`/`modules`/`presentation` e veredito vs goal de completude
- [Plano de execução das correções pendentes do backend](./planning/backend-work-plan.md) — sequência de entregas, gates, dependências e bloqueios atuais.
- [Errata normativa de autoridade do plano org](./planning/org-plan-authority-errata.md) — grants e `revocation_epoch` têm SoT PostgreSQL `org`; projections não autorizam.
- [Análise de módulos ainda não desenvolvidos](./planning/unimplemented-modules-analysis.md) — capacidades previstas sem implementação completa, dependências e ordem recomendada.
- [Auditoria completa de docs](./planning/docs-audit.md) — estrutura canônica, duplicidades, fontes preservadas e estado do grafo.

## Design técnico

- [SDD — Contrato de apresentação do monitor e tolerância zero](./sdd/monitor-presentation-contract-sdd.md) — snapshot completo, recuperação após lag e shutdown verificável.

- [Proposta 0001 — Separação entre core, módulos e MVC no backend](./proposals/0001-backend-core-modules-mvc.md) — proposta `draft`; seam TUI–monitor confirmado pelo owner; aguardando revisão e decisão humana.
- [SDD — Correções de configuração, mercado e organização do backend](./sdd/backend-corrections-sdd.md)
- [SDD T-05 — Restringir redirects REST do monitor Binance](./sdd/rest-redirect-sdd.md)
- [SDD T-07 — Fixture do backtest e custo da saída por sinal](./sdd/backtest-trades-and-slippage-sdd.md)
- [SDD T-10 — Pausa e retomada do monitor de candles](./sdd/monitor-pause-resume-sdd.md)
- [SDD T-13 — Remoção de arquivos legados de configuração e migração](./sdd/legacy-file-cleanup-sdd.md)
- [SDD T-15 — Política de persistência opcional do monitor](./sdd/monitor-persistence-policy-sdd.md)
- [SDD T-16 — Mapa de módulos do backend no README](./sdd/backend-module-map-sdd.md)
- [SDD — Core database (PG 18+ / Neo4j)](./sdd/core-database-sdd.md)
- [SDD — Gate 1 persistência catálogo bots](./sdd/bots-catalog-persistence-gate1-sdd.md) — draft PostgreSQL (`0002_agents_bots_scaffold.sql`).
- [SDD — Módulo bots (estratégia × timeframe)](./sdd/bots-module-sdd.md) — identidade versionada, catálogo, ranking e HTTP.
- [SDD — Módulo orders (fail-closed)](./sdd/orders-module-sdd.md) — validação de risco e port de execução bloqueado.
- [SDD — Gate 2 orders live (parcial)](./sdd/orders-live-execution-gate2-sdd.md) — `HttpOrderExecutor`, idempotência HTTP; adapter exchange real pendente.
- [SDD — Gate 2 bots runtime (parcial)](./sdd/bots-runtime-live-gate2-sdd.md) — `BotRuntimePort`, `MonitorEvaluatorKind` (SMA/EMA), promoção HTTP; auth owner pendente.
- [SDD — Módulo agents (fundação IdentityOnly)](./sdd/agents-module-sdd.md) — registry + espelhamento/hidratação PG, hierarquia, lifecycle e seam Jev; auth owner pendente.
- [SDD — Módulo `org`](./sdd/org-module-sdd.md) — desenho do sistema organizacional completo: unidades, cargos, posições, ocupações, autoridade e auditoria; draft aguardando revisão Critic e aprovação do owner, com mutações fail-closed até autenticação verificável.
- [SDD — Product owner bootstrap (G1 fatia)](./sdd/agents-owner-bootstrap-g1-sdd.md) — PG `0010`, env ACK, bind de registro; não substitui IdP.
- [SDD — HTTP admin bearer seam](./sdd/http-admin-auth-seam-sdd.md) — `BOT_HTTP_ADMIN_TOKEN`, binds opcionais de owner/agency; não substitui auth owner Gate 1.
- [SDD — Configuração centralizada](./sdd/centralized-config-sdd.md) — `system.toml`, `.env`, `bot.toml`, árvore `core/config/`.
- [SDD — Provider credentials (PostgreSQL)](./sdd/provider-credentials-db-sdd.md) — migração `0007`, cache em memória; `loads_credentials_from_postgres` no script PG **21/21**.
- [SDD — Outbox PG → Neo4j (F2.1)](./sdd/graph-projection-outbox-sdd.md) — migração `0009`, enqueue/drain best-effort.
- [SDD — GraphQueryPort read-only (F3)](./sdd/graph-query-port-f3-sdd.md) — `list_agents`, `supervision_chain`, `bots_for_agent`; CLI `graph query …`; não substitui PG como SoT.
- [SDD — Projeção Neo4j orders (F3)](./sdd/orders-neo4j-projection-sdd.md) — `OrderIntent` redigido; F3.1 aresta `SUBMITTED`.

## Governança

- [Instruções para agentes do projeto](./AGENTS.md) — regras de governança, fluxo, segurança e operação.

## Convenções

- Pesquisas ficam em `research/`.
- Planos e acompanhamento ficam em `planning/`.
- Runbooks ficam em `operations/`.
- Referências estáveis ficam em `reference/`.
- SDDs e designs técnicos ficam em `sdd/`.
- O estado de cada entrega continua registrado no próprio documento; o plano consolida a ordem de execução e os bloqueios.
- Verificação de imports: `backend/scripts/check-import-direction.sh` — heurística MVC.
