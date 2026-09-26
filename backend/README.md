# Rust Trading Bot

Backend-only Rust trading bot operated through a terminal UI (Ratatui). There is no web frontend. The initial executable slice reads Binance Spot Test Network market data, calculates configurable SMA crossover signals, optionally asks Jev/TypeSafe for advisory evaluations, and renders state/logs in the terminal.

> **Trading safety:** the current version does not submit orders. Production startup and testnet order mode are deliberately blocked pending adapter verification, risk controls, and order-idempotency/reconciliation tests. Do not interpret a displayed signal as a trading recommendation.

## Initial modes and profiles

Operation modes: `hft`, `scalper`, `day_trader`, `swing_trader`. Timeframes are allocated by mode: scalper `1m/3m/5m`, day trader `5m/15m/30m`, swing trader `1h/4h`; HFT is marked unsupported because REST polling and candle intervals are not HFT infrastructure. The market-data adapter accepts the listed intervals. Risk profiles: `conservative`, `moderate`, `aggressive`, `auto`. Profiles scale a configured order cap; the absolute cap is never exceeded.

The operation presets are defined in Rust: scalper and day trader use SMA 5/20; swing trader uses SMA 20/50. `Config::validate` rejects other periods or timeframes for these operations.

## Requirements

- Rust toolchain (install via rustup or your platform's package manager).
- Network access to Binance Spot Test Network for live test candles.
- Optional TypeSafe/ Jev API key for advisory queries.

## Configure

Edit `src/config/bot.toml`. Its defaults are `dev`, `observe`, BTC/USDT, 15m, SMA 5/20. Both monitor and backtest require a readable config file. Without `--config`, they read `src/config/bot.toml` relative to the process working directory; run the commands below from `backend/` or pass an absolute path. A missing or unreadable file exits with an error that includes its path. Optional fresh testnet credentials must be supplied through process variables `BINANCE_TESTNET_API_KEY` and `BINANCE_TESTNET_SECRET`; the bot never prints them. The bot currently only requests public OHLCV, so credentials are not required for the first run.

For Jev, set `jev.enabled = true` and export `TYPESAFE_API_KEY`. The default endpoint is `https://api.typesafe.ai/v1/systemone`; override it with `TYPESAFE_ENDPOINT` only for a trusted compatible endpoint. HTTP is accepted only for localhost. The bot sends a small market/indicator snapshot only—no credentials, balances, or private order data. Jev is advisory and has no order authority.

Never commit `.env` or credentials. `.env.example` is a placeholder only. Credentials exposed in chat should be revoked and replaced.

## Run

```sh
# SMA backtest (synthetic 1m dataset, JSON summary on stdout)
cargo run -- backtest --config src/config/bot.toml

# Optional: ingest the synthetic dataset into PostgreSQL (database name must be `trading_bot`)
export DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot'
cargo run -- backtest --persist

# Live monitor (TUI) — default config path is `src/config/bot.toml`
cargo run -- --config src/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative

# Optional: persist each successful 1m OHLCV poll (off by default; requires DATABASE_URL + timeframe 1m)
export PERSIST_MARKET_DATA=1
```

### WebSocket + REST hybrid (read-only market data)

When `market.timeframe` is `1m`, the monitor runs **both**:

- **WebSocket** (`wss://testnet.binance.vision/ws/<symbol>@kline_1m` in `dev`): the base URL comes from the active `src/config/exchanges/binance.toml` Spot account. Each **closed** 1m kline upserts into `HybridCandleFeed` (`src/market_feed.rs`) and triggers SMA evaluation if that bar timestamp was not already evaluated (avoids duplicate eval when REST catches up).
- **REST poll** (`market.poll_seconds`): the Spot testnet origin in the same TOML is applied to ccxt's public endpoint as `https://testnet.binance.vision/api/v3`. It refreshes the sliding candle window as backfill/fallback; evaluates only when the newest bar timestamp advances past `last_evaluated_ts`.

For timeframes other than `1m`, only REST drives strategy (WS is not started). Shutdown cancels the stream via `CancellationToken`.

If the WS plan cannot start or its channel closes, the monitor logs a warning, disables WS reception and keeps REST polling and the terminal UI active. An absent WS channel from startup follows the same REST path.

Incoming WS messages are forwarded only when they are closed `kline` events for the configured symbol and `1m` interval, with aligned nonnegative timestamps and valid OHLCV. Invalid messages are ignored before reaching feed or persistence; a debug log records the rejection without printing the payload.

Evaluation needs the final `sma_slow + 1` closed candles to be contiguous at the configured timeframe. Older gaps outside that final window do not block evaluation. A WS candle received before enough REST history produces a visible warmup state and leaves its timestamp eligible for a later REST backfill. Once a complete window is evaluated, the timestamp advances monotonically, so subsequent REST polls of the same candle do not repeat a signal.

An out-of-order WS candle can also fill a gap behind the newest REST candle. In that case the feed triggers evaluation of the newest candle and records that candle's timestamp, preventing the next REST poll from evaluating it again.

The active Spot account accepts only the exact testnet REST origin and WS base shown above; external hosts, localhost, mainnet, userinfo, query, fragment and explicit ports are rejected before market data starts. The configured futures account is registered but is not used by the monitor. The ccxt HTTP client can follow redirects after the initial public REST request, so this origin check does not constrain a redirect target; the OHLCV request does not attach credentials. Production and order submission remain disabled.

Every REST OHLCV poll passes `authorize_rest_use` before calling the market-data adapter. The policy permits only public historical backfill for a selected Binance Spot account in `dev`; balance and order REST uses are rejected. The account and symbol are selected from the registry separately from this operation gate.

The REST adapter validates every completed candle in a returned window before the monitor can feed the strategy or persist the window. It rejects the entire window if any completed candle has a negative or misaligned timestamp, non-finite OHLCV, a non-positive price, negative volume, inconsistent high/low bounds, or a duplicate timestamp. The monitor reports a failed poll and leaves its feed and persistence paths untouched for that window. The current, unfinished candle is omitted.

With `PERSIST_MARKET_DATA=1` and `DATABASE_URL` set, 1m candles from REST windows (`monitor-rest`) and WS closes (`monitor-ws`) use the same `persist_dataset` path.

### PostgreSQL integration test (ignored by default)

```sh
DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot' \
  cargo test persist_dataset_round_trip -- --ignored --nocapture
```

CI (`.github/workflows/backend-ci.yml`) runs the same test in its PostgreSQL integration job with `DATABASE_URL` pointing at database `trading_bot`.

The Rust backend loads migrations only from `src/persistence/migrations/`. The repository-root `docker-compose.bot.yml` provisions `bot_agents` by default, not the required `trading_bot` database; configure a separate `trading_bot` database and `DATABASE_URL` for Rust persistence. The local test suite skips the PostgreSQL integration test unless explicitly run with that database.

When using the SQLx CLI manually from `backend/`, specify the migration source: `sqlx migrate run --source src/persistence/migrations`. The CLI's default `migrations/` path is no longer present.

Supported operation enum values are `hft`, `scalper`, `day-trader`, `swing-trader`; profile values are `conservative`, `moderate`, `aggressive`, `auto`. HFT currently exits with an explicit unsupported-mode error. Select only a timeframe allocated to the operation in `src/config/bot.toml`; invalid combinations fail at startup.

TUI: Space pauses/resumes market evaluation; `q` or Esc quits. Logs are structured to stderr and daily-rotated JSON files under `logs/`.

## Verify

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Architecture

- `src/config/`: `bot.toml` for monitor/backtest settings and `exchanges/binance.toml` as the active account/endpoint source. Rust code owns operation presets and fail-closed validation.
- `src/exchanges/binance.rs`: Binance testnet-only market data adapter via community `ccxt-rust`.
- `src/strategy/`: SMA strategy implemented with `mantis-ta`.
- `src/risk.rs`: deterministic risk caps, independent of strategy and Jev.
- `src/jev/`: optional structured HTTP adapter to TypeSafe System One.
- `src/app.rs`: asynchronous orchestration; the same evaluation settings (strategy configuration, risk limits, advisory, account and UI sender) are used for REST and WS candles. No direct order submission in this version.
- `src/ui/`: terminal dashboard and pause/quit controls.
- `src/market.rs`: canonical 1m `HistoricalDataset` for backtests/persistence; converts to `mantis_ta` candles for SMA evaluation.
- `src/persistence/`: the sole PostgreSQL migration directory is `src/persistence/migrations/`; `persist_dataset` serves `backtest --persist` and the monitor with `PERSIST_MARKET_DATA=1`.
- `src/market_feed.rs`: `HybridCandleFeed` merges REST windows and WS closed 1m bars with de-duplicated evaluation triggers.
- `src/exchanges/live.rs`: WS session plan + Binance `kline_1m` stream (`tokio-tungstenite`). No order submission on the WS path.
- `src/backtest_cli.rs`: non-TUI `backtest` subcommand.
- `src/domain.rs` / `src/portfolio.rs`: ranking and balance models used in tests and backtest output (not live execution).

`ccxt-rust` is a young community project, not the official CCXT library. Verify its sandbox endpoint behavior and API compatibility before enabling any order path. Testnet success is not production-readiness evidence.
