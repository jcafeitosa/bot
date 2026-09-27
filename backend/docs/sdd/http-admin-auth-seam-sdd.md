---
title: SDD — HTTP admin bearer seam (fail-closed)
description: BOT_HTTP_ADMIN_TOKEN e BOT_HTTP_OWNER_ID para rotas mutantes; não substitui Gate 1 owner auth
tags:
  - sdd
  - backend
  - security
  - http
status: draft
---

# SDD — HTTP admin bearer seam

## Contexto

Rotas HTTP mutantes (agents lifecycle, bots catalog persist, orders submit, monitor commands) precisam de um controle mínimo em ambientes expostos, sem implementar bootstrap verificável do owner (Gate 1 bloqueado na pesquisa).

## Comportamento

| Variável | Efeito |
|----------|--------|
| `BOT_HTTP_ADMIN_TOKEN` ausente/vazio | Sem exigência de bearer (comportamento dev/local). |
| `BOT_HTTP_ADMIN_TOKEN` definido | Rotas mutantes listadas exigem `Authorization: Bearer <token>`; falha → **401**. |
| `BOT_HTTP_OWNER_ID` definido (com token) | `POST /api/v1/agents` exige `owner_id` igual; falha → **403** `owner_mismatch`. |

Implementação: `presentation/http/admin_auth.rs`, `ApiState::require_http_admin`, `require_register_owner_id`.

## Fora de escopo

- Prova de identidade do owner humano, bootstrap único, autorização por agência no transporte.
- Proteção de rotas de simulação (`risk/*`, `backtest/*`) — permanecem abertas quando admin token ativo.

## Validação

- Testes unitários `admin_auth.rs`.
- Testes HTTP `server.rs`: bearer obrigatório (agents register, bots persist, orders submit, monitor commands), owner mismatch.
- `./scripts/verify-backend-gates.sh`.
