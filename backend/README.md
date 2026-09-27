# Rust Trading Bot

Backend-only Rust trading bot operated through a terminal UI (Ratatui). There is no web frontend. The initial executable slice reads Binance Spot Test Network market data, calculates configurable SMA crossover signals, optionally asks Jev/TypeSafe for advisory evaluations, and renders state/logs in the terminal.

> **Trading safety:** production order submission remains disabled. Testnet Spot orders are opt-in (`BOT_ORDERS_EXECUTION=live_exchange`, `BOT_ORDERS_EXCHANGE_SUBMIT=testnet`, `BINANCE_TESTNET_*`) and still require risk gates, idempotency, and reconciliation before any production use. Do not interpret a displayed signal as a trading recommendation.

## Initial modes and profiles

Operation modes: `hft`, `scalper`, `day_trader`, `swing_trader`. Timeframes are allocated by mode: scalper `1m/3m/5m`, day trader `5m/15m/30m`, swing trader `1h/4h`; HFT is marked unsupported because REST polling and candle intervals are not HFT infrastructure. The market-data adapter accepts the listed intervals. Risk profiles: `conservative`, `moderate`, `aggressive`, `auto`. Profiles scale a configured order cap; the absolute cap is never exceeded.

The operation presets are defined in Rust: scalper and day trader use SMA 5/20; swing trader uses SMA 20/50. `Config::validate` rejects other periods or timeframes for these operations. There is no separate `profiles.toml`; edit the active Rust definitions and validation together when changing a preset.

## Requirements

