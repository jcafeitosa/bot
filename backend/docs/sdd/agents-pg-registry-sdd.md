---
title: SDD — PgAgentIdentityStore (registry PostgreSQL)
description: Read/write agent_identities + agent_identity_events; cold-start hydrate; fail-closed sem auth owner
tags:
  - sdd
  - backend
  - agents
  - persistence
status: draft
---

# SDD — `PgAgentIdentityStore` (identidade PG)

- **Estado:** draft — implementação em `modules/agents/adapters/pg_registry.rs`; runtime continua **fonte de verdade em memória** (`AgentRegistry`); PG é espelho durável + cold-start.
- **Referências:** [agents-module-sdd](./agents-module-sdd.md), [scaffold agents/bots](./persistence-agents-bots-scaffold-sdd.md), [Gate 1 agents/bots](./bots-catalog-persistence-gate1-sdd.md), [core database](./core-database-sdd.md).

## 1. Contexto

O módulo `agents` expõe `AgentRegistry` in-memory com auditoria append-only. Gate 1 exige schema versionado (`0002`, `0005`) e store PostgreSQL alinhado a `AgentDefinition` / `IdentityAuditEvent`, sem autenticação verificável do owner e sem live trading.

## 2. Objetivo

| Seam | Comportamento |
|------|----------------|
| `AgentIdentityStore` | `upsert_agent`, `append_event`, `load_snapshot` sobre `agent_identities` + `agent_identity_events`. |
| `PgAgentIdentityStore` | Implementação sqlx; mapeamento supervisor/lifecycle/role/event kind fail-closed em valores desconhecidos. |
| `write_through_agent_identity` | Best-effort após mutações HTTP quando `AppDatabases` tem PG; no-op sem pool. |
| `load_agent_identity_snapshot` + `apply_agent_identity_snapshot` | Boot `build_api_state_for_http_serve`: hidrata registry **somente se vazio**; warn em falha PG (processo continua). |
| `AgentRegistry::restore_from_snapshot` | Valida hierarquia antes de substituir mapa + audit trail. |

**Fora de escopo:** auth owner live, reconciliação contínua PG↔memória, substituir `AgentRegistry` in-memory por store PG direto.

## 3. Mapeamento coluna ↔ modelo

Igual ao [scaffold §4](./persistence-agents-bots-scaffold-sdd.md#4-mapeamento-modelo--coluna), mais `promote_runtime_bot` (migração `0005`).

## 4. Riscos

| Risco | Mitigação |
|-------|-----------|
| PG indisponível | Write-through ignora; boot warn; HTTP usa memória. |
| Registry já populado vs PG | `apply_agent_identity_snapshot` no-op se registry não vazio. |
| Divergência audit memória/PG | Snapshot carrega eventos PG ordenados por `event_id`; mutações futuras re-escrevem via write-through. |

## 5. Validação

- Gates: `./scripts/verify-backend-gates.sh`.
- Unit: `lifecycle_sql_round_trip`, supervisor column mapping, `apply_snapshot_*`, `register_agent_maps_promote_runtime_bot_capability`.
- Integração ignorada: `pg_identity_snapshot_round_trip`, `pg_agent_lifecycle_write_through_round_trip`, `pg_cold_start_apply_snapshot_after_write_through`, `pg_register_agent_and_persist_cold_start_via_snapshot` — `./scripts/run-pg-integration-tests.sh`.

## 6. Rollback

Remover hidratação em `build_api_state_for_http_serve` e write-through nas rotas agents; manter schema (dados PG órfãos inofensivos).

## 7. Próximo

Auth owner verificável (Gate 1 produto) e revisão Critic independente antes de `status: approved`.
