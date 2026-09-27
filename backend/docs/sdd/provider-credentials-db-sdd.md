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

- Update row in `provider_credentials`; restart process or call `reload_from_pool` (HTTP admin CRUD: **not implemented** — fail-closed stub deferred).

## Tests

- Unit: env fallback when cache empty.
- Scaffold: migration SQL contains table name.
- Integration: `loads_credentials_from_postgres` em `core/providers/credentials/pg_integration.rs` (`#[ignore]`); incluído em `./scripts/run-pg-integration-tests.sh` (**15/15** com `DATABASE_URL` → `trading_bot`; evidência 2026-09-27: `OK: PostgreSQL integration tests passed (15 tests)`).
