# SDD: Configuração centralizada (.env)

## Regra

Toda leitura de variável de processo vive em `src/core/config/*.rs`. Módulos e presentation importam `crate::core::config::{...}` ou `AppConfig`; **não** há `config.rs` de env em `modules/*` nem `presentation/*`.

Bootstrap: `ensure_dotenv_loaded()` em `main` (dotenvy, idempotente).

## Árvore `core/config/`

| Arquivo | Env keys / notas |
|---------|------------------|
| `env_loader.rs` | (carrega `.env`) |
| `env_parse.rs` | helpers internos |
| `mod.rs` | TOML `Config`, CLI, reexports |
| `app.rs` | `AppConfig` agregador (lazy) |
| `database.rs` | `DATABASE_URL`, `BOT_AGENTS_ENABLED`, `BOT_NEO4J_*` |
| `neo4j.rs` | doc → vars em `database.rs` |
| `exchanges.rs` | `BINANCE_TESTNET_*`, `BINANCE_PROD_*` |
| `providers.rs` | `TYPESAFE_*`, `OPENAI_*`, `NINE_ROUTER_*`, `NVIDIA_*`, `NGC_*` |
| `monitor.rs` | `PERSIST_MARKET_DATA`, `DATABASE_URL` (bootstrap monitor) |
| `orders.rs` | `BOT_ORDERS_*`, `BOT_PAPER_FILL_UNIT_PRICE` |
| `agents.rs` | `BOT_AGENCY` |
| `bots.rs` | `BOT_RUNTIME_ENABLED` |
| `http.rs` | `BOT_HTTP_*` |
| `backtest.rs` | `--persist` → `DATABASE_URL` |
| `market.rs`, `strategy.rs`, `risk.rs`, `portfolio.rs`, `logging.rs`, `health.rs`, `terminal.rs`, `http_bridge.rs` | sem env (TOML / futuro) |
| `bot.toml`, `exchanges/binance.toml` | TOML monitor/exchange (não `.env`) |

`core/database/config.rs` reexporta `core::config::database`.

## Seams

- `MonitorEnvError` mapeado para `StartupError` em `main`.
- `modules/http_bridge/config.rs` permanece DTO HTTP (snapshot TOML), sem env.

## Riscos

- Testes: `core::test_env_lock` ao mutar env.
- Compatibilidade: nomes de env inalterados vs `.env.example`.

## Validação

`scripts/verify-backend-gates.sh` — fmt, clippy, env-centralization rg, tests.
