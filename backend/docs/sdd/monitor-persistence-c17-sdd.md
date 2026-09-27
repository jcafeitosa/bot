---
title: SDD C17 — persistência do monitor (fatias)
tags: [sdd, monitor, persistence, c17]
---

# C17 — persistência do monitor

## Fatia 1 — metadata opcional do supervisor em PG

### Escopo

Singleton `monitor_supervisor_snapshot` (migração `0011_monitor_supervisor_snapshot.sql`): `last_tick_ms`, `promoted_bot_id` (opcional), `updated_at_ms`. **Não** é SoT de orders/agents/bots.

### Semântica

- **Wiring:** com `DATABASE_URL` → `trading_bot`, o monitor conecta/migra (best-effort; falha não impede boot).
- **Hydrate (boot):** se existir linha, log + entrada advisory na TUI; não altera promoção runtime nem reconciliação de ordens.
- **Save (pós-tick):** após ciclo de avaliação aplicado no loop do supervisor, `UPSERT` best-effort (erro → `warn`, monitor continua).

### Testes

- Unit: `snapshot_row_equality_is_fieldwise`, SQL da migração.
- PG: `pg_monitor_supervisor_snapshot_round_trip` (manifesto **28**).

## Fatia 2 — TUI/presentation + REST do estado de arquivo (T-15 runtime)

### Escopo

Estado de sessão `OFF` / `HEALTHY` / `DEGRADED` / `GAP` já mantido por `PersistenceHealth` no loop do supervisor (`apply_ws_losses`, janela REST contígua, fail-closed em opt-out). Esta fatia **expõe** esse estado na fronteira de apresentação e no `GET /api/v1/monitor/snapshot`, sem nova lógica de trading ou gravação.

### Semântica

- **TUI:** rótulos completos em `Dashboard.persistence_status` (strings `HEALTHY · …`, `DEGRADED · …`, `GAP · …`, `OFF`) — inalterado.
- **Contrato `MonitorSnapshot`:** `PersistenceStatus` enum fechado `Healthy | Degraded | Gap | Unavailable` (`Unavailable` = opt-out / rótulo desconhecido, fail-closed).
- **Mapeamento:** prefixo `GAP` → `Gap` (não colapsar em `Degraded`); prefixo `DEGRADED` → `Degraded`; prefixo `HEALTHY` → `Healthy`; demais → `Unavailable`.
- **REST:** campo `persistence_status` em `MonitorSnapshotResponse` (`healthy` | `degraded` | `gap` | `unavailable`), derivado do watch publicado pelo supervisor headless/TUI via `monitor_snapshot_from_dashboard`.
- **Overflow WS / recuperação REST:** comportamento runtime conforme [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) § C17; fatia 2 não duplica testes de loop — apenas garante que transições visíveis na TUI chegam ao snapshot HTTP.

### Testes (unit)

- `monitor_snapshot_maps_gap_persistence_label`
- `monitor_snapshot_maps_initial_degraded_persistence_label`
- `monitor_snapshot_maps_unknown_persistence_label_to_unavailable`
- `snapshot_from_domain_exposes_persistence_status_for_rest`

### Fora de escopo (fatia 2)

- V18 PostgreSQL isolado; métricas/SLO; variantes extras no OpenAPI além do campo acima; persistir estado de arquivo em PG.
