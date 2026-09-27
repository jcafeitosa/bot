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

- [Guia de execução e operação](./operations/runbook.md) — pré-requisitos, inicialização, modos suportados, persistência e diagnóstico.
- [Referência de CLI e configuração](./reference/cli-and-config.md) — comandos, opções, variáveis de ambiente e validações.
- [Referência de módulos do backend](./architecture/backend-module-reference.md) — visão consolidada, interfaces, seams, fluxos e limites.
- [Catálogo completo de módulos](./architecture/module-catalog.md) — todos os módulos Rust, submódulos de exchange, contratos e invariantes.
- [Matriz de testes](./reference/test-matrix.md) — cobertura por módulo, integração, evidências e lacunas.
- [Integrações do backend](./architecture/integrations.md) — Binance, ccxt, PostgreSQL, Jev, terminal e controles.
- [Estado atual e planejamento](./planning/current-state-and-roadmap.md) — feito, pendências, bloqueios, gates e roadmap.
- README do backend — visão geral, arquitetura e comandos rápidos.

## Pesquisa

- [Pesquisa de capacidades para o módulo `agents`](./research/agents-capability-research.md) — comparação de referências e limites para a primeira etapa `IdentityOnly`.

## Fontes preservadas

- [Grok Bot — overview](./external-sources/grok-bot-overview.md)
- [Grok Bot — teams and enterprises](./external-sources/grok-bot-teams-and-enterprises.md)
- [OpenBot — README](./external-sources/openbot-readme.md)
- [OpenClaw — Security](./external-sources/openclaw-security.md)
- [Meta Model API — overview](./external-sources/meta-model-api-overview.md)

## Planejamento

- [Plano de execução das correções pendentes do backend](./planning/backend-work-plan.md) — sequência de entregas, gates, dependências e bloqueios atuais.
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

## Governança

- [Instruções para agentes do projeto](./AGENTS.md) — regras de governança, fluxo, segurança e operação.

## Convenções

- Pesquisas ficam em `research/`.
- Planos e acompanhamento ficam em `planning/`.
- Runbooks ficam em `operations/`.
- Referências estáveis ficam em `reference/`.
- SDDs e designs técnicos ficam em `sdd/`.
- O estado de cada entrega continua registrado no próprio documento; o plano consolida a ordem de execução e os bloqueios.
- [Verificação de imports](scripts/check-import-direction.sh) — heurística MVC
