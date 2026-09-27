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

- **Estado:** **parcial** — G1 + seam `BotRuntimePort` (`FailClosedBotRuntime` default; `InMemoryBotRuntime` com `BOT_RUNTIME_ENABLED=true`), HTTP `GET /bots/runtime/status`, `POST .../promote|demote` (admin), `shared_bot_runtime()` + enrich em `MonitorHandle::publish_snapshot`, publicação headless via `monitor_snapshot_from_dashboard` no supervisor. **Parcial (loop):** `MonitorStrategyRegistry` resolve SMA por `strategy@version` (default `sma-cross@1` do config); `strategy_evaluation_binding` + `run_evaluation_cycle` propagam `promoted_bot_id` em `BotSignal`; catálogo HTTP expõe `monitor_fast_period`/`monitor_slow_period`. **Parcial (auth seam):** com `BOT_HTTP_AGENCY_ID`, `promoted_by` exige agente ativo com `promote_runtime_bot` (capability persistida em PG via migração `0005`). **Pendente:** evaluators não-SMA; auth owner produto.
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

Evidência parcial (2026-09-27): **298** testes bin `bot`, **6** ignorados; `shared_bot_runtime`, `enrich_monitor_snapshot_from_shared_runtime` em `MonitorHandle::publish_snapshot`, HTTP `/bots/runtime/*` com `assert_bot_promotion_allowed` (catálogo + mercado), `MonitorStrategyRegistry` + `monitor_strategy_from_config`, `strategy_evaluation_binding`; catálogo HTTP expõe `monitor_fast_period` / `monitor_slow_period` + `BotSignal.bot_id`, testes `bots_runtime_promote_*` (incl. `bots_runtime_promote_monitor_registry_v2_bot_in_catalog` para `sma-cross@2`), `meta_and_bot_runtime_status_agree_on_runtime_enabled`, com `BOT_HTTP_AGENCY_ID` e `fresh_agent_registry()` em `server.rs`.

## Testes HTTP (isolamento)

Testes de `POST /bots/runtime/promote` em `presentation/http/server.rs` usam `fresh_agent_registry()` (registry por teste) e `bot_id` alinhado ao `Config::default()` (`15m`, símbolo compacto `BTCUSDT`); `bots_runtime_promote_rejects_timeframe_not_matching_monitor_config` cobre rejeição de timeframe divergente.

## Validação Gate 2 (quando implementado)

- Testes de promoção/demote sem rede; `MonitorHandle::publish_snapshot` enriquece `MonitorSnapshot` via `shared_bot_runtime()`; HTTP espelha campos no snapshot; supervisor usa `strategy_evaluation_binding` + `BotSignal.bot_id` quando promoção casa com mercado; `promote_bot_http` rejeita `bot_id` fora do catálogo (`assert_catalog_contains_bot`). **Pendente:** parâmetros SMA/estratégia por definição de catálogo (hoje permanecem no `Config` global).
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
