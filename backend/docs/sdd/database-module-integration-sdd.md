---
title: SDD — integração unificada de módulos com PostgreSQL + Neo4j
description: Bootstrap único AppDatabases, seams por entrypoint e rollback operacional
tags:
  - sdd
  - backend
  - database
  - integration
status: accepted
---

# SDD — integração de módulos com bancos de dados

Ver `AppDatabases::bootstrap_runtime`, `bootstrap_monitor_postgres`, `postgres_for_cli_persist` em `src/core/database/bundle.rs` e `monitor_bootstrap.rs`.

## Validação

- `./scripts/verify-backend-gates.sh`
- `./scripts/run-pg-integration-tests.sh`
