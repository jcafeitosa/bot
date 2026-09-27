# Rust Trading Bot

Backend-only Rust trading bot operated through a terminal UI (Ratatui). There is no web frontend. The initial executable slice reads Binance Spot Test Network market data, calculates configurable SMA crossover signals, optionally asks Jev/TypeSafe for advisory evaluations, and renders state/logs in the terminal.

> **Trading safety:** the current version does not submit orders. Production startup and testnet order mode are deliberately blocked pending adapter verification, risk controls, and order-idempotency/reconciliation tests. Do not interpret a displayed signal as a trading recommendation.

## Initial modes and profiles

Operation modes: `hft`, `scalper`, `day_trader`, `swing_trader`. Timeframes are allocated by mode: scalper `1m/3m/5m`, day trader `5m/15m/30m`, swing trader `1h/4h`; HFT is marked unsupported because REST polling and candle intervals are not HFT infrastructure. The market-data adapter accepts the listed intervals. Risk profiles: `conservative`, `moderate`, `aggressive`, `auto`. Profiles scale a configured order cap; the absolute cap is never exceeded.

The operation presets are defined in Rust: scalper and day trader use SMA 5/20; swing trader uses SMA 20/50. `Config::validate` rejects other periods or timeframes for these operations. There is no separate `profiles.toml`; edit the active Rust definitions and validation together when changing a preset.

## Requirements

