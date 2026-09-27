---
title: SDD — Product owner bootstrap verificável (G1 fatia mínima)
description: Bootstrap durável em PostgreSQL + validação fail-closed no registro HTTP; não substitui IdP humano
tags:
  - sdd
  - backend
  - agents
  - security
status: partial
---

# SDD — Product owner bootstrap (G1 fatia mínima)

## Escopo desta fatia

- Migração `0010_product_owner_bootstrap.sql`: tabela singleton `product_owner_bootstrap` + eventos append-only.
- Env explícito: `BOT_PRODUCT_OWNER_BOOTSTRAP_ID` + `BOT_PRODUCT_OWNER_BOOTSTRAP_ACK=1|true|yes` (via `ProductOwnerBootstrapConfig::from_env`).
- Boot HTTP (`ApiState::build_api_state_for_http_serve`): `ensure_product_owner_bootstrapped` quando `DATABASE_URL` conecta; conflito env≠PG → erro logado, SoT PG carregada se existir.
- Registro HTTP: `verify_register_owner_id` exige `owner_id` igual ao owner bootstrapped quando presente (**403** `owner_mismatch`), além do seam `BOT_HTTP_*` ([http-admin-auth-seam-sdd](./http-admin-auth-seam-sdd.md)).
- Observabilidade: `GET /api/v1/meta` → `http_seams.product_owner_bootstrap_active` (booleano, sem vazar id).

## Fora de escopo (permanece pendente G1 produto)

- Autenticação verificável do **owner humano** (IdP, claims, sessão).
- Bootstrap sem operador explícito (ACK) ou via endpoint público.
- Revisão Critic independente + contrato público acordado.

## Validação

- Unitários: `register_owner.rs`, `ProductOwnerBootstrapConfig`, `pg_owner_bootstrap` (SQL embed + PG `pg_product_owner_bootstrap_idempotent_and_conflict_fail_closed`).
- HTTP: `agents_register_rejects_owner_mismatch_when_product_owner_verified`; `bots_runtime_promote_rejects_promoted_by_mismatch_when_product_owner_verified`; `meta_reports_product_owner_bootstrap_active_when_verified`; `promote_bot_http_rejects_owner_mismatch_when_product_owner_verified` (`state.rs`).
- `./scripts/verify-backend-gates.sh` verde (**469** bin `bot`); PG **21/21** via `run-pg-integration-tests.sh`.
