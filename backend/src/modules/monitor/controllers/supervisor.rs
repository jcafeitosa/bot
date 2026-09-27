use std::{future::Future, pin::Pin, sync::Arc, time::Instant};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{self, Duration},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::modules::monitor::controllers::dashboard_mapping::{
    monitor_snapshot_from_dashboard, new_dashboard, terminal_market_row,
};
use crate::modules::monitor::views::presentation_contract::MonitorCommand;
use crate::modules::monitor::views::terminal_dashboard::{AppEvent, Dashboard, MonitorState};
use crate::modules::monitor::MonitorHandle;
use crate::{
    core::config::{Config, RunMode},
    core::error::{BotError, BotResult},
    core::notifications::{LogNotifier, Notification, Notifier, Severity},
    core::persistence::Database,
    core::providers::{JevAdvisor, JevReviewInput},
    modules::application_contracts::{signal_label, BotSignal},
    modules::{
        agents::{MonitorAgentHook, NoopMonitorAgentHook},
        exchanges::{
            binance::BinanceMarketData,
            bootstrap::{load_registry, registered_market_types, spot_account_for_symbol},
            capabilities::catalog,
            live::{market_stream_plan, spawn_market_kline_stream, ClosedKline, WsOverflowSignal},
            preflight::log_transport_plan,
            resources::ResourceManager,
            rest::{authorize_rest_use, RestUse},
            router::{rest_use_for_need, route_market_need, MarketNeed},
            ExchangeAccountId, MarketDataSource,
        },
        market::{
            persist_historical_dataset, ws_matches_configured_timeframe, Candle, HistoricalDataset,
            HybridCandleFeed,
        },
        monitor::controllers::persistence_health::{PersistenceHealth, PersistenceState},
        orders::{
            live_exchange_submit_backend_enabled, submit_order, ExchangeSpotExecutor,
            FailClosedExecutor, OrderExecutionPort, OrderSide, OrdersError, PaperLedgerExecutor,
            SubmitOrderRequest,
        },
        portfolio::Asset,
        risk::{gate_signal, profile_limits, ExecutionContext, RiskLimits},
        strategy::{evaluate_for_kind, Signal, StrategySnapshot},
    },
};

fn monitor_spot_client_order_id(symbol: &str, candle_timestamp_ms: i64, side: OrderSide) -> String {
    let compact = symbol.replace('/', "");
    let side_label = match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    };
    format!("mon:{compact}:{candle_timestamp_ms}:{side_label}")
}

fn record_monitor_spot_submit_reconciliation(client_order_id: &str, symbol: &str, side: OrderSide) {
    use crate::modules::orders::{
        recording_bind_client_exchange, shared_live_order_reconciliation_ledger,
        take_last_spot_submit_ack, try_mirror_reconciliation_upsert, OrderReconciliationLedger,
    };
    let ledger = shared_live_order_reconciliation_ledger();
    if ledger.mark_pending(client_order_id, symbol, side).is_err() {
        // duplicate client_order_id for same candle/side — keep best-effort binding only
    }
    if let Some(ack) = take_last_spot_submit_ack() {
        recording_bind_client_exchange(client_order_id, &ack.exchange_order_id);
        let _ = ledger.confirm_exchange_order(client_order_id, &ack.exchange_order_id);
    }
    if let Some(state) = ledger.state(client_order_id) {
        let _ = try_mirror_reconciliation_upsert(client_order_id, symbol, side, &state);
    }
}

fn paper_fill_unit_price_from_close(close: f64) -> Option<f64> {
    if close.is_finite() && close > 0.0 {
        Some(close)
    } else {
        None
    }
}

async fn persist_polled_1m_window(
    db: &Database,
    symbol: &str,
    candles: Vec<mantis_ta::types::Candle>,
) -> Result<(), String> {
    match HistoricalDataset::from_mantis_1m(symbol, "monitor-rest", candles) {
        Ok(dataset) => persist_historical_dataset(db, &dataset)
            .await
            .map_err(|_| "database write failed".into()),
        Err(_) => Err("validation".into()),
    }
}

async fn persist_ws_1m_candle(db: &Database, symbol: &str, candle: Candle) -> Result<(), String> {
    match HistoricalDataset::from_1m(symbol, "monitor-ws", vec![candle]) {
        Ok(dataset) => persist_historical_dataset(db, &dataset)
            .await
            .map_err(|_| "database write failed".into()),
        Err(_) => Err("validation".into()),
    }
}

async fn jev_note(advisor: &Option<JevAdvisor>, snapshot: &StrategySnapshot) -> Option<String> {
    if let Some(jev) = advisor {
        let input = JevReviewInput {
            signal_label: signal_label(snapshot.signal),
            close: snapshot.close,
            fast_sma: snapshot.fast_sma,
            slow_sma: snapshot.slow_sma,
            candle_timestamp_ms: snapshot.candle_timestamp_ms,
        };
        match jev.review(&input).await {
            Ok(reviews) => Some(reviews.join(" | ")),
            Err(e) => {
                warn!(target: "jev", error=%e, "Jev unavailable; advisory omitted; strategy execution remains independently gated");
                None
            }
        }
    } else {
        None
    }
}

struct EvaluationSettings<'a> {
    config: &'a Config,
    limits: RiskLimits,
    spot_account_id: &'a ExchangeAccountId,
    event_tx: &'a mpsc::Sender<AppEvent>,
    dashboard_tx: &'a watch::Sender<Dashboard>,
}

async fn apply_strategy_snapshot(
    snapshot: StrategySnapshot,
    note: Option<String>,
    feed_source: &str,
    promoted_bot_id: Option<String>,
    settings: &EvaluationSettings<'_>,
    execution_ctx: &mut ExecutionContext,
    dashboard: &mut Dashboard,
) -> bool {
    let EvaluationSettings {
        config,
        limits,
        spot_account_id,
        event_tx,
        dashboard_tx,
        ..
    } = settings;
    info!(
        target: "market",
        symbol=%config.market.symbol,
        close=snapshot.close,
        candle_ts=snapshot.candle_timestamp_ms,
        feed_source,
        "Market candle evaluated"
    );
    let bot_signal = BotSignal {
        bot_id: promoted_bot_id,
        signal: snapshot.signal,
    };
    info!(target: "strategy", bot_signal=?bot_signal, fast_sma=?snapshot.fast_sma, slow_sma=?snapshot.slow_sma, "Strategy state");
    if matches!(snapshot.signal, Signal::Buy | Signal::Sell) {
        let _live = route_market_need(spot_account_id, MarketNeed::LiveIncremental);
        let _orders = route_market_need(spot_account_id, MarketNeed::OrderLifecycle);
        if let Some(rest_use) = rest_use_for_need(MarketNeed::OrderLifecycle) {
            if rest_use.requires_execution_gate() {
                let _ = authorize_rest_use(spot_account_id, rest_use);
            }
        }
        if RestUse::OrderSubmit.requires_execution_gate() {
            if let Err(e) = authorize_rest_use(spot_account_id, RestUse::OrderSubmit) {
                info!(target: "exchanges", error=%e, transport=?RestUse::OrderSubmit.transport(), "Order REST path blocked by execution policy");
            }
        }
        let open_positions_at_intent = execution_ctx.open_positions;
        match gate_signal(snapshot.signal, *limits, config.run_mode, execution_ctx) {
            Ok(()) => {
                let spot_seam =
                    config.run_mode == RunMode::Testnet && live_exchange_submit_backend_enabled();
                if config.run_mode != RunMode::Paper && !spot_seam {
                    warn!(
                        target: "risk",
                        environment=%config.environment,
                        mode=%config.run_mode,
                        quote_cap=limits.max_order_quote,
                        open_positions=execution_ctx.open_positions,
                        "Strategy intent passed risk gate; order execution remains fail-closed"
                    );
                }
                let side = match snapshot.signal {
                    Signal::Buy => OrderSide::Buy,
                    Signal::Sell => OrderSide::Sell,
                    _ => unreachable!("buy/sell branch only"),
                };
                let paper_fill_unit_price = paper_fill_unit_price_from_close(snapshot.close);
                let monitor_client_order_id = spot_seam.then(|| {
                    monitor_spot_client_order_id(
                        &config.market.symbol,
                        snapshot.candle_timestamp_ms,
                        side,
                    )
                });
                let order = SubmitOrderRequest {
                    symbol: &config.market.symbol,
                    side,
                    quote_amount: limits.max_order_quote,
                    estimated_daily_loss: execution_ctx.estimated_daily_loss_quote,
                    open_positions: open_positions_at_intent,
                    paper_fill_unit_price,
                    client_order_id: monitor_client_order_id.as_deref(),
                };
                static SPOT_EXECUTOR: ExchangeSpotExecutor = ExchangeSpotExecutor;
                let executor: &dyn OrderExecutionPort = if config.run_mode == RunMode::Paper {
                    &PaperLedgerExecutor
                } else if spot_seam {
                    &SPOT_EXECUTOR
                } else {
                    &FailClosedExecutor
                };
                match submit_order(order, *limits, executor) {
                    Ok(()) => {
                        if let Some(ref client_order_id) = monitor_client_order_id {
                            record_monitor_spot_submit_reconciliation(
                                client_order_id,
                                &config.market.symbol,
                                side,
                            );
                        }
                    }
                    Err(OrdersError::ExecutionDisabled) => info!(
                        target: "orders",
                        symbol=%config.market.symbol,
                        side=?side,
                        "Order seam reached fail-closed executor (ExecutionDisabled)"
                    ),
                    Err(e) => {
                        warn!(target: "orders", error=%e, "Order submit failed after risk gate")
                    }
                }
            }
            Err(e) => warn!(target: "risk", error=%e, "Strategy intent rejected by risk gate"),
        }
    }
    dashboard.set_paper_open_positions(execution_ctx.open_positions);
    dashboard.update_market(terminal_market_row(&snapshot), note);
    dashboard.push_log(format!(
        "{} | {} | {} | {}",
        config.market.symbol, config.operation, config.risk_profile, feed_source
    ));
    publish_dashboard(dashboard, dashboard_tx, event_tx);
    !event_tx.is_closed()
}

