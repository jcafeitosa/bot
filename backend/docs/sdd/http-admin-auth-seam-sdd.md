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

Rotas HTTP mutantes (agents lifecycle, bots catalog persist, bots runtime promote/demote, orders submit, `orders_reconciliation_poll` / `orders_reconciliation_poll_succeeds_with_admin_bearer_when_enabled`, monitor commands) precisam de um controle mínimo em ambientes expostos, sem implementar bootstrap verificável do owner (Gate 1 bloqueado na pesquisa).

## Comportamento

| Variável | Efeito |
|----------|--------|
| `BOT_HTTP_ADMIN_TOKEN` ausente/vazio | Sem exigência de bearer (comportamento dev/local). |
| `BOT_HTTP_ADMIN_TOKEN` definido | Rotas mutantes listadas exigem `Authorization: Bearer <token>`; falha → **401**. |
| `BOT_HTTP_OWNER_ID` definido (com token) | `POST /api/v1/agents` exige `owner_id` igual; falha → **403** `owner_mismatch`. |
| `BOT_HTTP_AGENCY_ID` definido | Rotas `/api/v1/agents*` exigem `agency` igual (query ou body); falha → **403** `http_agency_mismatch`. Com bind ativo, `POST /api/v1/bots/runtime/promote` também exige que `promoted_by` seja agente ativo da agência com capability `promote_runtime_bot` (`assert_runtime_promotion_authorized`). |

Rotas `/api/v1/admin/provider-credentials*` exigem o mesmo bearer quando o token está ativo, mas respondem **501** (CRUD não implementado; sem vazamento de `secret`) — ver [provider-credentials-db-sdd](./provider-credentials-db-sdd.md).

Implementação: `presentation/http/admin_auth.rs`, `ApiState::require_http_admin`, `require_register_owner_id`, `require_bound_agency` (rotas `routes/agents.rs`); promoção de bot em `ApiState::promote_bot_http`; stub em `routes/provider_credentials_admin.rs`.

## Fora de escopo

- Prova de identidade do owner humano, bootstrap único (binding de agência por env é seam, não prova de tenant). Itens **Não** do checklist [agents G1](./agents-module-sdd.md#critérios-de-fechamento-g1-checklist) permanecem bloqueadores do goal de completude ([auditoria](../planning/modules-completeness-audit.md)).
- Proteção de rotas de simulação (`risk/*`, `backtest/*`) — permanecem abertas quando admin token ativo.

## Observabilidade (read-only)

`GET /api/v1/meta` inclui `http_seams` read-only: `http_admin_auth_enabled` (token admin ativo), `http_owner_binding_active` / `http_agency_binding_active` (booleanos — não expõem IDs; espelham `BOT_HTTP_OWNER_ID` / `BOT_HTTP_AGENCY_ID`), `order_execution_mode` / `live_exchange_wired` (alinhados com `GET /orders/execution-status`), `bot_runtime_enabled` (alinhado com `GET /bots/runtime/status` → `runtime_enabled`). Não substitui auditoria de rotas mutantes.

## Validação

- Testes unitários `admin_auth.rs` (`binding_active_flags_reflect_env_bindings_without_leaking_ids`).
- Testes HTTP `server.rs`: `meta_*`, `meta_and_*`, `router_after_build_api_state_meta_agrees_with_http_seam_endpoints` (boot `build_api_state_for_http_serve`: `http_seams` ↔ runtime + orders execution-status), agency/owner mismatch, bots runtime capability, monitor commands, OpenAPI smoke.
- Testes HTTP `http_integration_tests.rs`: bearer obrigatório (agents register/pause, bots catalog persist, bots runtime promote, orders submit paper/admin, `orders_reconciliation_poll`); ver [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token).
- `state.rs` `state_tests`: `for_http_server_wires_process_wide_bot_runtime_like_serve` (mesmo `Arc` que `shared_bot_runtime()` / `HttpApiSeams::from_env`).
- `./scripts/verify-backend-gates.sh`.
