# SDD: Configuração centralizada

## Source of truth

| Camada | Conteúdo |
|--------|----------|
| **`.env`** (gitignored) | Secrets e overrides operacionais (DATABASE_URL, Neo4j, Binance, HTTP admin, flags). **Env vence** TOML quando setado. |
| **`system.toml`** | Defaults **não sensíveis** do sistema (agents/neo4j/monitor/orders/bots/providers estrutura). |
| **`bot.toml`** | Preset monitor (market, strategy, risk, jev, logging, run_mode). |
| **`exchanges/binance.toml`** | Contas/endpoints exchange (público). |
| **PostgreSQL `provider_credentials`** | API keys LLM (primary); env bootstrap deprecated — ver `provider-credentials-db-sdd.md`. |

Bootstrap: `ensure_dotenv_loaded()` → `SystemConfig::init_from_path(--system-config)` → `Config::load(bot.toml)`.

## Árvore única `src/core/config/`

```
core/config/
  mod.rs              # Config (bot.toml), MonitorCli, reexports
  bot.toml
  system.toml
  env_loader.rs
  env_parse.rs
  load.rs
  system/mod.rs       # SystemConfig loader
  database/mod.rs + file.rs
  monitor/mod.rs + file.rs
  orders/mod.rs + file.rs
  agents/mod.rs + file.rs
  bots/mod.rs + file.rs
  http/mod.rs + file.rs
  providers/mod.rs + file.rs   # endpoints + resolve_* (keys via PG)
  backtest/mod.rs + file.rs
  exchanges/mod.rs + credentials.rs + binance.toml
```

`core/database/config.rs` — reexport fino de `core::config::database`.

## Removidos (cleanup)

- Arquivos `.rs` env-only na raiz (`monitor.rs`, `database.rs`, …).
- `config.rs` de env em `modules/*` e `presentation/*`.
- TOML duplicados por domínio (`database/database.toml`, …) → consolidados em `system.toml`.
- `app.rs` agregador não referenciado.

## Validação

`scripts/verify-backend-gates.sh` → **461** passed / **0** ignored (bin `bot`); PG opcional `./scripts/verify-backend-full.sh` → **21/21** (`run-pg-integration-tests.sh`, incl. `loads_credentials_from_postgres`). Checagens: fmt, clippy, import-direction, env reads em `core/config/**` + `core/providers/credentials/**`.
