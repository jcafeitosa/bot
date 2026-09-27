---
title: SDD — HTTP admin bearer seam (fail-closed)
description: BOT_HTTP_ADMIN_TOKEN, BOT_HTTP_OWNER_ID e BOT_HTTP_AGENCY_ID; não substitui Gate 1 owner auth
tags:
  - sdd
  - backend
  - security
  - http
status: draft
---

# SDD — HTTP admin bearer seam

## Contexto

Rotas HTTP mutantes (agents lifecycle, bots catalog persist, bots runtime promote/demote, orders submit, monitor commands) precisam de um controle mínimo em ambientes expostos, sem implementar bootstrap verificável do owner (Gate 1 bloqueado na pesquisa).

## Comportamento

| Variável | Efeito |
|----------|--------|
| `BOT_HTTP_ADMIN_TOKEN` ausente/vazio | Sem exigência de bearer (comportamento dev/local). |
| `BOT_HTTP_ADMIN_TOKEN` definido | Rotas mutantes listadas exigem `Authorization: Bearer <token>`; falha → **401**. |
| `BOT_HTTP_OWNER_ID` definido (com token) | `POST /api/v1/agents` exige `owner_id` igual; falha → **403** `owner_mismatch`. |
| `BOT_HTTP_AGENCY_ID` definido | Rotas `/api/v1/agents*` exigem `agency` igual (query ou body); falha → **403** `http_agency_mismatch`. |

Implementação: `presentation/http/admin_auth.rs`, `ApiState::require_http_admin`, `require_register_owner_id`, `require_bound_agency` (todas as rotas em `routes/agents.rs`).

## Fora de escopo

- Prova de identidade do owner humano, bootstrap único (binding de agência por env é seam, não prova de tenant).
- Proteção de rotas de simulação (`risk/*`, `backtest/*`) — permanecem abertas quando admin token ativo.

## Validação

- Testes unitários `admin_auth.rs`.
- Testes HTTP `server.rs`: bearer obrigatório (agents register, bots persist, orders submit, monitor commands), `owner_mismatch`, `agents_list_rejects_agency_mismatch_when_bound`, `agents_register_rejects_agency_mismatch_when_bound`.
- `./scripts/verify-backend-gates.sh`.