- Rust toolchain (install via rustup or your platform's package manager).
- Network access to Binance Spot Test Network for live test candles.
- Optional TypeSafe/ Jev API key for advisory queries.

## Configure

Edit `src/core/config/bot.toml` for monitor and backtest settings. The active Binance account and endpoint configuration is `src/core/config/exchanges/binance.toml`; the old `exchanges/config/binance.toml` path was removed and is not read. Defaults are `dev`, `observe`, BTC/USDT, 15m, SMA 5/20. Both monitor and backtest require a readable config file. Without `--config`, they read `src/core/config/bot.toml` relative to the process working directory; run the commands below from `backend/` or pass an absolute path. A missing or unreadable file exits with an error that includes its path. Optional fresh testnet credentials must be supplied through process variables `BINANCE_TESTNET_API_KEY` and `BINANCE_TESTNET_SECRET`; the bot never prints them. The bot currently only requests public OHLCV, so credentials are not required for the first run.

For Jev, set `jev.enabled = true` and export `TYPESAFE_API_KEY` (or `OPENAI_API_KEY` for an OpenAI-compatible proxy). The default advisory endpoint is `https://api.typesafe.ai/v1/systemone`; override with `TYPESAFE_ENDPOINT` for a trusted compatible route (including a self-hosted **9router** proxy if it exposes the same advisory contract). For generic OpenAI-compatible HTTP (`/v1/chat/completions`), set `NINE_ROUTER_BASE_URL` or `OPENAI_BASE_URL` (optional TOML `[providers].openai_base_url`). HTTP is accepted only for localhost. The bot sends a small market/indicator snapshot only—no credentials, balances, or private order data. Jev lives under [`core::providers`](src/core/providers/mod.rs) and has no order authority.

Never commit `.env` or credentials. `.env.example` is a placeholder only. Credentials exposed in chat should be revoked and replaced.

## Run

```sh
# SMA backtest (synthetic 1m dataset, JSON summary on stdout)
cargo run -- backtest --config src/core/config/bot.toml

# Optional: ingest the synthetic dataset into PostgreSQL (database name must be `trading_bot`)
export DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot'
cargo run -- backtest --persist

# Live monitor (TUI) — default config path is `src/core/config/bot.toml`
cargo run -- --config src/core/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative

# Optional: persist each successful 1m OHLCV poll (off by default; requires DATABASE_URL + timeframe 1m)
export PERSIST_MARKET_DATA=1
```

The backtest builds a deterministic synthetic 1m dataset for the selected timeframe: baseline bars warm the SMA, one higher bar generates a Buy, and the return to baseline generates a Sell. The simulation fills each signal on the following bar's open, including adverse slippage on a Sell exit; fees use the effective proceeds after slippage. The default 15m example currently reports `trades: 1`, `wins: 0`, `losses: 1`; its values illustrate the code path, not a market return. The fixture is capped at 20,000 1m candles and rejects larger or overflowing configurations. Changing the fixture changes its `dataset_id`, including for `--persist`. The CLI JSON summary does not expose `total_costs_quote`; use `BacktestReport` to inspect aggregate fees and slippage.

### WebSocket + REST hybrid (read-only market data)

When `market.timeframe` is `1m`, the monitor runs **both**:

- **WebSocket** (`wss://testnet.binance.vision/ws/<symbol>@kline_1m` in `dev`): the base URL comes from the active `src/core/config/exchanges/binance.toml` Spot account. Each **closed** 1m kline upserts into `HybridCandleFeed` ([modules/market/controllers/feed.rs](src/modules/market/controllers/feed.rs)) and triggers SMA evaluation if that bar timestamp was not already evaluated (avoids duplicate eval when REST catches up).
- **REST poll** (`market.poll_seconds`): the Spot testnet origin in the same TOML is applied to ccxt's public endpoint as `https://testnet.binance.vision/api/v3`. It refreshes the sliding candle window as backfill/fallback; evaluates only when the newest bar timestamp advances past `last_evaluated_ts`.

For timeframes other than `1m`, only REST drives strategy (WS is not started). Shutdown cancels the stream via `CancellationToken`.

If the WS plan cannot start or its channel closes, the monitor logs a warning, disables WS reception and keeps REST polling and the terminal UI active. An absent WS channel from startup follows the same REST path.

Space requests a state change, and the TUI displays the monitor's confirmed `RUNNING`, `PAUSED`, or `RESUMING` state. Pause stops strategy evaluation while continuing to drain incoming WS candles; it does not query REST. Resume immediately requests a public REST backfill and stays `RESUMING` until the latest closed candle is recent, the required SMA window is contiguous, and REST has caught up with any plausible WS timestamp observed during pause. Failed or stale backfills retry on the next poll. Only the newest eligible candle is evaluated once after recovery; missed signals from the pause are not replayed. REST and optional Jev advice run in cancelable tasks, so a blocked request does not delay pause confirmation. A future-dated WS candle is ignored, and a REST response older than the current WS feed does not replace it. Confirmed control state and the latest dashboard snapshot use `watch` channels; if the separate `AppEvent::Refresh` queue fills, the TUI still receives the latest snapshot, including reconciliation errors. Persistence uses one bounded write in flight. A second write offered while it is busy is treated as suspect and needs REST recovery. Quit cancels a pending write and logs that its commit outcome may be uncertain.

Incoming WS messages are forwarded only when they are closed `kline` events for the configured symbol and `1m` interval, with aligned nonnegative timestamps and valid OHLCV. Invalid messages are ignored before reaching feed or persistence; a debug log records the rejection without printing the payload.

The WS producer never waits for room in its bounded 64-candle channel. If the channel is full, it drops the incoming closed candle and logs the symbol, timestamp, capacity and cumulative drop count; it keeps reading the socket so heartbeat and reconnection remain responsive. A closed channel stops WS reception while REST polling and the TUI continue. REST backfill is the recovery path for dropped candles; a dropped WS event can delay evaluation until the next poll, and pauses do not guarantee a complete historical dataset.

Evaluation needs the final `sma_slow + 1` closed candles to be contiguous at the configured timeframe. Older gaps outside that final window do not block evaluation. A WS candle received before enough REST history produces a visible warmup state and leaves its timestamp eligible for a later REST backfill. Once a complete window is evaluated, the timestamp advances monotonically, so subsequent REST polls of the same candle do not repeat a signal.

An out-of-order WS candle can also fill a gap behind the newest REST candle. In that case the feed triggers evaluation of the newest candle and records that candle's timestamp, preventing the next REST poll from evaluating it again.

The active Spot account accepts only the exact testnet REST origin and WS base shown above; external hosts, localhost, mainnet, userinfo, query, fragment and explicit ports are rejected before market data starts. The configured futures account is registered but is not used by the monitor. The vendored ccxt HTTP client rejects redirects to a different URL origin before connecting to the redirect destination; local transport and pure policy tests for C9 passed independent review. A C10 test verifies that a rejected REST poll leaves the monitor running and a later WS candle can still update it. The test uses a simulated market source; it does not connect to Binance or prove DNS/proxy containment. The OHLCV request does not attach credentials. Production and order submission remain disabled.

Every REST OHLCV poll passes `authorize_rest_use` before calling the market-data adapter. The policy permits only public historical backfill for a selected Binance Spot account in `dev`; balance and order REST uses are rejected. The account and symbol are selected from the registry separately from this operation gate.

The REST adapter validates every completed candle in a returned window before the monitor can feed the strategy or persist the window. It rejects the entire window if any completed candle has a negative or misaligned timestamp, non-finite OHLCV, a non-positive price, negative volume, inconsistent high/low bounds, or a duplicate timestamp. The monitor reports a failed poll and leaves its feed and persistence paths untouched for that window. The current, unfinished candle is omitted.

Monitor persistence is off when `PERSIST_MARKET_DATA` is absent, `0`, or `false`; `DATABASE_URL` is not read or used for monitor startup in that case. Set `PERSIST_MARKET_DATA=1` or `true` to require a dedicated `trading_bot` PostgreSQL database and a `1m` timeframe. Missing or invalid URL, wrong database, failed connection/health check, or failed migration stops the monitor before market connections or the TUI start. Startup errors omit the URL and credentials. Any other flag value is a configuration error. `backtest --persist` is independent of this flag. When enabled, 1m candles from REST windows (`monitor-rest`) and WS closes (`monitor-ws`) use the same `persist_dataset` path. The TUI shows archive `DEGRADED` until the first contiguous REST window is committed; `HEALTHY` means no known gap since that session baseline, not a complete historical archive. Failed or uncertain writes, pause, WS overflow or disconnection stay `DEGRADED` until a later contiguous REST window covers the suspect minute through the latest observed close and commits. A recovery window that starts after the suspect minute sets sticky `GAP`; inspect PostgreSQL and reconcile history externally before trusting the archive. A successful later WS candle or recent REST window does not repair `GAP`. The strategy and paper evaluation continue during archive degradation. A write times out after five seconds; Quit cancels an in-flight write. The current schema ignores conflicting OHLCV for an existing `(symbol,time_ms)`, so a successful commit does not prove byte-for-byte agreement with a prior row.

### PostgreSQL integration test (ignored by default)

```sh
DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot' \
  cargo test persist_dataset_round_trip -- --ignored --nocapture
```

CI (`.github/workflows/backend-ci.yml`) runs the same test in its PostgreSQL integration job with `DATABASE_URL` pointing at database `trading_bot`.

The Rust backend loads migrations only from `src/core/persistence/migrations/`. The repository-root `docker-compose.bot.yml` provisions `bot_agents` by default, not the required `trading_bot` database; configure a separate `trading_bot` database and `DATABASE_URL` for Rust persistence. The local test suite skips the PostgreSQL integration test unless explicitly run with that database.

When using the SQLx CLI manually from `backend/`, specify the migration source: `sqlx migrate run --source src/core/persistence/migrations`. The old `migrations/0001_market_data.sql` copy was removed; the CLI's default `migrations/` path is no longer present. Keep the active migration in `src/core/persistence/migrations/` unchanged.

Supported operation enum values are `hft`, `scalper`, `day-trader`, `swing-trader`; profile values are `conservative`, `moderate`, `aggressive`, `auto`. HFT currently exits with an explicit unsupported-mode error. Select only a timeframe allocated to the operation in `src/core/config/bot.toml`; invalid combinations fail at startup.

TUI: Space pauses/resumes market evaluation; `q` or Esc quits. Logs are structured to stderr and daily-rotated JSON files under `logs/`.

## Verify

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/check-import-direction.sh
```

## Architecture

[main.rs](src/main.rs) is the composition root (`core`, `modules`, `presentation`). The monitor path loads [core::config](src/core/config/mod.rs), bootstraps persistence via [modules::monitor::controllers::startup](src/modules/monitor/controllers/startup.rs), then runs [modules::monitor::controllers::supervisor](src/modules/monitor/controllers/supervisor.rs). Exchange wiring lives under [modules::exchanges](src/modules/exchanges/mod.rs) (Binance REST, optional `1m` WS via [adapters/live](src/modules/exchanges/adapters/live.rs)). [modules::market](src/modules/market/mod.rs) merges hybrid candles; [modules::strategy](src/modules/strategy/mod.rs), [modules::risk](src/modules/risk/mod.rs), and optional [`core::providers::jev`](src/core/providers/jev/mod.rs) advisory notes. [presentation::terminal](src/presentation/terminal/mod.rs) renders the TUI. Opt-in [core::persistence](src/core/persistence/mod.rs) stores validated 1m datasets; [persistence_health](src/modules/monitor/controllers/persistence_health.rs) tracks session baseline/gaps.

For `backtest`, [modules::backtest::cli](src/modules/backtest/cli.rs) drives synthetic 1m fixtures through [modules::market](src/modules/market/mod.rs) and [modules::backtest](src/modules/backtest/mod.rs) simulation before JSON output.

### Root modules

| Module | Responsibility |
|---|---|
| [core](src/core/mod.rs) | Shared `config`, `error`, `health`, `logging`, `notifications`, `persistence`, and `providers` (OpenAI-compatible clients + Jev). |
| [modules](src/modules/mod.rs) | Domain modules: `market`, `monitor`, `exchanges`, `strategy`, `risk`, `portfolio`, `backtest`, `agents`, `bots`, `orders`; shared seams in [application_contracts](src/modules/application_contracts.rs). |
| [presentation](src/presentation/mod.rs) | Terminal UI (`presentation::terminal`). |

### `modules::agents` (IdentityOnly foundation)

Administrative agent identities live under [modules/agents](src/modules/agents/mod.rs): in-memory `AgentRegistry`, hierarchy validation (owner → CEO → Level B → Level A → specialist/worker), lifecycle transitions (pause/resume/retire) with an audit trail, and `run_advisory_step` delegating to [`core::providers::jev`](src/core/providers/jev/mod.rs) only when `AgentCapabilities.consult_jev` is set. Registering an agent does not start workers, tools, or LLM calls. Design: [docs/sdd/agents-module-sdd.md](docs/sdd/agents-module-sdd.md). PostgreSQL persistence and owner authentication remain blocked per [agents capability research](docs/research/agents-capability-research.md).

**Agents ≠ bots:** `modules/agents` is product **identity and governance** only. [`modules/bots`](src/modules/bots/mod.rs) holds versioned strategy×timeframe executors (catalog, ranking, HTTP); live runtime and PostgreSQL remain gated. [`modules/orders`](src/modules/orders/mod.rs) is a fail-closed order seam (`submit_order` validates risk then returns `ExecutionDisabled`). Do not confuse `bots`/`backtest::BotId` with [`modules/agents`](src/modules/agents/mod.rs) administrative identity. See [bots-module-sdd.md](docs/sdd/bots-module-sdd.md) and [agents-module-sdd.md](docs/sdd/agents-module-sdd.md).
### `modules::bots` and `modules::orders`

[`modules/bots`](src/modules/bots/mod.rs): `BotIdentity`, catalog from active config, full PnL ranking, and HTTP routes under `/api/v1/bots/*`. Design: [docs/sdd/bots-module-sdd.md](docs/sdd/bots-module-sdd.md).

[`modules/orders`](src/modules/orders/mod.rs): `submit_order` runs `risk::validate_intent` then `FailClosedExecutor` (no live exchange). Design: [docs/sdd/orders-module-sdd.md](docs/sdd/orders-module-sdd.md).

