---
title: SDD — Módulo bots (estratégia × timeframe)
description: Identidade versionada de executor simulado, catálogo strategy×timeframe, ranking completo reutilizando métricas de backtest
tags:
  - sdd
  - backend
  - bots
  - backtest
status: draft
---
# SDD — Módulo `modules/bots`

- **Estado:** G1 + seam Gate 2 parcial — catálogo/ranking/HTTP; `BotRuntimePort` + rotas `/bots/runtime/*`; `MonitorSnapshot` ganha promoção em `publish_snapshot` (`shared_bot_runtime`); headless `--with-monitor` publica via `monitor_snapshot_from_dashboard`; HTTP espelha campos — supervisor preenche `BotSignal.bot_id` via `strategy_evaluation_binding` quando promoção casa com mercado (SMA ainda do `Config`) — [Gate 2 runtime](./bots-runtime-live-gate2-sdd.md).
- **Referências:** [SDD agents — Relação com bots](./agents-module-sdd.md), [Pesquisa agents](../research/agents-capability-research.md), [Módulos não implementados §1b](../planning/unimplemented-modules-analysis.md), [Convenção MVC](./modules-mvc-convention-sdd.md).
- **Premissas:** Bots são **variações estratégia × timeframe** (e símbolo de mercado quando aplicável), versionados, avaliados por métricas de simulação. Não são identidades administrativas (`modules/agents`).

## 1. Contexto e objetivo

O produto distingue **agentes** (governança) de **bots** (executores versionados de trading). Até esta fatia, tipos `BotId`, `BotDefinition`, `BotMetrics` e `rank_bots` viviam em `modules/backtest` como chaves de experimento. O módulo `bots` torna esse domínio explícito: catálogo de combinações, identidade estável e **ranking completo** ordenado por PnL líquido.

**Objetivo:** MVC em `src/modules/bots/` com models/controllers/adapters; backtest delega simulação e consome tipos de `bots`; CLI/HTTP de backtest passam ranking pelo controller de bots.

### Relação com agents, backtest e monitor

| Dimensão | `modules/bots` | `modules/agents` | `modules/backtest` | `modules/monitor` |
|----------|----------------|------------------|--------------------|-------------------|
| **Propósito** | Executor versionado strategy×timeframe (+ símbolo) | Identidade administrativa | Motor SMA crossover offline | Loop live/paper |
| **Ranking** | `full_ranking` / `rank_bots` (domínio) | Não | `run_sma_crossover`, agregação `rank_strategies` | Não |
| **Persistência** | `BotCatalogStore` (memória/PG) | `PgAgentIdentityStore` | Resultados JSON/CLI | Sessão opcional |

**Mensagem:** `AgentRegistry::register` ≠ criar bot. `BotIdentity::bot_id()` compõe `strategy@version:timeframe:symbol` — mesma chave canônica usada na simulação, agora no domínio `bots`.

### Não objetivos

- Loop do monitor executando bot promovido, promoção automática ou ordens reais.
- Substituir ou fundir com `modules/agents`.

## 2. Convenção MVC

```text
modules/bots/
  mod.rs
  models/           # BotIdentity, BotId, BotDefinition, BotMetrics, ranking
  controllers/      # catalog, full_ranking
  adapters/         # BotCatalogStore (memória, PG)
```

## 3. Seams públicos

| Símbolo | Consumidor | Contrato |
|---------|------------|----------|
| `BotIdentity`, `BotId` | backtest, CLI, futuro HTTP bots | Timeframe ∈ `OperationMode::all_timeframes()`; símbolo `BASE/QUOTE` normalizado. |
| `BotDefinition` | backtest simulation | Valida timeframe para `OperationMode`. |
| `BotMetrics` | backtest report, ranking | Campos de identidade coerentes com `BotId`. |
| `build_catalog_from_config` | testes legados | Uma estratégia × timeframes do modo. |
| `build_catalog_from_monitor_registry` | HTTP `GET /bots/catalog`, persist | Uma entrada por (estratégia registrada × timeframe). |
| `MonitorStrategyRegistry` / `MonitorEvaluatorKind` | monitor + catálogo HTTP + backtest | `sma_cross` (default) ou `ema_cross` por `strategy@version`; supervisor e `run_sma_crossover` usam `evaluate_for_kind`. |
| `full_ranking` / `rank_bots` | backtest CLI, agregadores | Escopo único (window, dataset_hash, quote); ordenação PnL ↓, drawdown ↑, bot_id. |
| `BotCatalogStore` | HTTP, boot API | `InMemoryBotCatalogStore`, `PgBotCatalogStore`, `BotCatalogBackend`; HTTP persist/snapshot usa PG quando disponível. Auth owner Gate 1 pendente. |

## 4. Migração desde backtest

- **Movido para `bots/models`:** `StrategyId`, `StrategyVersion`, `BotId`, `BotIdentity`, `BotDefinition`, `EvaluationWindow`, `BotMetrics`, lógica de `rank_bots`, tipos `BotRankEntry`, `BotRankingReport`.
- **Permanece em `backtest/models`:** `StrategyDefinition`, `RunId`, `StrategyRanking`, `rank_strategies`, `DomainError` (simulação + agregação estratégia).
- **Compat:** `backtest` reexporta tipos de bot e `rank_bots` delegando a `bots::controllers::ranking`.

## 5. Validação e rollout

- `cargo fmt`, `clippy -D warnings`, `cargo test --locked`, `./scripts/check-import-direction.sh`.
- Rollback: reverter `pub mod bots` e restaurar tipos monolíticos em `backtest/models.rs` (commit único).

## 6. Pendências

- Evidência PG reproduzível: teste ignorado `pg_catalog_store_round_trip` (ver [Gate 1](./bots-catalog-persistence-gate1-sdd.md)).
- [Gate 2 runtime](./bots-runtime-live-gate2-sdd.md): checklist **Critérios de fechamento G2**; promoção HTTP + `evaluate_for_kind` (SMA/EMA) feitos; pendem auth owner, alinhamento runtime `serve` vs testes HTTP, Critic.
- Baseline: `./scripts/verify-backend-gates.sh` verde; bin `bot` **317** testes (`promoted_sma_cross_identity_uses_config_periods` + gates).
- Auth owner verificável no transporte (fora do seam `BOT_HTTP_*`).
