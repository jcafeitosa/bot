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

- **Estado:** draft G1 — implementação fundacional em memória; persistência Gate 1 pendente.
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
| **Persistência** | Trait `BotCatalogStore` (stub) | Gate 1 PostgreSQL | Resultados JSON/CLI | Sessão opcional |

**Mensagem:** `AgentRegistry::register` ≠ criar bot. `BotIdentity::bot_id()` compõe `strategy@version:timeframe:symbol` — mesma chave canônica usada na simulação, agora no domínio `bots`.

### Não objetivos

- Runtime live de executor, promoção automática ou ordens reais.
- Substituir ou fundir com `modules/agents`.
- Persistência PostgreSQL de catálogo (Gate 1).

## 2. Convenção MVC

```text
modules/bots/
  mod.rs
  models/           # BotIdentity, BotId, BotDefinition, BotMetrics, ranking
  controllers/      # catalog, full_ranking
  adapters/         # BotCatalogStore (stub)
```

## 3. Seams públicos

| Símbolo | Consumidor | Contrato |
|---------|------------|----------|
| `BotIdentity`, `BotId` | backtest, CLI, futuro HTTP bots | Timeframe ∈ `OperationMode::all_timeframes()`; símbolo `BASE/QUOTE` normalizado. |
| `BotDefinition` | backtest simulation | Valida timeframe para `OperationMode`. |
| `BotMetrics` | backtest report, ranking | Campos de identidade coerentes com `BotId`. |
| `build_catalog_from_config` | testes, futura API | Uma entrada por timeframe suportado no modo da config. |
| `full_ranking` / `rank_bots` | backtest CLI, agregadores | Escopo único (window, dataset_hash, quote); ordenação PnL ↓, drawdown ↑, bot_id. |
| `BotCatalogStore` | Gate 1 | Trait only; `NoopBotCatalogStore` e `InMemoryBotCatalogStore` (snapshot em processo); PostgreSQL Gate 1 pendente. |

## 4. Migração desde backtest

- **Movido para `bots/models`:** `StrategyId`, `StrategyVersion`, `BotId`, `BotIdentity`, `BotDefinition`, `EvaluationWindow`, `BotMetrics`, lógica de `rank_bots`, tipos `BotRankEntry`, `BotRankingReport`.
- **Permanece em `backtest/models`:** `StrategyDefinition`, `RunId`, `StrategyRanking`, `rank_strategies`, `DomainError` (simulação + agregação estratégia).
- **Compat:** `backtest` reexporta tipos de bot e `rank_bots` delegando a `bots::controllers::ranking`.

## 5. Validação e rollout

- `cargo fmt`, `clippy -D warnings`, `cargo test --locked`, `./scripts/check-import-direction.sh`.
- Rollback: reverter `pub mod bots` e restaurar tipos monolíticos em `backtest/models.rs` (commit único).

## 6. Pendências

- Gate 1: implementar `BotCatalogStore` com PostgreSQL.
- HTTP: catalog, catalog/persist, catalog/snapshot (store em ApiState), ranking; evoluir OpenAPI conforme novos campos.
- Mapeamento formal executor versionado ↔ agentes autorizadores.