async fn recv_ws_kline(rx: &mut Option<mpsc::Receiver<ClosedKline>>) -> Option<ClosedKline> {
    match rx {
        Some(channel) => channel.recv().await,
        None => std::future::pending().await,
    }
}

async fn fetch_authorized_candles<S: MarketDataSource + ?Sized>(
    source: &S,
    account: &ExchangeAccountId,
    symbol: &str,
    timeframe: &str,
    limit: u32,
) -> BotResult<Vec<mantis_ta::types::Candle>> {
    authorize_rest_use(account, RestUse::HistoricalBackfill).map_err(|error| {
        BotError::Exchange(format!("Historical REST backfill rejected: {error}"))
    })?;
    source.candles(symbol, timeframe, limit).await
}

type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;
type RestWindow = Vec<mantis_ta::types::Candle>;
type ReviewFuture = Pin<Box<dyn Future<Output = Option<String>> + Send>>;
type Review = Arc<dyn Fn(StrategySnapshot) -> ReviewFuture + Send + Sync>;
type PersistFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
type PersistRest = Arc<dyn Fn(RestWindow) -> PersistFuture + Send + Sync>;
type PersistWs = Arc<dyn Fn(Candle) -> PersistFuture + Send + Sync>;

fn persistence_error_class(error: &str) -> &'static str {
    match error {
        "validation" => "validation",
        "rest-write timeout" | "ws-write timeout" => "timeout",
        _ => "database-write",
    }
}
const PERSIST_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
enum WriteKind {
    Ws(i64, bool),
    Rest(Vec<i64>),
}

struct PersistenceWrite {
    kind: WriteKind,
    revision: u64,
    started: Instant,
    task: JoinHandle<Result<(), String>>,
}

struct MarketLoop {
    config: Config,
    limits: RiskLimits,
    account: ExchangeAccountId,
    source: Arc<dyn MarketDataSource>,
    review: Review,
    agent_hook: std::sync::Arc<dyn MonitorAgentHook>,
    persistence_enabled: bool,
    persist_rest: PersistRest,
    persist_ws: PersistWs,
    ws_rx: Option<mpsc::Receiver<ClosedKline>>,
    overflow_signal: Option<Arc<WsOverflowSignal>>,
    commands: mpsc::Receiver<MonitorCommand>,
    events: mpsc::Sender<AppEvent>,
    dashboard_tx: watch::Sender<Dashboard>,
    state_tx: watch::Sender<MonitorState>,
    interval: time::Interval,
    now_ms: Clock,
}

struct EvaluationCandidate {
    generation: u64,
    timestamp: i64,
    source: &'static str,
    snapshot: StrategySnapshot,
    note: Option<String>,
    promoted_bot_id: Option<String>,
}

fn period_ms(timeframe: &str) -> Option<i64> {
    Some(match timeframe {
        "1m" => 60_000,
        "3m" => 180_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "4h" => 14_400_000,
        _ => return None,
    })
}

fn closed_by_clock(timestamp: i64, period: i64, now: i64) -> bool {
    timestamp
        .checked_add(period)
        .is_some_and(|close| close <= now)
}

fn fresh_closed_by_clock(timestamp: i64, period: i64, now: i64) -> bool {
    timestamp
        .checked_add(period)
        .and_then(|close| now.checked_sub(close))
        .is_some_and(|age| (0..=period).contains(&age))
}

fn valid_rest_window(candles: &[mantis_ta::types::Candle]) -> bool {
    !candles.is_empty()
        && candles
            .iter()
            .all(|candle| Candle::from(*candle).validate().is_ok())
        && candles
            .windows(2)
            .all(|pair| pair[0].timestamp < pair[1].timestamp)
}

fn publish_dashboard(
    dashboard: &Dashboard,
    dashboard_tx: &watch::Sender<Dashboard>,
    events: &mpsc::Sender<AppEvent>,
) {
    dashboard_tx.send_replace(dashboard.clone());
    match events.try_send(AppEvent::Refresh(dashboard.clone())) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!(target: "ui", "Dashboard refresh queue full; latest state is available via watch");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            warn!(target: "ui", "Dashboard refresh receiver closed");
        }
    }
}

fn publish_state(
    state: MonitorState,
    state_tx: &watch::Sender<MonitorState>,
    dashboard: &mut Dashboard,
    dashboard_tx: &watch::Sender<Dashboard>,
    events: &mpsc::Sender<AppEvent>,
) {
    dashboard.monitor_state = state;
    let _ = state_tx.send(state);
    publish_dashboard(dashboard, dashboard_tx, events);
}

fn publish_health(
    health: &PersistenceHealth,
    dashboard: &mut Dashboard,
    dashboard_tx: &watch::Sender<Dashboard>,
    events: &mpsc::Sender<AppEvent>,
) {
    if dashboard.persistence_status != health.label() {
        let previous = dashboard.persistence_status.clone();
        info!(target: "persistence", symbol=%dashboard.header.symbol, previous=%previous, current=health.label(), suspect_from=?health.suspect_from(), "Archive session state changed");
        notify_archive_transition(&previous, health);
        dashboard.persistence_status = health.label().into();
        publish_dashboard(dashboard, dashboard_tx, events);
    }
}

fn notify_archive_transition(previous: &str, health: &PersistenceHealth) {
    let notifier = LogNotifier;
    match health.state() {
        PersistenceState::Gap => notifier.notify(&Notification {
            severity: Severity::Critical,
            target: "notifications::persistence",
            message: format!(
                "Market archive entered GAP (previous status: {previous}; current: {})",
                health.label()
            ),
        }),
        PersistenceState::Degraded if previous.starts_with("HEALTHY") => {
            notifier.notify(&Notification {
                severity: Severity::Warning,
                target: "notifications::persistence",
                message: format!(
                    "Market archive degraded from HEALTHY (current: {})",
                    health.label()
                ),
            })
        }
        _ => {}
    }
}

fn apply_ws_losses(signal: &Option<Arc<WsOverflowSignal>>, health: &mut PersistenceHealth) {
    if health.state() == PersistenceState::Off {
        return;
    }
    let Some(signal) = signal else { return };
    if let Some(timestamp) = signal.take() {
        health.mark_suspect(timestamp);
        warn!(target: "persistence", timestamp, "WS channel overflow; REST recovery required");
    }
    if signal.take_disconnect() {
        if let Some(timestamp) = health
            .last_confirmed()
            .and_then(|last| last.checked_add(60_000))
        {
            health.mark_suspect(timestamp);
            warn!(target: "persistence", timestamp, "WS disconnected; REST recovery required");
        }
    }
}

fn start_rest(inputs: &MarketLoop, generation: u64) -> JoinHandle<(u64, BotResult<RestWindow>)> {
    let source = inputs.source.clone();
    let account = inputs.account.clone();
    let symbol = inputs.config.market.symbol.clone();
    let timeframe = inputs.config.market.timeframe.clone();
    let limit = inputs.config.market.candle_limit;
    tokio::spawn(async move {
        let result = time::timeout(
            Duration::from_secs(10),
            fetch_authorized_candles(source.as_ref(), &account, &symbol, &timeframe, limit),
        )
        .await
        .unwrap_or_else(|_| Err(BotError::MarketData("REST backfill timed out".into())));
        (generation, result)
    })
}

