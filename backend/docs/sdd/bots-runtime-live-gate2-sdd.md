---
title: SDD — Gate 2 runtime bots (executor live)
description: Promoção e execução de bots versionados no monitor; pós-fundação catálogo/PG
tags:
  - sdd
  - backend
  - bots
  - monitor
status: draft
---

# SDD — Gate 2: runtime bots (executor live)

- **Estado:** **não implementado** — G1 entregue (catálogo, ranking, `BotCatalogBackend`, HTTP `/api/v1/bots/*`, `PgBotCatalogStore` opcional). Monitor e estratégia operam sinais sem promover/executar bot versionado como runtime dedicado.
- **Referências:** [SDD bots G1](./bots-module-sdd.md), [Gate 1 PG](./bots-catalog-persistence-gate1-sdd.md), [auditoria](../planning/modules-completeness-audit.md), `modules/monitor`, `modules/agents` (governança).

## Contexto

`BotId` identifica executor `strategy@version:timeframe:symbol`. Backtest e ranking avaliam métricas offline. Não há loop que selecione um bot do catálogo, valide autorização via agentes e acople ao supervisor do monitor.

## Objetivo (quando aprovado)

1. Seam `BotRuntimePort` (ou equivalente) entre monitor/supervisor e instância de bot versionada.
2. Política de promoção (manual via HTTP/admin ou regras documentadas) com auditoria.
3. Mapeamento formal **agente autorizador → bot(s)** antes de ativar runtime (não inferir de `AgentRegistry::register`).
4. Fail-closed: sem promoção válida, monitor permanece no comportamento atual (estratégia/config global).

## Não-objetivos

- Fundir `modules/agents` com `modules/bots`.
- Enviar ordens reais sem [Gate 2 orders](./orders-live-execution-gate2-sdd.md) aprovado e wired.
- Promoção automática para produção sem revisão de segurança.

## Seams públicos (propostos)

| Símbolo | Contrato |
|---------|----------|
| `BotRuntimeHandle` | Instância ativa de um `BotId` no processo monitor. |
| `BotPromotionRecord` | Quem promoveu, quando, métricas de referência, estado (active/paused). |
| HTTP (opcional) | Rotas de promoção/status sob mesmo seam admin que mutações atuais. |

## Validação (baseline G1)

```text
./scripts/verify-backend-gates.sh
```

Evidência G1 (2026-09-27): **208** testes bin `bot`, **5** ignorados; `modules/bots/tests.rs`, HTTP bots catalog/persist, `PgBotCatalogStore` teste `#[ignore]`.

## Validação Gate 2 (quando implementado)

- Testes de promoção/demote sem rede; integração com `MonitorHandle` em processo.
- Nenhuma ordem real sem executor orders Gate 2.
- Revisão Critic + SDD agents (autorização).
- `./scripts/verify-backend-gates.sh` verde.

## Rollout / rollback

- Flag `bots.runtime_enabled` default `false`.
- Rollback: desativar promoções e voltar ao supervisor monolítico atual.

## Pendências de decisão

- Promoção por API vs apenas CLI/TUI.
- Persistência de promoção (PG vs memória).
- Relação com `MonitorAgentHook` existente.
