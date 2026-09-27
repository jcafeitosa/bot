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

- **Estado:** **parcial** — G1 + seam `BotRuntimePort` (`FailClosedBotRuntime` default; `InMemoryBotRuntime` com `BOT_RUNTIME_ENABLED=true`), HTTP `GET /bots/runtime/status`, `POST .../promote|demote` (admin), `shared_bot_runtime()` + enrich em `MonitorHandle::publish_snapshot`, publicação headless via `monitor_snapshot_from_dashboard` no supervisor. **Pendente:** loop do supervisor trocar estratégia/executor pelo `BotId` promovido e autorização agente→bot.
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
| `shared_bot_runtime()` | Singleton por processo (`OnceLock`); `HttpApiSeams::from_env` e futuros hooks do monitor compartilham promoção. |
| `BotPromotionRecord` | Quem promoveu, quando, métricas de referência, estado (active/paused). |
| HTTP (opcional) | Rotas de promoção/status sob mesmo seam admin que mutações atuais. |

## Validação (baseline G1)

```text
./scripts/verify-backend-gates.sh
```

Evidência parcial (2026-09-27): **232** testes bin `bot`, **5** ignorados; `shared_bot_runtime`, `enrich_monitor_snapshot_from_shared_runtime` em `MonitorHandle::publish_snapshot`, HTTP `/bots/runtime/*`, `bots_runtime_promote_http_visible_on_status_and_monitor_snapshot`, `apply_bot_runtime_to_monitor_snapshot_copies_promotion`.

## Validação Gate 2 (quando implementado)

- Testes de promoção/demote sem rede; `MonitorHandle::publish_snapshot` enriquece `MonitorSnapshot` via `shared_bot_runtime()`; HTTP espelha campos no snapshot. Estratégia do supervisor ainda não troca por `BotId` promovido.
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
