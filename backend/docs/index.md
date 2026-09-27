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
- [Referência de módulos do backend](./architecture/backend-module-reference.md) — inventário módulo a módulo, interfaces, seams, fluxos e testes.
- [Integrações do backend](./architecture/integrations.md) — Binance, ccxt, PostgreSQL, Jev, terminal e controles.
- [Estado atual e planejamento](./planning/current-state-and-roadmap.md) — feito, pendências, bloqueios, gates e roadmap.
- README do backend — visão geral, arquitetura e comandos rápidos.

## Pesquisa

- [Pesquisa de capacidades para o módulo `agents`](./research/agents-capability-research.md) — comparação de referências e limites para a primeira etapa `IdentityOnly`.

## Planejamento

- [Plano de execução das correções pendentes do backend](./planning/backend-work-plan.md) — sequência de entregas, gates, dependências e bloqueios atuais.

## Design técnico

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
