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
