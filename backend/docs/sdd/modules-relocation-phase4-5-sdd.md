---
title: SDD — Relocação F4/F5/F6 parcial (monitor, presentation, domínios)
status: implemented
date: 2026-09-27
---
# SDD — Relocação F4/F5/F6 parcial

## Escopo

- **F4:** `app`, `monitor_startup`, `persistence_health` → `modules/monitor/{controllers,views}`; scaffold `MonitorHandle`/contrato em `views/presentation_contract.rs` + `controllers/handle.rs`.
- **Presentation:** `ui/` → `presentation/terminal/`.
- **F5:** `strategy`, `risk`, `portfolio`, `backtest` (+ CLI), `exchanges`, `jev` → `modules/*`.
- **F6 parcial:** `main.rs` declara apenas `core`, `domain`, `modules`, `presentation`.

## Fora de escopo (próximo turno)

- F2 `application_contracts` (`Signal` neutro).
- MVC em pastas `models/controllers/adapters` para strategy/risk/exchanges (hoje `mod.rs` plano ou `market` com `models`+`services`).
- TUI importar somente API pública do monitor (ainda usa `modules::strategy`/`risk`).

## Validação

`cargo fmt`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`.