fn run_evaluation_cycle(
    feed: &HybridCandleFeed,
    timestamp: i64,
    source: &'static str,
    inputs: &MarketLoop,
    generation: u64,
) -> JoinHandle<EvaluationCandidate> {
    let feed = feed.clone();
    let review = inputs.review.clone();
    let binding = crate::modules::bots::strategy_evaluation_binding(&inputs.config);
    let evaluator = binding.evaluator;
    let fast = binding.sma_fast;
    let slow = binding.sma_slow;
    let timeframe = binding.timeframe.clone();
    let promoted_bot_id = binding.promoted_bot_id.clone();
    tokio::spawn(async move {
        let snapshot = if feed.ready_for_evaluation(slow, &timeframe) {
            evaluate_for_kind(evaluator, feed.candles(), fast, slow)
        } else {
            StrategySnapshot {
                signal: Signal::Warmup,
                close: feed.candles().last().map_or(0.0, |bar| bar.close),
                fast_sma: None,
                slow_sma: None,
                candle_timestamp_ms: timestamp,
            }
        };
        let note = if snapshot.signal == Signal::Warmup {
            None
        } else {
            review(snapshot.clone()).await
        };
        EvaluationCandidate {
            generation,
            timestamp,
            source,
            snapshot,
            note,
            promoted_bot_id,
        }
    })
}

fn observe_paused_ws(kline: ClosedKline, watermark: &mut Option<i64>, period: i64, now: i64) {
    let ts = kline.candle.timestamp_ms;
    if closed_by_clock(ts, period, now) {
        *watermark = Some(watermark.map_or(ts, |last| last.max(ts)));
    } else {
        warn!(target: "market", timestamp=ts, "Future WS candle ignored during pause/resume");
    }
}

fn drain_paused_ws(
    rx: &mut Option<mpsc::Receiver<ClosedKline>>,
    watermark: &mut Option<i64>,
    period: i64,
    now: i64,
) {
    let Some(channel) = rx.as_mut() else { return };
    loop {
        match channel.try_recv() {
            Ok(kline) => observe_paused_ws(kline, watermark, period, now),
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                *rx = None;
                break;
            }
        }
    }
}

