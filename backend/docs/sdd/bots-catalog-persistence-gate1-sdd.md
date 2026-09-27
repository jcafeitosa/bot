---
title: SDD — Gate 1 persistência do catálogo bots (PostgreSQL)
description: Schema, adapter PostgresBotCatalogStore e critérios de aceite após InMemoryBotCatalogStore
tags:
  - sdd
  - backend
  - bots
  - persistence
status: draft
---

# SDD — Gate 1: `BotCatalogStore` PostgreSQL

- **Estado:** draft — não implementado; `InMemoryBotCatalogStore` + HTTP persist/snapshot já existem.
- **Referências:** [SDD bots](./bots-module-sdd.md), migrações `src/core/persistence/migrations/`.

## Objetivo

Migração `0002_bot_catalog.sql`, `PostgresBotCatalogStore`, teste integração ignorado (`trading_bot`).

## Schema proposto

Tabela `bot_catalog_snapshot`: `id`, `config_hash`, `captured_at`, `payload` (JSONB de `BotDefinition[]`).

## Validação

`cargo test --locked` + `cargo test bot_catalog_store_round_trip -- --ignored` com `DATABASE_URL`.

## Rollback

Remover migração e adapter; HTTP permanece em memória.
