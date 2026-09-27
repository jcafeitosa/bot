---
title: SDD — Scaffold PostgreSQL agents/bots (sem wiring)
description: Migração 0002 com tabelas de identidade administrativa e catálogo de bots; sem auth live nem repositórios Rust
tags:
  - sdd
  - backend
  - persistence
  - agents
  - bots
status: draft
---

# SDD — Scaffold PostgreSQL `agents` / `bots`

- **Estado:** draft — fatia pós-objetivo literal; **não** habilita Gate 1 auth, **não** conecta `AgentRegistry` ou `BotCatalogStore` ao banco.
- **Referências:** [agents-module-sdd](./agents-module-sdd.md), [bots-module-sdd](./bots-module-sdd.md), [unimplemented-modules-analysis](../planning/unimplemented-modules-analysis.md) § V18 / Gate 1.

## 1. Contexto

Identidades de agentes e catálogo de bots vivem em memória. O backend já migra market data via `core::persistence::Database::migrate` no banco dedicado `trading_bot`. Antes de implementar repositórios e auth do owner, precisamos de **schema versionado** alinhado aos modelos Rust atuais, aplicável pela mesma migrator sqlx, sem alterar runtime HTTP/main.

## 2. Objetivo

Adicionar `0002_agents_bots_scaffold.sql` em `src/core/persistence/migrations/` com:

| Tabela | Propósito |
|--------|-----------|
| `agent_identities` | Snapshot de `AgentDefinition` (papéis, supervisor, lifecycle, `consult_jev`). |
| `agent_identity_events` | Append-only de `IdentityAuditEvent`. |
| `bot_catalog_entries` | Linhas de `BotDefinition` (chave `bot_id` canônica). |

## 3. Seams públicos (esta fatia)

| Seam | Comportamento |
|------|----------------|
| `Database::migrate` | Aplica 0001 + 0002 quando `DATABASE_URL` aponta para `trading_bot`. |
| Teste unitário `migration_scaffold_sql_declares_core_tables` | Falha se o SQL scaffold perder nomes de tabela esperados (sem PG). |
| Teste ignorado `postgres_scaffold_tables_exist_after_migrate` | Opcional em CI com PG; verifica `information_schema` pós-migrate. |

**Fora de escopo:** `PgAgentRegistry`, `PgBotCatalogStore`, rotas com auth, live trading, orders reais.

## 4. Mapeamento modelo → coluna

- `AgentRole` → `role` (`ceo`, `level_b`, `level_a`, `specialist`, `worker`).
- `SupervisorRef` → `supervisor_kind` + `supervisor_owner_id` / `supervisor_agent_id` (CHECK mutuamente exclusivo).
- `AgentLifecycleState` → `lifecycle_state` (`active`, `paused`, `retired`).
- `BotDefinition` → colunas explícitas + `operation_mode` (`hft`, `scalper`, `day_trader`, `swing_trader`).

## 5. Riscos e mitigação

| Risco | Mitigação |
|-------|-----------|
| Scaffold usado como “PG pronto” | Docs + status: schema only; memória continua fonte de verdade em runtime. |
| FK circular na hierarquia | Sem FK de supervisor → agent nesta fatia; Gate 1 pode endurecer constraints. |
| Divergência SQL vs Rust | Teste unitário sobre arquivo SQL; integração ignorada valida tabelas. |

## 6. Validação

- `cargo fmt`, `clippy -D warnings`, `cargo test --locked`, `./scripts/check-import-direction.sh`.
- Rollback: reverter migração 0002 e este SDD; 0001 inalterado.

## 7. Próximo gate

Implementar `BotCatalogStore` / repositório de agents sobre estas tabelas **após** revisão G1 de auth/bootstrap — ainda fail-closed para execução financeira.
