---
title: Documentação do backend
description: Índice da documentação técnica, pesquisas, planos e SDDs do backend
tags:
  - backend
  - documentation
  - index
---
# Documentação do backend

Esta é a entrada principal da documentação do backend. Os documentos estão agrupados por função: pesquisa, planejamento e design técnico.

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

## Convenções

- Pesquisas ficam em `research/`.
- Planos e acompanhamento ficam em `planning/`.
- SDDs e designs técnicos ficam em `sdd/`.
- O estado de cada entrega continua registrado no próprio documento; o plano consolida a ordem de execução e os bloqueios.
