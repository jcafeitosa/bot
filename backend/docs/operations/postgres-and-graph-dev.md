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
| PostgreSQL 18+ | `timescale/timescaledb-ha` em `docker-compose.bot.yml` |
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

Testes `#[ignore]` de PG: `cd backend && ./scripts/run-pg-integration-tests.sh` com `DATABASE_URL` apontando para `trading_bot`.

## Provider credentials (LLM API keys)

Após `0007_provider_credentials.sql` (e nota `0008`), chaves ficam na tabela `provider_credentials` — **não** commitar segredos no repositório.

1. Aplique migrações (`DATABASE_URL` → `trading_bot`; boot HTTP ou `Database::migrate`).
2. Copie `backend/scripts/seed-provider-credentials.example.sql` para um arquivo local, substitua os placeholders (`<TYPESAFE_API_KEY>`, etc.) e execute com `psql` ou cliente SQL.
3. Reinicie o processo HTTP (ou reconecte PG) para `reload_from_pool` popular o cache.

Colunas: `provider_id` (`typesafe`, `openai`, `nvidia`, `ngc`), `key_name` (`api_key`), `secret` (texto; nunca logar), `updated_at` (TIMESTAMPTZ). Bootstrap por env (`TYPESAFE_API_KEY`, …) permanece **deprecated** quando a tabela está vazia.

CRUD HTTP admin (`/api/v1/admin/provider-credentials*`) retorna **501** fail-closed; use SQL ou rotação ops até Gate 1 — ver [provider-credentials-db-sdd](../sdd/provider-credentials-db-sdd.md).

## Neo4j (opcional)

```text
BOT_AGENTS_ENABLED=true
BOT_NEO4J_URI=bolt://127.0.0.1:7688
BOT_NEO4J_USER=neo4j
BOT_NEO4J_PASSWORD=<local-only>
```

Verificar extensões:

```sql
SELECT extname FROM pg_extension WHERE extname IN ('timescaledb', 'vector');
```
