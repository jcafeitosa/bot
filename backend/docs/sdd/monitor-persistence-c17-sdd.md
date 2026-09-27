---
title: SDD C17 fatia 1 — snapshot PG do supervisor do monitor
tags: [sdd, monitor, persistence, c17]
---

# C17 fatia 1 — metadata opcional do supervisor em PG

## Escopo

Singleton `monitor_supervisor_snapshot` (migração `0011_monitor_supervisor_snapshot.sql`): `last_tick_ms`, `promoted_bot_id` (opcional), `updated_at_ms`. **Não** é SoT de orders/agents/bots.

## Semântica

- **Wiring:** com `DATABASE_URL` → `trading_bot`, o monitor conecta/migra (best-effort; falha não impede boot).
- **Hydrate (boot):** se existir linha, log + entrada advisory na TUI; não altera promoção runtime nem reconciliação de ordens.
- **Save (pós-tick):** após ciclo de avaliação aplicado no loop do supervisor, `UPSERT` best-effort (erro → `warn`, monitor continua).

## Testes

- Unit: `snapshot_row_equality_is_fieldwise`, SQL da migração.
- PG: `pg_monitor_supervisor_snapshot_round_trip` (manifesto **28**).
