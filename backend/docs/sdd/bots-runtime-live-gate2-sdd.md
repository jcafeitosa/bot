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

- **Estado:** **parcial** — G1 + seam `BotRuntimePort` (`FailClosedBotRuntime` default; `InMemoryBotRuntime` com `BOT_RUNTIME_ENABLED=true`), HTTP `GET /bots/runtime/status`, `POST .../promote|demote` (admin), `shared_bot_runtime()` + enrich em `MonitorHandle::publish_snapshot`, publicação headless via `monitor_snapshot_from_dashboard` no supervisor. **Parcial (loop):** `MonitorStrategyRegistry` + `MonitorEvaluatorKind` (`sma_cross` default, `ema_cross` via `[[strategy.monitor_registry]]`); `strategy_evaluation_binding` + `run_evaluation_cycle` → `evaluate_for_kind`; catálogo HTTP expõe `monitor_fast_period`/`monitor_slow_period`/`monitor_evaluator`. **Parcial (auth seam):** com `BOT_HTTP_AGENCY_ID`, `promoted_by` exige agente ativo com `promote_runtime_bot` (capability persistida em PG via migração `0005`). **Pendente:** auth owner produto.
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

Evidência parcial (2026-09-27): **367** testes bin `bot`, **8** ignorados; `shared_bot_runtime`, `evaluate_for_kind` no supervisor e em `run_sma_crossover`, HTTP `/bots/runtime/*`, `MonitorEvaluatorKind` + catálogo `monitor_evaluator`, testes `strategy_evaluation_binding_uses_ema_evaluator_from_registry`, `ema_crossover_backtest_uses_strategy_evaluator`, `bots_runtime_promote_*`, `meta_and_bot_runtime_status_agree_on_runtime_enabled`.

## Testes HTTP (isolamento)

Testes de `POST /bots/runtime/promote` em `presentation/http/server.rs` usam `fresh_agent_registry()` (registry por teste) e `bot_id` alinhado ao `Config::default()` (`15m`, símbolo compacto `BTCUSDT`); `bots_runtime_promote_rejects_timeframe_not_matching_monitor_config` cobre rejeição de timeframe divergente. `state_tests::persist_catalog_then_promote_monitor_registry_v2_bot` valida promoção `sma-cross@2` + `strategy_evaluation_binding_with_runtime` (períodos 3/15).

## Validação Gate 2 (quando implementado)

- Testes de promoção/demote sem rede; supervisor/backtest usam `evaluate_for_kind` com `MonitorEvaluatorKind`; catálogo HTTP expõe `monitor_evaluator`. **Pendente:** alinhar runtime injetado em testes HTTP com `shared_bot_runtime()` do `serve`.
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

## Critérios de fechamento G2 (checklist)

| Critério | Evidência atual | Fechado |
|----------|-----------------|--------|
| `BotRuntimePort` + `BOT_RUNTIME_ENABLED` | `InMemoryBotRuntime`, `shared_bot_runtime` | Sim (seam) |
| HTTP promote/demote + catálogo | `assert_bot_promotion_allowed`, testes `bots_runtime_*` | Sim |
| `strategy_evaluation_binding` + `BotSignal.bot_id` | `evaluation_binding.rs`, supervisor | Sim |
| `MonitorEvaluatorKind` (SMA/EMA) monitor + backtest | `evaluate_for_kind`, catálogo/config HTTP | Sim |
| Capability `promote_runtime_bot` com agency bind | `bot_promotion.rs`, migração `0005` | Sim (seam) |
| Auth owner produto (não só `BOT_HTTP_*`) | [pesquisa agents](../research/agents-capability-research.md) | **Não** |
| Runtime injetado em testes HTTP = `serve` production | `HttpApiSeams::from_env` + `shared_bot_runtime()`; `from_env_shares_process_wide_bot_runtime_with_serve`; `for_http_server_wires_process_wide_bot_runtime_like_serve` | **Parcial** (vários testes HTTP ainda usam `InMemoryBotRuntime` isolado por isolamento) |
| Ordens reais / exchange | Monitor+HTTP: paper/recording/testnet ([orders G2](./orders-live-execution-gate2-sdd.md)); prod REST bloqueado | **Parcial** |
| Revisão Critic | — | **Não** |
| `./scripts/verify-backend-gates.sh` verde | **367** testes bin `bot` (2026-09-27) | Sim (baseline parcial) |
