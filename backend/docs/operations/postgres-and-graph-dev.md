---
title: PostgreSQL 18 + Neo4j (desenvolvimento)
tags:
  - operations
  - postgres
  - neo4j
---

# PostgreSQL 18+ e Neo4j — desenvolvimento

| Componente | Referência |
|------------|------------|
| PostgreSQL 18+ (mínimo imposto em runtime por `core/database/postgres.rs`, `MIN_SERVER_VERSION_NUM = 180000`) | `timescale/timescaledb-ha` (pinado por digest) em `docker-compose.bot.yml`; o CI usa `timescaledb-ha:pg16` — divergente (issue aberta) |
| TimescaleDB | migration `0000_extensions.sql` |
| pgvector | migration `0000_extensions.sql` |
| Neo4j | serviço `graph` no compose (Bolt `7688`) |

## DATABASE_URL (market / migrations)

Nome de banco exigido pelo código: **`trading_bot`**.

```text
DATABASE_URL=postgresql://USER:PASSWORD@127.0.0.1:5432/trading_bot
```

O compose local (`docker-compose.bot.yml`) usa por padrão o banco `bot_agents` na porta **55433**. Para migrações de mercado/agents/bots/orders, crie `trading_bot` no mesmo cluster:

```sql
CREATE DATABASE trading_bot OWNER bot_agents;
```

Testes de integração PG: `cd backend && ./scripts/run-pg-integration-tests.sh` com `DATABASE_URL` apontando para `trading_bot` (sem env, os mesmos testes passam com skip via `core/persistence/pg_integration.rs`).

Auditoria V18 (T-15 rollback/idempotência candles): `cd backend && ./scripts/pg-v18-monitor-persistence-audit.sh` com o mesmo `DATABASE_URL`.

### `postgres_scaffold_tables_exist_after_migrate` falha (tabela ausente)

Sintoma típico: `missing table order_idempotency_keys after migrate` (ou outra tabela do scaffold). O teste chama `Database::migrate()` em cada execução (`core/persistence/pg_integration.rs`); migrações em `src/core/database/migrations/` (`0004_order_idempotency_keys.sql`, `0009_graph_projection_outbox.sql`, etc.).

Causa usual: banco `trading_bot` reutilizado com `_sqlx_migrations` desalinhado. Se um `trading_bot` novo passa o manifesto, trate como reset de ambiente, não bug de migração no repo.

```bash
psql "postgresql://postgres:postgres@127.0.0.1:55433/postgres" -c "DROP DATABASE IF EXISTS trading_bot;"
psql "postgresql://postgres:postgres@127.0.0.1:55433/postgres" -c "CREATE DATABASE trading_bot OWNER bot_agents;"
export DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:55433/trading_bot
cd backend && ./scripts/run-pg-integration-tests.sh
```

## Provider credentials (LLM API keys)

Após `0007_provider_credentials.sql` (e nota `0008`), chaves ficam na tabela `provider_credentials` — **não** commitar segredos no repositório.

1. Aplique migrações (`DATABASE_URL` → `trading_bot`; boot HTTP ou `Database::migrate`).
2. Copie `backend/scripts/seed-provider-credentials.example.sql` para um arquivo local, substitua os placeholders (`<TYPESAFE_API_KEY>`, etc.) e execute com `psql` ou cliente SQL.
3. Reinicie o processo HTTP (ou reconecte PG) para `reload_from_pool` popular o cache.

Colunas: `provider_id` (`typesafe`, `openai`, `nvidia`, `ngc`), `key_name` (`api_key`), `secret` (texto; nunca logar), `updated_at` (TIMESTAMPTZ). Bootstrap por env (`TYPESAFE_API_KEY`, …) permanece **deprecated** quando a tabela está vazia.

CRUD HTTP admin (`/api/v1/admin/provider-credentials*`) disponível com PG + bearer admin; respostas mascaradas (nunca `secret` em claro). Alternativa ops: SQL seed — ver [provider-credentials-db-sdd](../sdd/provider-credentials-db-sdd.md).

## Neo4j (opcional)

Stack de grafo de produto (projeção write-only + leitura F3 advisory). Preferir `BOT_GRAPH_ENABLED=true`; legado equivalente: `BOT_AGENTS_ENABLED=true`.

```text
BOT_GRAPH_ENABLED=true
BOT_NEO4J_URI=bolt://127.0.0.1:7688
BOT_NEO4J_USER=neo4j
BOT_NEO4J_PASSWORD=<local-only>
```

Com `DATABASE_URL` → `trading_bot` e Neo4j wired, migração `0009` cria `graph_projection_outbox`. O HTTP `serve` inicia o worker **F2.1.2** (`spawn_graph_projection_outbox_worker`) quando intervalo > 0 (`neo4j.graph_projection_outbox_drain_secs` / `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS`, default 30; `0` desliga). Batch por tick: `BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_BATCH` (default 32).

### Outbox — operação F2.1.2 / F2.1.3

| Ação | Comando / endpoint |
|------|-------------------|
| Backlog no health | `GET /healthz` → `graph_projection_outbox` (`pending`, `retry`, `oldest_pending_age_secs`, `degraded`); `status: degraded` com fila relevante |
| Drain manual | `cargo run --locked -- graph-projection drain --limit 32` (JSON `processed` / `succeeded` / `failed`; exige PG + Neo4j) |
| Leitura CLI (F3) | `cargo run --locked -- graph query agents --limit 32`; subcomandos `supervision-chain`, `bots-for-agent`, `code-impact --module-path modules/orders` (só retorna entidades após `scripts/sync-code-graph-neo4j.sh`; sem push → `entities: []`) |
| Evidência no gate | `./scripts/verify-backend-gates.sh` chama `assert-completeness-evidence.sh` após os testes do bin `bot` |

SDD: [graph-projection-outbox-sdd](../sdd/graph-projection-outbox-sdd.md) (§6–7), [graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md). Runbook: [runbook.md](./runbook.md#grafo-de-produto--outbox-neo4j-f212--f213).

## Verificar extensões PostgreSQL

```sql
SELECT extname FROM pg_extension WHERE extname IN ('timescaledb', 'vector');
```