- Rust toolchain (install via rustup or your platform's package manager).
- Network access to Binance Spot Test Network for live test candles.
- Optional TypeSafe/ Jev API key for advisory queries.

## Configure

Copy `backend/.env.example` → `backend/.env`. Non-sensitive defaults: `src/core/config/system.toml` and `bot.toml` for monitor and backtest settings. The active Binance account and endpoint configuration is `src/core/config/exchanges/binance.toml`; the old `exchanges/config/binance.toml` path was removed and is not read. Defaults are `dev`, `observe`, BTC/USDT, 15m, SMA 5/20. Both monitor and backtest require a readable config file. Without `--config`, they read `src/core/config/bot.toml` relative to the process working directory; run the commands below from `backend/` or pass an absolute path. A missing or unreadable file exits with an error that includes its path. Optional fresh testnet credentials must be supplied through process variables `BINANCE_TESTNET_API_KEY` and `BINANCE_TESTNET_SECRET`; the bot never prints them. The bot currently only requests public OHLCV, so credentials are not required for the first run.

For Jev, set `jev.enabled = true` and export `TYPESAFE_API_KEY` (or `OPENAI_API_KEY` for an OpenAI-compatible proxy). The default advisory endpoint is `https://api.typesafe.ai/v1/systemone`; override with `TYPESAFE_ENDPOINT` for a trusted compatible route (including a self-hosted **9router** proxy if it exposes the same advisory contract). For generic OpenAI-compatible HTTP (`/v1/chat/completions`), set `NINE_ROUTER_BASE_URL` or `OPENAI_BASE_URL` (optional TOML `[providers].openai_base_url`). HTTP is accepted only for localhost. The bot sends a small market/indicator snapshot only—no credentials, balances, or private order data. Jev lives under [`core::providers`](src/core/providers/mod.rs) and has no order authority.

Copy `backend/.env.example` to `backend/.env` (gitignored). On startup, `main` calls `core::config::ensure_dotenv_loaded()` once; each module reads its variables only from its `config.rs` (see `docs/sdd/centralized-config-sdd.md`). Never commit `.env` or credentials. Credentials exposed in chat should be revoked and replaced.

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

### PostgreSQL integration tests

Stack local recomendado: TimescaleDB HA (`docker-compose.bot.yml`); banco **`trading_bot`**. Ver [docs/operations/postgres-and-graph-dev.md](docs/operations/postgres-and-graph-dev.md).

```sh
DATABASE_URL='postgresql://user:pass@localhost:5432/trading_bot' \
  ./scripts/run-pg-integration-tests.sh
```

Executa **18** testes de integração PG (scaffold, market persist, agents/bots/orders adapters, HTTP `state.rs` incl. boot espelhando `serve`). Sem `DATABASE_URL`, os mesmos testes no gate passam com skip (`pg_integration`). CI (`.github/workflows/backend-ci.yml`) usa `timescale/timescaledb-ha:pg16` e o mesmo script.

Retenção orders G2 (manual): executar os `SELECT` comentados (dry-run) no topo de `scripts/pg-orders-retention-purge.sql`; revisar contagens; backup em produção; então `psql "$DATABASE_URL" -f scripts/pg-orders-retention-purge.sql` — ver [cli-and-config](docs/reference/cli-and-config.md#pg-orders-retention-gate-2).

Migrations load from `src/core/database/migrations/` (PostgreSQL 18+, TimescaleDB + pgvector via `0000_extensions.sql`). Optional Neo4j: set `BOT_AGENTS_ENABLED=true` and `BOT_NEO4J_*`; see `docker-compose.bot.yml` service `graph`. With Neo4j enabled, agent hierarchy (F1) and bot catalog/promotion (F2) project best-effort after PostgreSQL or runtime mutations — see [bots-neo4j-projection-sdd](docs/sdd/bots-neo4j-projection-sdd.md). HTTP `POST /api/v1/bots/catalog/persist` writes to PostgreSQL when `DATABASE_URL` is healthy, otherwise in-memory.

Supported operation enum values are `hft`, `scalper`, `day-trader`, `swing-trader`; profile values are `conservative`, `moderate`, `aggressive`, `auto`. HFT currently exits with an explicit unsupported-mode error. Select only a timeframe allocated to the operation in `src/core/config/bot.toml`; invalid combinations fail at startup.


### HTTP API (optional)

```sh
cargo run -- serve --bind 127.0.0.1:8080 --config src/core/config/bot.toml
# OpenAPI: http://127.0.0.1:8080/openapi.json — Scalar UI: http://127.0.0.1:8080/docs
# Optional: --with-monitor attaches a headless monitor for /api/v1/monitor/*
```

Mutating routes (agents lifecycle, bots catalog persist, bots runtime promote/demote, orders submit, `POST /api/v1/orders/reconciliation/poll`, monitor commands) honor optional `BOT_HTTP_ADMIN_TOKEN` when set; optional `BOT_HTTP_OWNER_ID` and `BOT_HTTP_AGENCY_ID` further restrict agent registration and agency-scoped agent routes. `GET /api/v1/meta` returns read-only `http_seams` (execution mode, admin/binding flags, bot runtime, live exchange wired). Orders: `GET /api/v1/orders/execution-status`, `GET /api/v1/orders/reconciliation/{client_order_id}`, `POST /api/v1/orders/submit`, `POST /api/v1/orders/reconciliation/poll` — **503** `execution_disabled` / `live_exchange_not_wired` / `order_store_unavailable` (PG) when applicable; **200** with `BOT_ORDERS_EXECUTION=paper`/`dev_accept` or wired `live_exchange` (see `.env.example` / [cli-and-config](docs/reference/cli-and-config.md)). See [docs/index.md](docs/index.md) and [module completeness audit](docs/planning/modules-completeness-audit.md).

TUI: Space pauses/resumes market evaluation; `q` or Esc quits. Logs are structured to stderr and daily-rotated JSON files under `logs/`.

## Verify

```sh
./scripts/verify-backend-gates.sh
# → OK: … 456 passed; 0 ignored (+ assert-pg-integration-manifest 21 tests)
# Gates + PG (when DATABASE_URL → trading_bot):
./scripts/verify-backend-full.sh
# → OK: backend full verification passed (+ PG 21/21)
# HTTP mutating routes / admin bearer (subset; included in gate total):
cargo test --locked --bin bot http_integration -- --test-threads=1
# → 48 passed
```

Runs `fmt`, `clippy` (`--bin bot`, `-D warnings`), import-direction check, `assert-pg-integration-manifest.sh` (PG **21** vs docs) (incl. HTTP routes → `ApiState`/`http_bridge`, sem domínio direto), `cargo test --locked --bin bot -- --test-threads=1`, then five workspace integration suites (`backtest_fixture`, `config_cli`, `monitor_startup_cli`, `redirect_origin_test`, `redirect_policy_test`) — does **not** re-run the full workspace `cargo test --locked` (would parallelize bin `bot` again and flake). A mensagem final inclui o resumo `test result:` do bin `bot`. CI (`.github/workflows/backend-ci.yml`): job `rust` executa este script; job `postgres-integration` executa `run-pg-integration-tests.sh` (**21/21** PG). As of 2026-09-27: **464** tests in `bot`, **0** ignored (integração PG/Neo4j/testnet via `pg_integration`; `run-pg-integration-tests.sh` **21/21** com `DATABASE_URL`). Module completeness audit (JSON snapshot: [docs/planning/modules-completeness-evidence.json](docs/planning/modules-completeness-evidence.json)): [docs/planning/modules-completeness-audit.md](docs/planning/modules-completeness-audit.md). Test matrix (incl. [bots runtime vs `serve` G2](docs/reference/test-matrix.md#bot-runtime-no-serve-vs-testes-http-g2-parcial)): [docs/reference/test-matrix.md](docs/reference/test-matrix.md). Checklists de itens ainda abertos: [agents G1](docs/sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist), [orders G2](docs/sdd/orders-live-execution-gate2-sdd.md#critérios-de-fechamento-g2-checklist), [bots runtime G2](docs/sdd/bots-runtime-live-gate2-sdd.md#critérios-de-fechamento-g2-checklist). HTTP admin seam (não substitui auth owner): [docs/sdd/http-admin-auth-seam-sdd.md](docs/sdd/http-admin-auth-seam-sdd.md); testes de rotas mutantes em `http_integration_tests.rs` (**48** passed com o comando acima).

## Architecture

Layer mapping (domain / application / infrastructure / presentation): [docs/architecture/layer-mapping.md](docs/architecture/layer-mapping.md). Module inventory and HTTP facades: [docs/architecture/module-catalog.md](docs/architecture/module-catalog.md) (§3d `http_bridge`, §3e `presentation::http`), [docs/architecture/integrations.md](docs/architecture/integrations.md).

[main.rs](src/main.rs) is the composition root (`core`, `modules`, `presentation`). The monitor path loads [core::config](src/core/config/mod.rs), bootstraps persistence via [modules::monitor::controllers::startup](src/modules/monitor/controllers/startup.rs), then runs [modules::monitor::controllers::supervisor](src/modules/monitor/controllers/supervisor.rs). Exchange wiring lives under [modules::exchanges](src/modules/exchanges/mod.rs) (Binance REST, optional `1m` WS via [adapters/live](src/modules/exchanges/adapters/live.rs)). [modules::market](src/modules/market/mod.rs) merges hybrid candles; [modules::strategy](src/modules/strategy/mod.rs), [modules::risk](src/modules/risk/mod.rs), and optional [`core::providers::jev`](src/core/providers/jev/mod.rs) advisory notes. [presentation::terminal](src/presentation/terminal/mod.rs) renders the TUI. Opt-in [core::persistence](src/core/persistence/mod.rs) stores validated 1m datasets; [persistence_health](src/modules/monitor/controllers/persistence_health.rs) tracks session baseline/gaps.

For `backtest`, [modules::backtest::cli](src/modules/backtest/cli.rs) drives synthetic 1m fixtures through [modules::market](src/modules/market/mod.rs) and [modules::backtest](src/modules/backtest/mod.rs) simulation before JSON output.

### Root modules

| Module | Responsibility |
|---|---|
| [core](src/core/mod.rs) | Shared `config`, `error`, `health`, `logging`, `notifications`, `persistence`, and `providers` (OpenAI-compatible clients + Jev). |
| [modules](src/modules/mod.rs) | Domain modules: `market`, `monitor`, `exchanges`, `strategy`, `risk`, `portfolio`, `backtest`, `agents`, `bots`, `orders`; HTTP application facades in [http_bridge](src/modules/http_bridge/mod.rs); shared seams in [application_contracts](src/modules/application_contracts.rs). |
| [presentation](src/presentation/mod.rs) | Terminal UI (`presentation::terminal`). |

### `modules::agents` (IdentityOnly foundation)

Administrative agent identities live under [modules/agents](src/modules/agents/mod.rs): in-memory `AgentRegistry`, hierarchy validation (owner → CEO → Level B → Level A → specialist/worker), lifecycle transitions (pause/resume/retire) with an audit trail, and `run_advisory_step` delegating to [`core::providers::jev`](src/core/providers/jev/mod.rs) only when `AgentCapabilities.consult_jev` is set. Registering an agent does not start workers, tools, or LLM calls. Design: [docs/sdd/agents-module-sdd.md](docs/sdd/agents-module-sdd.md). Optional PostgreSQL mirror + cold-start hydrate when `DATABASE_URL` points at `trading_bot`; **owner authentication on HTTP remains blocked** per [agents capability research](docs/research/agents-capability-research.md).

**Agents ≠ bots:** `modules/agents` is product **identity and governance** only. [`modules/bots`](src/modules/bots/mod.rs) holds versioned strategy×timeframe executors (catalog, ranking, HTTP); live runtime remains gated; catalog persist uses PostgreSQL when `DATABASE_URL` is healthy (see `PgBotCatalogStore`). [`modules/orders`](src/modules/orders/mod.rs) is a fail-closed order seam (`submit_order` validates risk then returns `ExecutionDisabled`). Do not confuse `bots`/`backtest::BotId` with [`modules/agents`](src/modules/agents/mod.rs) administrative identity. See [bots-module-sdd.md](docs/sdd/bots-module-sdd.md) and [agents-module-sdd.md](docs/sdd/agents-module-sdd.md).
### `modules::bots` and `modules::orders`

[`modules/bots`](src/modules/bots/mod.rs): `BotIdentity`, catalog from active config, full PnL ranking, and HTTP routes under `/api/v1/bots/*`. Design: [docs/sdd/bots-module-sdd.md](docs/sdd/bots-module-sdd.md).

[`modules/orders`](src/modules/orders/mod.rs): `submit_order` runs `risk::validate_intent` then `FailClosedExecutor` (no live exchange). Design: [docs/sdd/orders-module-sdd.md](docs/sdd/orders-module-sdd.md).

