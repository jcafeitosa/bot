---
title: SDD — Módulo market (fase 3)
description: Migração de market.rs e market_feed.rs para modules/market com camadas Model e Service
tags:
  - sdd
  - backend
  - market
  - mvc
status: draft
---
# SDD — Módulo `modules::market` (fase 3)

- **Estado:** draft — implementação G3 desta fatia.
- **Referências:** [Convenção MVC](./modules-mvc-convention-sdd.md), [Proposta 0001](../proposals/0001-backend-core-modules-mvc.md), [Core F1](./core-extraction-phase1-sdd.md).
- **Gate G1:** convenção MVC já documentada; este mini-SDD cobre apenas o domínio market.

## Contexto

`market.rs` concentra tipos OHLCV, timeframes, datasets históricos e erros de validação. `market_feed.rs` implementa `HybridCandleFeed` (merge REST/WS, watermark de avaliação). O monitor (`app.rs`) e exchanges (`live`, `binance`) são consumidores; `core::persistence` grava `HistoricalDataset` (acoplamento legado F1).

Esta fatia move o código para `src/modules/market/` sem alterar semântica — a suíte existente permanece verde.

## Objetivo

1. `modules/market/models.rs` — conteúdo migrado de `src/market.rs`.
2. `modules/market/services.rs` — conteúdo migrado de `src/market_feed.rs` (`HybridCandleFeed`, `ws_matches_configured_timeframe`).
3. `modules/market/mod.rs` — reexports públicos estáveis.
4. Remover `src/market.rs` e `src/market_feed.rs`; atualizar imports do crate.
5. **Não** criar `adapters/` vazio (I/O de candles permanece em `exchanges` nesta fatia).

## Mapeamento MVC (esta entrega)

| Camada (convenção) | Arquivo F3 | Responsabilidade |
|--------------------|------------|------------------|
| **Model** | `models.rs` | `Candle`, `Timeframe`, `HistoricalDataset`, `MarketError`, validação e resample |
| **Controller / Service** | `services.rs` | `HybridCandleFeed`, política de ingestão REST/WS e gatilho de avaliação |
| **Adapter** | — | Omitido (REST/WS em `exchanges::*`) |

Subpastas `models/` e `controllers/` da convenção longa podem ser introduzidas em refino posterior; F3 usa arquivos planos para diff mínimo.

## Seams públicos

Consumidores devem importar via `crate::modules::market` (ou reexports abaixo):

| Tipo / função | Origem |
|---------------|--------|
| `BASE_TIMEFRAME_MS`, `Timeframe`, `Candle`, `DatasetManifest`, `HistoricalDataset`, `MarketError` | `models` |
| `HybridCandleFeed`, `ws_matches_configured_timeframe` | `services` |

**Não-objetivos:** mover `exchanges`, `app`, `persistence_health`, `monitor_startup`; extrair adapters REST/WS para dentro de market.

## Riscos

| Risco | Mitigação |
|-------|-----------|
| Imports espalhados `crate::market` | grep + atualização em `src/` e testes |
| `core::persistence` → market | Atualizar path; débito core→modules documentado em F1 |

## Critérios de aceite

1. `src/modules/market/{mod,models,services}.rs` existem com testes unitários preservados.
2. `main.rs` não declara `mod market` / `mod market_feed` na raiz.
3. `cargo fmt`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` passam.
4. Comportamento observável inalterado (TDD = suite verde).

## Rollback

Restaurar `market.rs` / `market_feed.rs` na raiz e reverter `modules/market` se gates falharem.

## Próxima fatia (F4 — monitor)

- `app.rs`, `monitor_startup`, `persistence_health` → `modules/monitor`.
- `ui` → `presentation/terminal`.
- Opcional: desacoplar `core::persistence` de `HistoricalDataset` via trait/port.
