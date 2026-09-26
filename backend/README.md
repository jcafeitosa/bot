# Rust Trading Bot

Backend-only Rust trading bot operated through a terminal UI (Ratatui). There is no web frontend. The initial executable slice reads Binance Spot Test Network market data, calculates configurable SMA crossover signals, optionally asks Jev/TypeSafe for advisory evaluations, and renders state/logs in the terminal.

> **Trading safety:** the current version does not submit orders. Production startup and testnet order mode are deliberately blocked pending adapter verification, risk controls, and order-idempotency/reconciliation tests. Do not interpret a displayed signal as a trading recommendation.

## Initial modes and profiles

Operation modes: `hft`, `scalper`, `day_trader`, `swing_trader`. Timeframes are allocated by mode: scalper `1m/3m/5m`, day trader `5m/15m/30m`, swing trader `1h/4h`; HFT is marked unsupported because REST polling and candle intervals are not HFT infrastructure. The market-data adapter accepts the listed intervals. Risk profiles: `conservative`, `moderate`, `aggressive`, `auto`. Profiles scale a configured order cap; the absolute cap is never exceeded.

## Requirements

- Rust toolchain (install via rustup or your platform's package manager).
- Network access to Binance Spot Test Network for live test candles.
- Optional TypeSafe/ Jev API key for advisory queries.

## Configure

Edit `src/config/bot.toml`. Defaults are `dev`, `observe`, BTC/USDT, 1m, SMA 5/20. Optional fresh testnet credentials must be supplied through process variables `BINANCE_TESTNET_API_KEY` and `BINANCE_TESTNET_SECRET`; the bot never prints them. The bot currently only requests public OHLCV, so credentials are not required for the first run.

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

- **WebSocket** (`wss://stream.testnet.binance.vision/ws/<symbol>@kline_1m` in `dev`; mainnet uses `stream.binance.com` in `prod`): each **closed** 1m kline upserts into `HybridCandleFeed` (`src/market_feed.rs`) and triggers SMA evaluation if that bar timestamp was not already evaluated (avoids duplicate eval when REST catches up).
- **REST poll** (`market.poll_seconds`): refreshes the sliding candle window as backfill/fallback; evaluates only when the newest bar timestamp advances past `last_evaluated_ts`.

For timeframes other than `1m`, only REST drives strategy (WS is not started). Shutdown cancels the stream via `CancellationToken`.

With `PERSIST_MARKET_DATA=1` and `DATABASE_URL` set, 1m candles from REST windows (`monitor-rest`) and WS closes (`monitor-ws`) use the same `persist_dataset` path.

### PostgreSQL integration test (ignored by default)

```sh
DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot' \
  cargo test persist_dataset_round_trip -- --ignored --nocapture
```

CI (`.github/workflows/backend-ci.yml`) runs the same test in an optional job with a PostgreSQL service when `DATABASE_URL` points at database `trading_bot`.

Supported operation enum values are `hft`, `scalper`, `day-trader`, `swing-trader`; profile values are `conservative`, `moderate`, `aggressive`, `auto`. HFT currently exits with an explicit unsupported-mode error. Select only a timeframe allocated to the operation in `src/config/bot.toml`; invalid combinations fail at startup.

TUI: Space pauses/resumes market evaluation; `q` or Esc quits. Logs are structured to stderr and daily-rotated JSON files under `logs/`.

## Verify

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Architecture

- `src/config/`: código + TOML padrão (`bot.toml`, `profiles.toml`, `exchanges/`). explicit environment/mode/profile parsing and fail-closed startup validation.
- `src/exchanges/binance.rs`: Binance testnet-only market data adapter via community `ccxt-rust`.
- `src/strategy/`: SMA strategy implemented with `mantis-ta`.
- `src/risk.rs`: deterministic risk caps, independent of strategy and Jev.
- `src/jev/`: optional structured HTTP adapter to TypeSafe System One.
- `src/app.rs`: asynchronous orchestration; no direct order submission in this version.
- `src/ui/`: terminal dashboard and pause/quit controls.
- `src/market.rs`: canonical 1m `HistoricalDataset` for backtests/persistence; converts to `mantis_ta` candles for SMA evaluation.
- `src/persistence/`: optional PostgreSQL migrations + `persist_dataset` (`backtest --persist` or monitor with `PERSIST_MARKET_DATA=1`).
- `src/market_feed.rs`: `HybridCandleFeed` merges REST windows and WS closed 1m bars with de-duplicated evaluation triggers.
- `src/exchanges/live.rs`: WS session plan + Binance `kline_1m` stream (`tokio-tungstenite`). No order submission on the WS path.
- `src/backtest_cli.rs`: non-TUI `backtest` subcommand.
- `src/domain.rs` / `src/portfolio.rs`: ranking and balance models used in tests and backtest output (not live execution).

`ccxt-rust` is a young community project, not the official CCXT library. Verify its sandbox endpoint behavior and API compatibility before enabling any order path. Testnet success is not production-readiness evidence.
