# SDD: Provider credentials in PostgreSQL

## Source of truth

- **Primary:** table `provider_credentials` in `trading_bot` (migration `0007_provider_credentials.sql`).
- **Bootstrap/dev (deprecated):** `.env` keys `TYPESAFE_API_KEY`, `OPENAI_API_KEY`, `NVIDIA_API_KEY`, `NGC_API_KEY` when cache is empty.
- **Non-secrets:** endpoints/URLs remain `system.toml` + env override (`TYPESAFE_ENDPOINT`, `NINE_ROUTER_BASE_URL`, etc.).

## Schema

| Column | Type | Notes |
|--------|------|-------|
| provider_id | TEXT PK | `typesafe`, `openai`, `nvidia`, `ngc` |
| key_name | TEXT PK | `api_key` (v1) |
| secret | TEXT | never logged; plaintext until encryption ADR (`provider_credentials_encryption=none` on meta/healthz) |
| updated_at | TIMESTAMPTZ | default now() |

## Public seam

- `core::providers::credentials::reload_from_pool(&PgPool)` — after migrations.
- `core::providers::credentials::lookup_secret(provider_id, key_name, env_fallback)` — cache then deprecated env.
- Wired from `core/config/providers/file.rs` (`resolve_bearer_api_key`, `resolve_nvidia_api_key`).

## Fail-closed

- Missing DB row and empty env → `BotError::Configuration` (Jev/NIM clients do not start).
- Reload errors after migrate → warn log; cache may stay empty until next connect.
- **Encryption-at-rest (v1):** not implemented. `core::providers::credentials::PROVIDER_CREDENTIALS_ENCRYPTION_MODE` is always `"none"`; HTTP exposes the same value on `http_seams.provider_credentials_encryption` and `/healthz` so operators never assume ciphertext in PostgreSQL. Future ADR must bump this seam before writing encrypted blobs.

## Encryption-at-rest (follow-up, not this slice)

- No application-level crypto in v1; no new dependencies.
- When implemented: ADR, migration strategy for existing rows, and seam value ≠ `none` only after encoder is wired on read/write paths.

## Rotation

- Update row in `provider_credentials`; restart process or call `reload_from_pool`.

## Dev seed (ops)

- Template: `backend/scripts/seed-provider-credentials.example.sql` (placeholders only).
- Migração `0008_provider_credentials_dev_seed_note.sql` — sem INSERT automático.
- Procedimento: [postgres-and-graph-dev.md](../operations/postgres-and-graph-dev.md#provider-credentials-llm-api-keys).

## HTTP admin CRUD (parcial, fail-closed)

> **Auth:** "fail-closed" aqui vale só para o **503** sem PG. O bearer é exigido apenas quando `BOT_HTTP_ADMIN_TOKEN` está definido; sem token, listagem e `POST`/`PUT`/`DELETE` de segredos ficam abertos (`admin_auth.rs:91-94`) — **lacuna de segurança ABERTA** (não corrigida), ver [admin-http-auth-fail-open](../security/admin-http-auth-fail-open.md).

| Método | Path | Auth | Resposta |
|--------|------|------|----------|
| GET | `/api/v1/admin/provider-credentials` | `BOT_HTTP_ADMIN_TOKEN` quando definido | **200** lista mascarada; **503** `provider_credentials_store_unavailable` sem PG |
| POST | `/api/v1/admin/provider-credentials` | idem | **200** upsert mascarado; **400** `invalid_provider_credential` |
| PUT | `/api/v1/admin/provider-credentials/{provider_id}/{key_name}` | idem | **200** replace mascarado |
| DELETE | `/api/v1/admin/provider-credentials/{provider_id}/{key_name}` | idem | **204** ou **404** `provider_credential_not_found` |

Corpo POST: `{ "provider_id", "key_name", "secret" }` (`provider_id` ∈ `typesafe|openai|nvidia|ngc`; v1 `key_name` = `api_key`). Respostas expõem `secret_masked` (`****` + últimos 4 chars); **nunca** coluna `secret` nem valor integral em JSON/logs. Após mutação, `reload_from_pool`. IdP/owner Gate 1 e encryption-at-rest permanecem follow-up.

## Tests

- Unit: env fallback when cache empty; `store::mask_secret` / validação `provider_id`.
- HTTP: `provider_credentials_admin_*` em `http_integration_tests.rs` (503 sem PG; upsert+list mascarado com PG; `provider_credentials_admin_delete_removes_row` com PG).
- Scaffold: migration SQL contains table name.
- Unit: `provider_credentials_encryption_mode_is_explicit_none_fail_closed` em `credentials/mod.rs`.
- HTTP: `meta_exposes_provider_credentials_encryption_none` e `healthz_exposes_provider_credentials_encryption_none` em `http_integration_tests.rs`.
- Integration: `loads_credentials_from_postgres` em `core/providers/credentials/pg_integration.rs` (skip sem `DATABASE_URL`); incluído em `./scripts/run-pg-integration-tests.sh` (manifesto de **28** testes; requer `DATABASE_URL` → `trading_bot`).
