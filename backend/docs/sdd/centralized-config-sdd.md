# SDD: Configuração centralizada (.env)

## Contexto

Todas as variáprocesso do backend `rust-trading-bot` são lidas em arquivos `config.rs` por camada (`core::config/*` e `modules/*/config.rs`, `presentation/*/config.rs`). O bootstrap `core::config::ensure_dotenv_loaded()` roda uma vez em `main` (dotenvy).

## Seams públicos

| Módulo | Arquivo | Funções principais |
|--------|---------|-------------------|
| core bootstrap | `src/core/config/env_loader.rs` | `ensure_dotenv_loaded()` |
| core database | `src/core/config/database.rs` | `postgres_url_from_env`, `load_agents_stack_from_env` |
| core exchanges | `src/core/config/exchanges.rs` | credenciais Binance, redaction |
| core providers | `src/core/config/providers.rs` | Jev/LLM/NIM env |
| monitor | `src/modules/monitor/config.rs` | `PERSIST_MARKET_DATA`, `DATABASE_URL` (monitor) |
| orders | `src/modules/orders/config.rs` | `BOT_ORDERS_*`, paper price |
| exchanges | `src/modules/exchanges/config.rs` | `BOT_ORDERS_EXCHANGE_SUBMIT`, reexport Binance |
| agents | `src/modules/agents/config.rs` | `BOT_AGENCY` |
| bots | `src/modules/bots/config.rs` | `BOT_RUNTIME_ENABLED` |
| backtest | `src/modules/backtest/config.rs` | `--persist` + `DATABASE_URL` |
| HTTP | `src/presentation/http/config.rs` | `BOT_HTTP_*` |
| terminal | `src/presentation/terminal/config.rs` | (sem env hoje) |

TOML monitor continua em `core::config::Config::load` (não .env).

## Migração

- Call sites de `std::env::var` movidos para os arquivos acima; `core/database/config.rs` reexporta `core::config::database`.
- `credentials_env.rs` mantido como fachada fina para compatibilidade de imports.

## Riscos

- Ordem de init: `.env` antes de `TopCli::parse` e de qualquer leitura de config.
- Testes paralelos: mutações de env usam `core::test_env_lock::with_env_test_lock`.
- Apresentação não importa `core::config` (regra import-direction); HTTP usa `modules::orders::config` e `presentation::http::config`.

## Validação

- `scripts/verify-backend-gates.sh` (fmt, clippy, test --test-threads=1, import-direction, gate env).
- Testes unitários em `core/config/{database,exchanges,providers}.rs`, `modules/orders/config.rs`.
