---
title: SDD — integração unificada de módulos com PostgreSQL + Neo4j
status: accepted
---

# Integração `AppDatabases`

Entrypoints: `bootstrap_runtime` (HTTP), `bootstrap_monitor_postgres` (TUI/monitor), `postgres_for_cli_persist` (backtest `--persist`).

Config: `core::config::database` + `system.toml`. Neo4j só com agents stack habilitado.

Validação: `./scripts/verify-backend-gates.sh`, `./scripts/run-pg-integration-tests.sh`.
