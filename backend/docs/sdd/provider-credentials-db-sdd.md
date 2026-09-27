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
| secret | TEXT | never logged; encryption-at-rest optional follow-up ADR |
| updated_at | TIMESTAMPTZ | default now() |

## Public seam

- `core::providers::credentials::reload_from_pool(&PgPool)` — after migrations.
- `core::providers::credentials::lookup_secret(provider_id, key_name, env_fallback)` — cache then deprecated env.
- Wired from `core/config/providers/file.rs` (`resolve_bearer_api_key`, `resolve_nvidia_api_key`).

## Fail-closed

- Missing DB row and empty env → `BotError::Configuration` (Jev/NIM clients do not start).
- Reload errors after migrate → warn log; cache may stay empty until next connect.

## Rotation

- Update row in `provider_credentials`; restart process or call `reload_from_pool`.

## Dev seed (ops)

- Template: `backend/scripts/seed-provider-credentials.example.sql` (placeholders only).
- Migração `0008_provider_credentials_dev_seed_note.sql` — sem INSERT automático.
- Procedimento: [postgres-and-graph-dev.md](../operations/postgres-and-graph-dev.md#provider-credentials-llm-api-keys).

## HTTP admin CRUD (parcial, fail-closed)

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
- Integration: `loads_credentials_from_postgres` em `core/providers/credentials/pg_integration.rs` (skip sem `DATABASE_URL`); incluído em `./scripts/run-pg-integration-tests.sh` (**22/22** com `DATABASE_URL` → `trading_bot`; evidência 2026-09-27: `OK: PostgreSQL integration tests passed (21 tests)`).