async fn run_market_loop(mut inputs: MarketLoop) {
    let mut state = MonitorState::Running;
    let mut generation = 0u64;
    let mut pause_watermark = None;
    let mut feed = HybridCandleFeed::new(inputs.config.market.candle_limit);
    let mut dashboard = new_dashboard(&inputs.config, inputs.limits);
    let mut execution_ctx = ExecutionContext::default();
    let mut rest_task: Option<JoinHandle<(u64, BotResult<RestWindow>)>> = None;
    let mut eval_task: Option<JoinHandle<EvaluationCandidate>> = None;
    let mut persistence_task: Option<PersistenceWrite> = None;
    let mut health = PersistenceHealth::new(inputs.persistence_enabled);
    dashboard.persistence_status = health.label().into();
    let period = period_ms(&inputs.config.market.timeframe).unwrap_or(60_000);
    loop {
        tokio::select! {
            cmd = inputs.commands.recv() => match cmd {
                Some(MonitorCommand::Shutdown) | None => break,
                Some(MonitorCommand::Pause) if state != MonitorState::Paused => {
                    generation = generation.wrapping_add(1);
                    if let Some(task) = rest_task.take() { task.abort(); }
                    if let Some(task) = eval_task.take() { task.abort(); }
                    pause_watermark = None;
                    if let Some(next) = health.last_confirmed().and_then(|last| last.checked_add(60_000)).or_else(|| health.latest_closed()) {
                        health.mark_suspect(next);
                    }
                    dashboard.persistence_status = health.label().into();
                    state = MonitorState::Paused;
                    publish_state(state, &inputs.state_tx, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                    info!(target: "system", "Monitor paused by operator");
                }
                Some(MonitorCommand::Resume) if state == MonitorState::Paused => {
                    generation = generation.wrapping_add(1);
                    state = MonitorState::Resuming;
                    publish_state(state, &inputs.state_tx, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                    rest_task = Some(start_rest(&inputs, generation));
                    info!(target: "system", "Monitor reconciling REST before resume");
                }
                _ => {}
            },
            kline = recv_ws_kline(&mut inputs.ws_rx) => match kline {
                Some(kline) => {
                    let now = (inputs.now_ms)();
                    if state != MonitorState::Running {
                        if closed_by_clock(kline.candle.timestamp_ms, period, now) {
                            health.mark_suspect(kline.candle.timestamp_ms);
                            publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                        }
                        observe_paused_ws(kline, &mut pause_watermark, period, now);
                    } else if !closed_by_clock(kline.candle.timestamp_ms, period, now) {
                        warn!(target: "market", timestamp=kline.candle.timestamp_ms, "Future WS candle ignored");
                    } else {
                        health.observe_closed(kline.candle.timestamp_ms);
                        if inputs.persistence_enabled {
                            let timestamp = kline.candle.timestamp_ms;
                            let isolated_in_flight = health.suspect_from().is_none();
                            health.mark_suspect(timestamp);
                            if persistence_task.is_none() {
                                let candle = kline.candle;
                                let write = (inputs.persist_ws)(candle);
                                let task = tokio::spawn(async move {
                                    time::timeout(PERSIST_WRITE_TIMEOUT, write)
                                        .await.unwrap_or_else(|_| Err("ws-write timeout".into()))
                                });
                                persistence_task = Some(PersistenceWrite { kind: WriteKind::Ws(timestamp, isolated_in_flight), revision: health.revision(), started: Instant::now(), task });
                            } else {
                                warn!(target: "persistence", timestamp, "WS write skipped while persistence is busy; REST recovery required");
                            }
                            publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                        }
                        if let Some(timestamp) = feed.ingest_ws_closed(kline.candle) {
                            if eval_task.is_none() {
                                eval_task = Some(run_evaluation_cycle(&feed, timestamp, "ws", &inputs, generation));
                            }
                        }
                    }
                }
                None => {
                    warn!(target: "exchanges::ws", "Websocket channel ended; REST polling remains active");
                    if let Some(next) = health.last_confirmed().and_then(|last| last.checked_add(60_000)) {
                        health.mark_suspect(next);
                        publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                    }
                    inputs.ws_rx = None;
                }
            },
            _ = async { inputs.overflow_signal.as_ref().expect("guarded overflow signal").notified().await }, if inputs.overflow_signal.is_some() => {
                apply_ws_losses(&inputs.overflow_signal, &mut health);
                publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
            },
            _ = inputs.interval.tick() => {
                if state != MonitorState::Paused && rest_task.is_none() {
                    rest_task = Some(start_rest(&inputs, generation));
                }
            },
            rest = async { rest_task.as_mut().expect("guarded REST task").await }, if rest_task.is_some() => {
                rest_task = None;
                let Ok((task_generation, result)) = rest else { continue };
                if task_generation != generation || state == MonitorState::Paused { continue }
                if state == MonitorState::Resuming {
                    drain_paused_ws(&mut inputs.ws_rx, &mut pause_watermark, period, (inputs.now_ms)());
                }
                let candles = match result {
                    Ok(candles) if valid_rest_window(&candles) => candles,
                    Ok(_) => {
                        dashboard.set_error("REST returned an empty or invalid candle window".into());
                        publish_dashboard(&dashboard, &inputs.dashboard_tx, &inputs.events);
                        continue;
                    }
                    Err(error) => {
                        warn!(target: "market", error=%error, "Market data poll failed");
                        dashboard.set_error(error.to_string());
                        publish_dashboard(&dashboard, &inputs.dashboard_tx, &inputs.events);
                        continue;
                    }
                };
                let latest = candles.last().expect("validated nonempty window").timestamp;
                let now = (inputs.now_ms)();
                if state == MonitorState::Resuming {
                    if !fresh_closed_by_clock(latest, period, now)
                        || pause_watermark.is_some_and(|watermark| latest < watermark)
                        || feed.last_evaluated_timestamp().is_some_and(|last| latest < last)
                    {
                        dashboard.set_error("REST reconciliation is stale or behind observed WS".into());
                        publish_dashboard(&dashboard, &inputs.dashboard_tx, &inputs.events);
                        continue;
                    }
                } else if !closed_by_clock(latest, period, now)
                    || feed.candles().last().is_some_and(|bar| latest < bar.timestamp)
                {
                    warn!(target: "market", timestamp=latest, "Stale or future REST window ignored");
                    continue;
                }
                let mut candidate = feed.clone();
                let trigger = candidate.ingest_rest_window(candles.clone());
                let eval_binding = crate::modules::bots::strategy_evaluation_binding(&inputs.config);
                if state == MonitorState::Resuming
                    && !candidate.ready_for_evaluation(eval_binding.sma_slow, &eval_binding.timeframe)
                {
                    dashboard.set_error("REST reconciliation lacks contiguous history".into());
                    publish_dashboard(&dashboard, &inputs.dashboard_tx, &inputs.events);
                    continue;
                }
                feed = candidate;
                apply_ws_losses(&inputs.overflow_signal, &mut health);
                health.observe_closed(latest);
                let timestamps: Vec<i64> = candles.iter().map(|candle| candle.timestamp).collect();
                health.observe_rest_window(&timestamps);
                if persistence_task.is_none() {
                    if inputs.persistence_enabled { health.mark_suspect(timestamps[0]); }
                    let revision = health.revision();
                    let write = (inputs.persist_rest)(candles);
                    let task = tokio::spawn(async move {
                        time::timeout(PERSIST_WRITE_TIMEOUT, write).await
                            .unwrap_or_else(|_| Err("rest-write timeout".into()))
                    });
                    persistence_task = Some(PersistenceWrite { kind: WriteKind::Rest(timestamps), revision, started: Instant::now(), task });
                } else {
                    health.mark_suspect(timestamps[0]);
                    warn!(target: "persistence", first=timestamps[0], last=latest, "REST write skipped while persistence is busy");
                }
                publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                if state == MonitorState::Resuming {
                    state = MonitorState::Running;
                    pause_watermark = None;
                    dashboard.last_error = None;
                    publish_state(state, &inputs.state_tx, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                    info!(target: "system", timestamp=latest, "Monitor resumed after REST reconciliation");
                }
                if let Some(timestamp) = trigger {
                    if let Some(task) = eval_task.take() { task.abort(); }
                    eval_task = Some(run_evaluation_cycle(&feed, timestamp, "rest", &inputs, generation));
                }
            },
            result = async { eval_task.as_mut().expect("guarded evaluation task").await }, if eval_task.is_some() => {
                eval_task = None;
                let Ok(candidate) = result else { continue };
                if candidate.generation != generation || state != MonitorState::Running { continue }
                let newest = feed.candles().last().map(|bar| bar.timestamp);
                if newest != Some(candidate.timestamp) {
                    if let Some(timestamp) = newest.filter(|timestamp| feed.last_evaluated_timestamp().is_none_or(|last| *timestamp > last)) {
                        eval_task = Some(run_evaluation_cycle(&feed, timestamp, "ws", &inputs, generation));
                    }
                    continue;
                }
                if candidate.snapshot.signal != Signal::Warmup {
                    feed.mark_evaluated(candidate.timestamp);
                }
                let evaluation_agents = inputs.agent_hook.evaluation_agents();
                inputs.agent_hook.on_evaluation_cycle(&evaluation_agents);
                let settings = EvaluationSettings {
                    config: &inputs.config,
                    limits: inputs.limits,
                    spot_account_id: &inputs.account,
                    event_tx: &inputs.events,
                    dashboard_tx: &inputs.dashboard_tx,
                };
                if !apply_strategy_snapshot(candidate.snapshot, candidate.note, candidate.source, candidate.promoted_bot_id.clone(), &settings, &mut execution_ctx, &mut dashboard).await {
                    break;
                }
            },
            completed = async { (&mut persistence_task.as_mut().expect("guarded persistence task").task).await }, if persistence_task.is_some() => {
                let write = persistence_task.take().expect("completed persistence task");
                apply_ws_losses(&inputs.overflow_signal, &mut health);
                let (source, first_ms, last_ms) = match &write.kind {
                    WriteKind::Ws(timestamp, _) => ("ws", *timestamp, *timestamp),
                    WriteKind::Rest(timestamps) => ("rest", timestamps[0], *timestamps.last().expect("nonempty REST write")),
                };
                let previous = health.label();
                let duration_ms = write.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
                let mut failure_class = None;
                match completed {
                    Ok(Ok(())) => match write.kind {
                        WriteKind::Ws(timestamp, isolated_in_flight) => health.ws_confirmed_at(timestamp, write.revision, isolated_in_flight),
                        WriteKind::Rest(timestamps) => {
                            let latest = health.latest_closed().unwrap_or_else(|| *timestamps.last().expect("nonempty REST write"));
                            health.rest_confirmed_at(&timestamps, latest, write.revision);
                        }
                    },
                    Ok(Err(error)) => failure_class = Some(persistence_error_class(&error)),
                    Err(_) => failure_class = Some("cancelled"),
                }
                if let Some(class) = failure_class {
                    warn!(target: "persistence", source, symbol=%inputs.config.market.symbol, first_ms, last_ms, duration_ms, previous, current=health.label(), error_class=class, "Persistence write failed or unconfirmed; REST recovery required");
                    dashboard.push_log(format!("Persistence {source} {class}; REST recovery required"));
                } else {
                    info!(target: "persistence", source, symbol=%inputs.config.market.symbol, first_ms, last_ms, duration_ms, previous, current=health.label(), "Persistence write confirmed");
                }
                publish_health(&health, &mut dashboard, &inputs.dashboard_tx, &inputs.events);
                if inputs.persistence_enabled {
                    publish_dashboard(&dashboard, &inputs.dashboard_tx, &inputs.events);
                }
            },
        }
    }
    if let Some(task) = rest_task {
        task.abort();
    }
    if let Some(task) = eval_task {
        task.abort();
    }
    if let Some(write) = persistence_task {
        error!(target: "persistence", "Persistence task cancelled on quit; commit outcome is uncertain");
        write.task.abort();
    }
}

/// Monitor entry with default noop agent hook (see `run_with_agent_hook`).
#[allow(dead_code)] // Public seam re-exported from `modules::monitor`.
pub async fn run(config: Config, database: Option<Database>) -> BotResult<()> {
    run_with_agent_hook(config, database, Arc::new(NoopMonitorAgentHook)).await
}

pub async fn run_with_agent_hook(
    config: Config,
    database: Option<Database>,
    agent_hook: Arc<dyn MonitorAgentHook>,
) -> BotResult<()> {
    run_with_agent_hook_inner(config, database, agent_hook, false, None).await
}

async fn run_with_agent_hook_inner(
    config: Config,
    database: Option<Database>,
    agent_hook: Arc<dyn MonitorAgentHook>,
    headless: bool,
    handle_tx: Option<tokio::sync::oneshot::Sender<MonitorHandle>>,
) -> BotResult<()> {
    let registry = load_registry(config.environment)
        .map_err(|e| crate::core::error::BotError::Configuration(e.to_string()))?;
    let spot = spot_account_for_symbol(&registry, config.environment, &config.market.symbol)?;
    info!(
        target: "exchanges",
        account_key = %spot.id.key(),
        rate_limit = spot.rate_limit_per_minute,
        markets = ?registered_market_types(&registry, config.environment),
        "Exchange registry loaded"
    );
    log_transport_plan(&registry, config.environment, spot, &config.market.symbol);
    // Hybrid feed: WS closed 1m klines trigger evaluation; REST poll refreshes the window on interval.
    let ws_shutdown = CancellationToken::new();
    let persistence_enabled = database.is_some();
    let overflow_signal = persistence_enabled.then(|| Arc::new(WsOverflowSignal::new()));
    let ws_for_timeframe = ws_matches_configured_timeframe(&config.market.timeframe);
    let (mut closed_kline_rx, ws_stream_task) = if ws_for_timeframe {
        match market_stream_plan(spot, &config.market.symbol) {
            Ok(plan) => {
                let (closed_kline_tx, closed_kline_rx) = mpsc::channel::<ClosedKline>(64);
                let task = spawn_market_kline_stream(
                    plan,
                    ws_shutdown.clone(),
                    Some(closed_kline_tx),
                    overflow_signal.clone(),
                );
                (Some(closed_kline_rx), Some(task))
            }
            Err(error) => {
                warn!(target: "exchanges::ws", error=%error, "WS market kline stream not started");
                (None, None)
            }
        }
    } else {
        info!(
            target: "exchanges::ws",
            timeframe = %config.market.timeframe,
            "Websocket kline stream disabled; only REST poll drives strategy for this timeframe"
        );
        (None, None)
    };
    if let Some(binance) = catalog().get(&spot.id.exchange) {
        info!(target: "exchanges", notes = %binance.notes, "Exchange capability catalog");
    }
    if let Some(quote) = config.market.symbol.split('/').nth(1) {
        if let Ok(asset) = Asset::new(quote) {
            let template = crate::modules::portfolio::paper_snapshot(&asset);
            let _ = template.validate();
            let _ = template.balance(&asset);
            info!(target: "portfolio", quote_asset = asset.as_str(), "Quote asset for configured symbol");
        }
    }
    for resource in ResourceManager::defaults_for_account(&spot.id) {
        if resource.enabled {
            info!(target: "exchanges", resource = %resource.name, kind = ?resource.kind, "Managed resource armed");
        }
    }

    let credentials = config.credentials()?;
    if credentials.api_key.is_some() {
        info!(target: "security", "Binance testnet credentials loaded from environment; values are redacted");
    } else {
        warn!(target: "security", "No testnet keys found; public market data only");
    }
    let exchange = Arc::new(BinanceMarketData::new(credentials, spot)?);
    let advisor = JevAdvisor::from_env(config.jev.clone())?;
    if advisor.is_some() {
        info!(target: "jev", "Jev advisor enabled for configured advisory questions");
    }
    let base_limits = RiskLimits {
        max_order_quote: config.risk.max_order_quote,
        max_daily_loss_quote: config.risk.max_daily_loss_quote,
        max_open_positions: config.risk.max_open_positions,
    };
    let limits = profile_limits(config.risk_profile, base_limits);
    let mut dashboard = new_dashboard(&config, limits);
    dashboard.persistence_status = PersistenceHealth::new(database.is_some()).label().into();
    let (event_tx, mut event_rx) = mpsc::channel(64);
    let (monitor_handle, command_rx, _) = MonitorHandle::channel(8, 64);
    let (state_tx, state_rx) = tokio::sync::watch::channel(
        crate::modules::monitor::views::terminal_dashboard::MonitorState::Running,
    );
    let (dashboard_tx, dashboard_rx) = watch::channel(dashboard.clone());
    if let Some(tx) = handle_tx {
        let _ = tx.send(monitor_handle.clone());
    }
    let mut ui_task = if headless {
        let monitor_pub = monitor_handle.clone();
        let initial_dashboard = dashboard.clone();
        tokio::spawn(async move {
            let mut revision: u64 = 0;
            let publish = |dash: &Dashboard, rev: u64| match monitor_snapshot_from_dashboard(
                dash, rev,
            ) {
                Ok(snapshot) => {
                    if let Err(error) = monitor_pub.publish_snapshot(snapshot) {
                        tracing::warn!(target: "api", ?error, "headless monitor snapshot publish failed");
                    }
                }
                Err(error) => {
                    tracing::warn!(target: "api", ?error, "headless monitor snapshot mapping failed");
                }
            };
            revision += 1;
            publish(&initial_dashboard, revision);
            let mut dash_rx = dashboard_rx;
            loop {
                tokio::select! {
                    changed = dash_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        revision += 1;
                        let dash = dash_rx.borrow().clone();
                        publish(&dash, revision);
                    }
                    msg = event_rx.recv() => {
                        if msg.is_none() {
                            break;
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        })
    } else {
        tokio::spawn(crate::presentation::terminal::run(
            monitor_handle.clone(),
            event_rx,
            state_rx,
            dashboard_rx,
        ))
    };

    let spot_account_id = spot.id.clone();
    let interval = time::interval(Duration::from_secs(config.market.poll_seconds));
    let persist_database = database.clone();
    let persist_symbol = config.market.symbol.clone();
    let persist_timeframe = config.market.timeframe.clone();
    let persist_rest: PersistRest = Arc::new(move |candles| {
        let db = persist_database.clone();
        let symbol = persist_symbol.clone();
        let timeframe = persist_timeframe.clone();
        Box::pin(async move {
            if let Some(db) = db {
                if timeframe == "1m" {
                    persist_polled_1m_window(&db, &symbol, candles).await
                } else {
                    Err(format!("unsupported persistence timeframe: {timeframe}"))
                }
            } else {
                Ok(())
            }
        })
    });
    let ws_database = database.clone();
    let ws_symbol = config.market.symbol.clone();
    let persist_ws: PersistWs = Arc::new(move |candle| {
        let db = ws_database.clone();
        let symbol = ws_symbol.clone();
        Box::pin(async move {
            if let Some(db) = db {
                persist_ws_1m_candle(&db, &symbol, candle).await
            } else {
                Ok(())
            }
        })
    });
    let mut market_task = tokio::spawn(run_market_loop(MarketLoop {
        config: config.clone(),
        limits,
        account: spot_account_id,
        source: exchange,
        review: Arc::new(move |snapshot| {
            let advisor = advisor.clone();
            Box::pin(async move { jev_note(&advisor, &snapshot).await })
        }),
        agent_hook: agent_hook.clone(),
        persistence_enabled,
        persist_rest,
        persist_ws,
        ws_rx: closed_kline_rx.take(),
        overflow_signal,
        commands: command_rx,
        events: event_tx,
        dashboard_tx,
        state_tx,
        interval,
        now_ms: Arc::new(|| chrono::Utc::now().timestamp_millis()),
    }));
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            let _ = monitor_handle.send(MonitorCommand::Shutdown);
            let _ = market_task.await;
            if let Ok(Err(error)) = ui_task.await {
                warn!(target: "ui", %error, "TUI task ended with an error");
            }
        },
        result = &mut ui_task => {
            if let Ok(Err(error)) = result {
                warn!(target: "ui", %error, "TUI task ended with an error");
            }
            let _ = monitor_handle.send(MonitorCommand::Shutdown);
            let _ = market_task.await;
        },
        _ = &mut market_task => {
            if let Ok(Err(error)) = ui_task.await {
                warn!(target: "ui", %error, "TUI task ended with an error");
            }
        },
    }
    drop(monitor_handle);
    ws_shutdown.cancel();
    if let Some(handle) = ws_stream_task {
        let _ = handle.await;
    }
    // The monitor and UI are shut down by their respective commands/cancellation paths.
    info!(target: "system", environment=%config.environment, "Shutdown complete; no live-order action was performed");
    Ok(())
}

/// Headless monitor for HTTP `serve --with-monitor` (no Ratatui).
pub async fn spawn_headless_for_api(
    config: Config,
    database: Option<Database>,
    agent_hook: Arc<dyn MonitorAgentHook>,
) -> BotResult<(MonitorHandle, JoinHandle<BotResult<()>>)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let join = tokio::spawn(async move {
        run_with_agent_hook_inner(config, database, agent_hook, true, Some(tx)).await
    });
    let handle = rx.await.map_err(|_| {
        BotError::Configuration("headless monitor exited before exposing handle".into())
    })?;
    Ok((handle, join))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::config::StrategyConfig,
        modules::exchanges::{ExchangeId, MarketType},
    };
    use std::{collections::VecDeque, sync::Mutex};

    fn test_market_loop(
        source: Arc<dyn MarketDataSource>,
        ws_rx: Option<mpsc::Receiver<ClosedKline>>,
    ) -> (
        MonitorHandle,
        mpsc::Receiver<AppEvent>,
        watch::Receiver<MonitorState>,
        JoinHandle<()>,
    ) {
        test_market_loop_with(
            source,
            ws_rx,
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3_600),
        )
    }

    fn test_market_loop_with(
        source: Arc<dyn MarketDataSource>,
        ws_rx: Option<mpsc::Receiver<ClosedKline>>,
        review: Review,
        persist_rest: PersistRest,
        poll_interval: Duration,
    ) -> (
        MonitorHandle,
        mpsc::Receiver<AppEvent>,
        watch::Receiver<MonitorState>,
        JoinHandle<()>,
    ) {
        test_market_loop_with_health(
            source,
            ws_rx,
            review,
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            poll_interval,
            false,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)] // Integration tests inject market, REST/WS writes, and WS loss paths separately.
    fn test_market_loop_with_health(
        source: Arc<dyn MarketDataSource>,
        ws_rx: Option<mpsc::Receiver<ClosedKline>>,
        review: Review,
        persist_rest: PersistRest,
        persist_ws: PersistWs,
        poll_interval: Duration,
        persistence_enabled: bool,
        overflow_signal: Option<Arc<WsOverflowSignal>>,
    ) -> (
        MonitorHandle,
        mpsc::Receiver<AppEvent>,
        watch::Receiver<MonitorState>,
        JoinHandle<()>,
    ) {
        let config = Config {
            market: crate::core::config::MarketConfig {
                timeframe: "1m".into(),
                poll_seconds: poll_interval.as_secs().max(1),
                ..Config::default().market
            },
            strategy: StrategyConfig {
                sma_fast: 2,
                sma_slow: 3,
                ..Config::default().strategy
            },
            ..Config::default()
        };
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            config.environment,
        )
        .unwrap();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let (monitor_handle, command_rx, _) = MonitorHandle::channel(8, 64);
        let (events, event_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(MonitorState::Running);
        let (dashboard_tx, _dashboard_rx) = watch::channel(new_dashboard(&config, limits));
        let handle = tokio::spawn(run_market_loop(MarketLoop {
            config,
            limits,
            account,
            source,
            review,
            agent_hook: std::sync::Arc::new(NoopMonitorAgentHook)
                as std::sync::Arc<dyn MonitorAgentHook>,
            persistence_enabled,
            persist_rest,
            persist_ws,
            ws_rx,
            overflow_signal,
            commands: command_rx,
            events,
            dashboard_tx,
            state_tx,
            interval: time::interval(poll_interval),
            now_ms: Arc::new(|| 420_000),
        }));
        (monitor_handle, event_rx, state_rx, handle)
    }

    struct BlockingMarketSource {
        started: tokio::sync::Notify,
        release: tokio::sync::Notify,
        window: Vec<mantis_ta::types::Candle>,
    }

    #[async_trait::async_trait]
    impl MarketDataSource for BlockingMarketSource {
        async fn candles(&self, _: &str, _: &str, _: u32) -> BotResult<RestWindow> {
            self.started.notify_one();
            self.release.notified().await;
            Ok(self.window.clone())
        }
    }

    struct ControlledMarketSource {
        started: tokio::sync::Notify,
        release: tokio::sync::Semaphore,
        results: Mutex<VecDeque<BotResult<RestWindow>>>,
    }

    #[async_trait::async_trait]
    impl MarketDataSource for ControlledMarketSource {
        async fn candles(&self, _: &str, _: &str, _: u32) -> BotResult<RestWindow> {
            self.started.notify_one();
            let permit = self.release.acquire().await.unwrap();
            permit.forget();
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .expect("test REST response")
        }
    }

    fn ws_bar(timestamp_ms: i64) -> ClosedKline {
        ClosedKline {
            symbol: "BTC/USDT".into(),
            candle: Candle {
                timestamp_ms,
                open: 10.0,
                high: 10.0,
                low: 10.0,
                close: 10.0,
                volume: 1.0,
            },
        }
    }

    async fn next_market_refresh(events: &mut mpsc::Receiver<AppEvent>) -> Dashboard {
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(next)) = events.recv().await {
                    if next.market.is_some() {
                        break next;
                    }
                }
            }
        })
        .await
        .unwrap()
    }

    async fn wait_ws_consumed(sender: &mpsc::Sender<ClosedKline>) {
        time::timeout(Duration::from_secs(1), async {
            while sender.capacity() < 8 {
                tokio::task::yield_now().await;
            }
            tokio::task::yield_now().await;
        })
        .await
        .unwrap();
    }

    fn test_window(latest: i64) -> RestWindow {
        (0..4)
            .map(|offset| {
                let close = [5.0, 5.0, 1.0, 10.0][offset];
                mantis_ta::types::Candle {
                    timestamp: latest - (3 - offset as i64) * 60_000,
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: 1.0,
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn pause_is_confirmed_while_rest_is_blocked_and_old_result_cannot_publish() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(300_000),
        });
        let (commands, mut events, mut state, handle) = test_market_loop(source.clone(), None);
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Paused);
        source.release.notify_one();
        tokio::task::yield_now().await;
        while let Ok(AppEvent::Refresh(next)) = events.try_recv() {
            assert!(next.market.is_none());
        }
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn resume_drains_ws_backlog_and_evaluates_only_fresh_rest_once() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let (ws_tx, ws_rx) = mpsc::channel(128);
        let (commands, mut events, mut state, handle) =
            test_market_loop(source.clone(), Some(ws_rx));
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        for _ in 0..70 {
            ws_tx
                .send(ClosedKline {
                    symbol: "BTC/USDT".into(),
                    candle: Candle {
                        timestamp_ms: 300_000,
                        open: 1.0,
                        high: 1.0,
                        low: 1.0,
                        close: 1.0,
                        volume: 1.0,
                    },
                })
                .await
                .unwrap();
        }
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Resuming);
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Running);
        let refresh = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(next)) = events.recv().await {
                    if next.market.is_some() {
                        break next;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(refresh.market.unwrap().candle_timestamp_ms, 360_000);
        assert!(events.try_recv().is_err());
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn running_discards_rest_window_older_than_ws_bar() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(240_000),
        });
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (persist_tx, mut persisted) = mpsc::unbounded_channel();
        let persist_rest: PersistRest = Arc::new(move |candles| {
            let tx = persist_tx.clone();
            Box::pin(async move {
                let _ = tx.send(candles.last().unwrap().timestamp);
                Ok(())
            })
        });
        let (commands, mut events, _state, handle) = test_market_loop_with(
            source.clone(),
            Some(ws_rx),
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Duration::from_secs(3_600),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        ws_tx.send(ws_bar(300_000)).await.unwrap();
        let first = next_market_refresh(&mut events).await;
        assert_eq!(first.market.unwrap().candle_timestamp_ms, 300_000);
        source.release.notify_one();
        assert!(time::timeout(Duration::from_millis(100), persisted.recv())
            .await
            .is_err());
        while let Ok(AppEvent::Refresh(next)) = events.try_recv() {
            assert_ne!(
                next.market
                    .as_ref()
                    .map(|market| market.candle_timestamp_ms),
                Some(240_000)
            );
        }
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn future_ws_does_not_evaluate_or_block_rest_resume() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, mut state, handle) =
            test_market_loop(source.clone(), Some(ws_rx));
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        ws_tx.send(ws_bar(420_000)).await.unwrap();
        tokio::task::yield_now().await;
        assert!(events.try_recv().is_err());
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        ws_tx.send(ws_bar(420_000)).await.unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Running);
        assert_eq!(
            next_market_refresh(&mut events)
                .await
                .market
                .unwrap()
                .candle_timestamp_ms,
            360_000
        );
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn rest_poll_error_preserves_ws_evaluation_and_monitor_state() {
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
            results: Mutex::new(VecDeque::from(vec![
                Ok(test_window(300_000)),
                Err(BotError::MarketData("redirect rejected".into())),
            ])),
        });
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, _state, handle) = test_market_loop_with(
            source.clone(),
            Some(ws_rx),
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_millis(50),
        );

        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.add_permits(1);
        assert_eq!(
            next_market_refresh(&mut events)
                .await
                .market
                .unwrap()
                .candle_timestamp_ms,
            300_000
        );

        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.add_permits(1);
        let rest_error = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(next)) = events.recv().await {
                    if next.last_error.as_deref() == Some("market data error: redirect rejected") {
                        break next;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(rest_error.monitor_state, MonitorState::Running);
        assert_eq!(rest_error.market.unwrap().candle_timestamp_ms, 300_000);

        ws_tx.send(ws_bar(360_000)).await.unwrap();
        let recovered = next_market_refresh(&mut events).await;
        assert_eq!(recovered.market.unwrap().candle_timestamp_ms, 360_000);
        assert_eq!(recovered.monitor_state, MonitorState::Running);
        assert_eq!(recovered.last_error, None);

        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn failed_resume_stays_resuming_until_next_valid_rest_poll_without_ws() {
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
            results: Mutex::new(VecDeque::from(vec![
                Err(BotError::MarketData("REST unavailable".into())),
                Ok(test_window(360_000)),
            ])),
        });
        let (commands, mut events, mut state, handle) = test_market_loop_with(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_millis(50),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        source.release.add_permits(1);
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(next)) = events.recv().await {
                    if next.last_error.is_some() {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Resuming);
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.add_permits(1);
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Running);
        assert_eq!(
            next_market_refresh(&mut events)
                .await
                .market
                .unwrap()
                .candle_timestamp_ms,
            360_000
        );
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn blocked_advisory_for_a_cannot_publish_after_ws_b_arrives() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(180_000),
        });
        let advisory_started = Arc::new(tokio::sync::Notify::new());
        let advisory_release = Arc::new(tokio::sync::Notify::new());
        let review: Review = {
            let started = advisory_started.clone();
            let release = advisory_release.clone();
            Arc::new(move |snapshot| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    if snapshot.candle_timestamp_ms == 180_000 {
                        started.notify_one();
                        release.notified().await;
                    }
                    None
                })
            })
        };
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, _state, handle) = test_market_loop_with(
            source.clone(),
            Some(ws_rx),
            review,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3_600),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), advisory_started.notified())
            .await
            .unwrap();
        ws_tx.send(ws_bar(240_000)).await.unwrap();
        wait_ws_consumed(&ws_tx).await;
        advisory_release.notify_one();
        let evaluated = next_market_refresh(&mut events).await;
        assert_eq!(evaluated.market.unwrap().candle_timestamp_ms, 240_000);
        while let Ok(AppEvent::Refresh(next)) = events.try_recv() {
            assert_ne!(
                next.market.as_ref().map(|m| m.candle_timestamp_ms),
                Some(180_000)
            );
        }
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn first_resume_rest_candidate_a_is_discarded_after_ws_b() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(300_000),
        });
        let advisory_started = Arc::new(tokio::sync::Notify::new());
        let advisory_release = Arc::new(tokio::sync::Notify::new());
        let review: Review = {
            let started = advisory_started.clone();
            let release = advisory_release.clone();
            Arc::new(move |snapshot| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    if snapshot.candle_timestamp_ms == 300_000 {
                        started.notify_one();
                        release.notified().await;
                    }
                    None
                })
            })
        };
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, mut state, handle) = test_market_loop_with(
            source.clone(),
            Some(ws_rx),
            review,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3_600),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Running);
        time::timeout(Duration::from_secs(1), advisory_started.notified())
            .await
            .unwrap();
        ws_tx.send(ws_bar(360_000)).await.unwrap();
        wait_ws_consumed(&ws_tx).await;
        advisory_release.notify_one();
        let evaluated = next_market_refresh(&mut events).await;
        assert_eq!(evaluated.market.unwrap().candle_timestamp_ms, 360_000);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn pause_cancels_blocked_advisory_without_publishing_signal() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(180_000),
        });
        let advisory_started = Arc::new(tokio::sync::Notify::new());
        let advisory_release = Arc::new(tokio::sync::Notify::new());
        let review: Review = {
            let started = advisory_started.clone();
            let release = advisory_release.clone();
            Arc::new(move |_| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    started.notify_one();
                    release.notified().await;
                    None
                })
            })
        };
        let (commands, mut events, mut state, handle) = test_market_loop_with(
            source.clone(),
            None,
            review,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3_600),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), advisory_started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Paused);
        advisory_release.notify_one();
        tokio::task::yield_now().await;
        while let Ok(AppEvent::Refresh(next)) = events.try_recv() {
            assert!(next.market.is_none());
        }
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn stale_rest_resume_retries_and_closed_ws_does_not_hold_resuming() {
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
            results: Mutex::new(VecDeque::from(vec![
                Ok(test_window(180_000)),
                Ok(test_window(360_000)),
            ])),
        });
        let (ws_tx, ws_rx) = mpsc::channel(1);
        drop(ws_tx);
        let (commands, mut events, mut state, handle) = test_market_loop_with(
            source.clone(),
            Some(ws_rx),
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_millis(50),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        source.release.add_permits(1);
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(next)) = events.recv().await {
                    if next.last_error.is_some() {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Resuming);
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.add_permits(1);
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Running);
        assert_eq!(
            next_market_refresh(&mut events)
                .await
                .market
                .unwrap()
                .candle_timestamp_ms,
            360_000
        );
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn quit_remains_responsive_while_persistence_is_blocked() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let persist_started = Arc::new(tokio::sync::Notify::new());
        let persist_release = Arc::new(tokio::sync::Notify::new());
        let persist_rest: PersistRest = {
            let started = persist_started.clone();
            let release = persist_release.clone();
            Arc::new(move |_| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    started.notify_one();
                    release.notified().await;
                    Ok(())
                })
            })
        };
        let (commands, mut events, mut state, handle) = test_market_loop_with_health(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3_600),
            true,
            None,
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), persist_started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Paused);
        while let Ok(AppEvent::Refresh(snapshot)) = events.try_recv() {
            assert!(snapshot.persistence_status.starts_with("DEGRADED"));
        }
        commands.send(MonitorCommand::Shutdown).unwrap();
        time::timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap();
        persist_release.notify_one();
    }

    #[tokio::test]
    async fn rest_commit_establishes_visible_session_health_with_fake_store() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let persist_rest: PersistRest = {
            let started = started.clone();
            let release = release.clone();
            Arc::new(move |_| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    started.notify_one();
                    release.notified().await;
                    Ok(())
                })
            })
        };
        let (commands, mut events, _state, handle) = test_market_loop_with_health(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3600),
            true,
            None,
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), started.notified())
            .await
            .unwrap();
        let market = next_market_refresh(&mut events).await;
        assert!(market.persistence_status.starts_with("DEGRADED"));
        release.notify_one();
        let healthy = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(healthy.market.unwrap().candle_timestamp_ms, 360_000);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn pause_during_unconfirmed_rest_write_keeps_degraded_after_late_success() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let persist_rest: PersistRest = {
            let started = started.clone();
            let release = release.clone();
            Arc::new(move |_| {
                let started = started.clone();
                let release = release.clone();
                Box::pin(async move {
                    started.notify_one();
                    release.notified().await;
                    Ok(())
                })
            })
        };
        let (commands, mut events, mut state, handle) = test_market_loop_with_health(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3600),
            true,
            None,
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), started.notified())
            .await
            .unwrap();
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*state.borrow(), MonitorState::Paused);
        while let Ok(AppEvent::Refresh(snapshot)) = events.try_recv() {
            assert!(snapshot.persistence_status.starts_with("DEGRADED"));
        }
        release.notify_one();
        let completed = time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();
        let AppEvent::Refresh(snapshot) = completed;
        assert_eq!(snapshot.monitor_state, MonitorState::Paused);
        assert!(snapshot.persistence_status.starts_with("DEGRADED"));
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn overflow_and_short_recovery_window_show_sticky_gap_even_if_write_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
            results: Mutex::new(VecDeque::from([
                Ok(test_window(360_000)),
                Ok(test_window(360_000)),
            ])),
        });
        let losses = Arc::new(WsOverflowSignal::new());
        let writes = Arc::new(AtomicUsize::new(0));
        let persist_rest: PersistRest = {
            let writes = writes.clone();
            Arc::new(move |_| {
                let attempt = writes.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    if attempt == 0 {
                        Ok(())
                    } else {
                        Err("database-write".into())
                    }
                })
            })
        };
        let (commands, mut events, mut state, handle) = test_market_loop_with_health(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3600),
            true,
            Some(losses.clone()),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.add_permits(1);
        let _healthy = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        losses.record(120_000);
        let degraded = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("DEGRADED") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(degraded.market.unwrap().candle_timestamp_ms, 360_000);
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        source.release.add_permits(1);
        let gap = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("GAP") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(gap.market.unwrap().candle_timestamp_ms, 360_000);
        let failed_write = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot
                        .logs
                        .iter()
                        .any(|line| line.contains("Persistence rest database-write"))
                    {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert!(failed_write.persistence_status.starts_with("GAP"));
        assert_eq!(writes.load(Ordering::SeqCst), 2);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn failed_ws_write_degrades_archive_without_stopping_market() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, _state, handle) = test_market_loop_with_health(
            source.clone(),
            Some(ws_rx),
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Arc::new(|_| Box::pin(async { Err("postgresql://secret@host".into()) })),
            Duration::from_secs(3600),
            true,
            None,
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        ws_tx.send(ws_bar(360_000)).await.unwrap();
        let degraded = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("DEGRADED") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(degraded.market.unwrap().candle_timestamp_ms, 360_000);
        assert!(!degraded.persistence_status.contains("secret"));
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn ws_disconnection_marks_session_degraded_after_baseline() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let losses = Arc::new(WsOverflowSignal::new());
        let (commands, mut events, _state, handle) = test_market_loop_with_health(
            source.clone(),
            None,
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3600),
            true,
            Some(losses.clone()),
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        losses.record_disconnect();
        let degraded = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("DEGRADED") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(degraded.market.unwrap().candle_timestamp_ms, 360_000);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn failed_rest_commit_keeps_paper_market_active_until_confirmed_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(2),
            results: Mutex::new(VecDeque::from([
                Ok(test_window(360_000)),
                Ok(test_window(360_000)),
            ])),
        });
        let writes = Arc::new(AtomicUsize::new(0));
        let persist_rest: PersistRest = {
            let writes = writes.clone();
            Arc::new(move |_| {
                let attempt = writes.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    if attempt == 0 {
                        Err("uncertain commit with secret".into())
                    } else {
                        Ok(())
                    }
                })
            })
        };
        let (commands, mut events, _state, handle) = test_market_loop_with_health(
            source,
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_millis(50),
            true,
            None,
        );
        let first_market = next_market_refresh(&mut events).await;
        assert_eq!(first_market.market.unwrap().candle_timestamp_ms, 360_000);
        assert!(first_market.persistence_status.starts_with("DEGRADED"));
        let healthy = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(healthy.market.unwrap().candle_timestamp_ms, 360_000);
        assert!(writes.load(Ordering::SeqCst) >= 2);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn timed_out_rest_commit_stays_degraded_until_confirmed_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let source = Arc::new(ControlledMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(2),
            results: Mutex::new(VecDeque::from([
                Ok(test_window(360_000)),
                Ok(test_window(360_000)),
            ])),
        });
        let attempts = Arc::new(AtomicUsize::new(0));
        let persist_rest: PersistRest = {
            let attempts = attempts.clone();
            Arc::new(move |_| {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    if attempt == 0 {
                        std::future::pending().await
                    } else {
                        Ok(())
                    }
                })
            })
        };
        let (commands, mut events, mut state, handle) = test_market_loop_with_health(
            source,
            None,
            Arc::new(|_| Box::pin(async { None })),
            persist_rest,
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Duration::from_secs(3600),
            true,
            None,
        );
        let timeout_snapshot = time::timeout(Duration::from_secs(6), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot
                        .logs
                        .iter()
                        .any(|line| line.contains("Persistence rest timeout"))
                    {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert!(timeout_snapshot.persistence_status.starts_with("DEGRADED"));
        commands.send(MonitorCommand::Pause).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        commands.send(MonitorCommand::Resume).unwrap();
        time::timeout(Duration::from_secs(1), state.changed())
            .await
            .unwrap()
            .unwrap();
        let healthy = time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(healthy.market.unwrap().candle_timestamp_ms, 360_000);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn timed_out_ws_write_keeps_archive_degraded() {
        let source = Arc::new(BlockingMarketSource {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            window: test_window(360_000),
        });
        let (ws_tx, ws_rx) = mpsc::channel(8);
        let (commands, mut events, _state, handle) = test_market_loop_with_health(
            source.clone(),
            Some(ws_rx),
            Arc::new(|_| Box::pin(async { None })),
            Arc::new(|_| Box::pin(async { Ok(()) })),
            Arc::new(|_| Box::pin(async { std::future::pending().await })),
            Duration::from_secs(3600),
            true,
            None,
        );
        time::timeout(Duration::from_secs(1), source.started.notified())
            .await
            .unwrap();
        source.release.notify_one();
        time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot.persistence_status.starts_with("HEALTHY") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        ws_tx.send(ws_bar(360_000)).await.unwrap();
        let timeout_snapshot = time::timeout(Duration::from_secs(6), async {
            loop {
                if let Some(AppEvent::Refresh(snapshot)) = events.recv().await {
                    if snapshot
                        .logs
                        .iter()
                        .any(|line| line.contains("Persistence ws timeout"))
                    {
                        break snapshot;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert!(timeout_snapshot.persistence_status.starts_with("DEGRADED"));
        commands.send(MonitorCommand::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[test]
    fn database_error_details_are_reduced_to_safe_class() {
        assert_eq!(
            persistence_error_class("postgresql://secret@host/trading_bot"),
            "database-write"
        );
        assert_eq!(persistence_error_class("ws-write timeout"), "timeout");
        assert_eq!(persistence_error_class("validation"), "validation");
    }

    #[test]
    fn ws_losses_do_not_create_persistence_warning_when_opted_out() {
        let losses = Arc::new(WsOverflowSignal::new());
        losses.record(120_000);
        losses.record_disconnect();
        let mut health = PersistenceHealth::new(false);
        apply_ws_losses(&Some(losses.clone()), &mut health);
        assert_eq!(health.state(), PersistenceState::Off);
        assert_eq!(health.label(), "OFF");
        assert_eq!(losses.take(), Some(120_000));
        assert!(losses.take_disconnect());
    }

    #[tokio::test]
    async fn latest_dashboard_error_survives_full_refresh_queue() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let mut dashboard = new_dashboard(&config, limits);
        let (events, _queued) = mpsc::channel(1);
        events
            .try_send(AppEvent::Refresh(dashboard.clone()))
            .unwrap();
        let (latest_tx, latest_rx) = watch::channel(dashboard.clone());
        dashboard.monitor_state = MonitorState::Resuming;
        dashboard.set_error("REST unavailable".into());
        publish_dashboard(&dashboard, &latest_tx, &events);
        assert_eq!(latest_rx.borrow().monitor_state, MonitorState::Resuming);
        assert_eq!(
            latest_rx.borrow().last_error.as_deref(),
            Some("REST unavailable")
        );
    }

    #[test]
    fn monitor_spot_client_order_id_formats_compact_symbol() {
        use crate::modules::orders::OrderSide;
        assert_eq!(
            monitor_spot_client_order_id("BTC/USDT", 1_234, OrderSide::Sell),
            "mon:BTCUSDT:1234:sell"
        );
    }

    #[test]
    fn paper_fill_unit_price_from_close_rejects_non_positive() {
        assert_eq!(paper_fill_unit_price_from_close(0.0), None);
        assert_eq!(paper_fill_unit_price_from_close(42.5), Some(42.5));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn testnet_run_mode_submits_via_spot_executor_when_recording_seam() {
        use crate::core::config::RunMode;
        use crate::core::test_env_lock::EnvTestGuard;
        use crate::modules::exchanges::{ExchangeId, MarketType};
        use crate::modules::orders::{
            lock_shared_live_order_reconciliation_ledger_for_test,
            shared_live_order_reconciliation_ledger, OrderReconciliationLedger,
            PaperLedgerExecutor, ReconciliationState, RecordingSpotOrderSubmitPort,
        };

        let _env = EnvTestGuard::acquire();
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        RecordingSpotOrderSubmitPort::clear();
        PaperLedgerExecutor::clear_ledger();
        let config = Config {
            run_mode: RunMode::Testnet,
            ..Default::default()
        };
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            config.environment,
        )
        .unwrap();
        let (events, _event_rx) = mpsc::channel(2);
        let (dashboard_tx, _dashboard_rx) = watch::channel(new_dashboard(&config, limits));
        let settings = EvaluationSettings {
            config: &config,
            limits,
            spot_account_id: &account,
            event_tx: &events,
            dashboard_tx: &dashboard_tx,
        };
        let mut dashboard = new_dashboard(&config, limits);
        let snapshot = StrategySnapshot {
            signal: Signal::Buy,
            close: 50_000.0,
            fast_sma: Some(1.0),
            slow_sma: Some(0.5),
            candle_timestamp_ms: 360_000,
        };
        let mut execution = ExecutionContext::default();
        assert!(
            apply_strategy_snapshot(
                snapshot,
                None,
                "rest",
                None,
                &settings,
                &mut execution,
                &mut dashboard,
            )
            .await
        );
        assert_eq!(RecordingSpotOrderSubmitPort::call_count(), 1);
        assert!(PaperLedgerExecutor::recorded_fills().is_empty());
        let ledger = shared_live_order_reconciliation_ledger();
        assert_eq!(
            ledger.state("mon:BTCUSDT:360000:buy"),
            Some(ReconciliationState::Reconciled {
                exchange_order_id: "recording-1".into(),
            })
        );
        RecordingSpotOrderSubmitPort::clear();
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    }

    #[tokio::test]
    async fn paper_run_mode_records_ledger_fill_at_candle_close() {
        use crate::core::config::RunMode;
        use crate::modules::exchanges::{ExchangeId, MarketType};
        use crate::modules::orders::{OrderSide, PaperLedgerExecutor};

        PaperLedgerExecutor::clear_ledger();
        let config = Config {
            run_mode: RunMode::Paper,
            ..Default::default()
        };
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            config.environment,
        )
        .unwrap();
        let (events, _event_rx) = mpsc::channel(2);
        let (dashboard_tx, _dashboard_rx) = watch::channel(new_dashboard(&config, limits));
        let settings = EvaluationSettings {
            config: &config,
            limits,
            spot_account_id: &account,
            event_tx: &events,
            dashboard_tx: &dashboard_tx,
        };
        let mut dashboard = new_dashboard(&config, limits);
        let snapshot = StrategySnapshot {
            signal: Signal::Buy,
            close: 50_000.0,
            fast_sma: Some(1.0),
            slow_sma: Some(0.5),
            candle_timestamp_ms: 360_000,
        };
        let mut execution = ExecutionContext::default();
        assert!(
            apply_strategy_snapshot(
                snapshot,
                None,
                "rest",
                None,
                &settings,
                &mut execution,
                &mut dashboard,
            )
            .await
        );
        let fills = PaperLedgerExecutor::recorded_fills();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].side, OrderSide::Buy);
        assert_eq!(fills[0].fill_unit_price, Some(50_000.0));
        assert_eq!(execution.open_positions, 1);
        use crate::modules::http_bridge::portfolio::{paper_wallet_snapshot, PaperSnapshotQuery};
        let http_snap = paper_wallet_snapshot(PaperSnapshotQuery {
            quote: "usdt".into(),
        })
        .expect("monitor paper fill visible via http_bridge portfolio");
        assert_eq!(http_snap.available, "990");
        assert_eq!(http_snap.positions.len(), 1);
        PaperLedgerExecutor::clear_ledger();
    }

    #[tokio::test]
    async fn persistence_gap_warning_survives_market_evaluation() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            config.environment,
        )
        .unwrap();
        let (events, _event_rx) = mpsc::channel(2);
        let (dashboard_tx, dashboard_rx) = watch::channel(new_dashboard(&config, limits));
        let settings = EvaluationSettings {
            config: &config,
            limits,
            spot_account_id: &account,
            event_tx: &events,
            dashboard_tx: &dashboard_tx,
        };
        let mut dashboard = new_dashboard(&config, limits);
        dashboard.persistence_status = "GAP · reconciliação externa necessária".into();
        let snapshot = StrategySnapshot {
            signal: Signal::Warmup,
            close: 10.0,
            fast_sma: None,
            slow_sma: None,
            candle_timestamp_ms: 360_000,
        };
        let mut execution = ExecutionContext::default();
        assert!(
            apply_strategy_snapshot(
                snapshot,
                None,
                "rest",
                None,
                &settings,
                &mut execution,
                &mut dashboard
            )
            .await
        );
        assert_eq!(
            dashboard_rx.borrow().persistence_status,
            "GAP · reconciliação externa necessária"
        );
    }
}
