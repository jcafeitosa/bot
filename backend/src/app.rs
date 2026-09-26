use std::{future::Future, sync::Arc};
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::ui::{self, AppEvent, Dashboard, UiCommand};
use crate::{
    config::Config,
    domain::BotSignal,
    error::{BotError, BotResult},
    exchanges::{
        binance::BinanceMarketData,
        bootstrap::{load_registry, registered_market_types, spot_account_for_symbol},
        capabilities::catalog,
        live::{market_stream_plan, spawn_market_kline_stream, ClosedKline},
        preflight::log_transport_plan,
        resources::ResourceManager,
        rest::{authorize_rest_use, RestUse},
        router::{rest_use_for_need, route_market_need, MarketNeed},
        ExchangeAccountId, MarketDataSource,
    },
    jev::JevAdvisor,
    market::{Candle, HistoricalDataset},
    market_feed::{ws_matches_configured_timeframe, HybridCandleFeed},
    persistence::{persist_market_data_enabled, Database},
    portfolio::Asset,
    risk::{gate_signal, profile_limits, ExecutionContext, RiskLimits},
    strategy::{evaluate, Signal, StrategySnapshot},
};

async fn persist_polled_1m_window(
    db: &Database,
    symbol: &str,
    candles: Vec<mantis_ta::types::Candle>,
) {
    match HistoricalDataset::from_mantis_1m(symbol, "monitor-rest", candles) {
        Ok(dataset) => {
            if let Err(error) = db.persist_dataset(&dataset).await {
                warn!(target: "persistence", error=%error, "Failed to persist polled 1m candles");
            }
        }
        Err(error) => {
            warn!(target: "persistence", error=%error, "Skipped persist: polled window is not a valid 1m series");
        }
    }
}

async fn persist_ws_1m_candle(db: &Database, symbol: &str, candle: Candle) {
    match HistoricalDataset::from_1m(symbol, "monitor-ws", vec![candle]) {
        Ok(dataset) => {
            if let Err(error) = db.persist_dataset(&dataset).await {
                warn!(target: "persistence", error=%error, "Failed to persist WS closed 1m candle");
            }
        }
        Err(error) => {
            warn!(target: "persistence", error=%error, "Skipped persist: WS candle failed validation");
        }
    }
}

async fn jev_note(advisor: &Option<JevAdvisor>, snapshot: &StrategySnapshot) -> Option<String> {
    if let Some(jev) = advisor {
        match jev.review(snapshot).await {
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
    advisor: &'a Option<JevAdvisor>,
    spot_account_id: &'a ExchangeAccountId,
    event_tx: &'a mpsc::Sender<AppEvent>,
}

async fn apply_strategy_snapshot(
    snapshot: StrategySnapshot,
    note: Option<String>,
    feed_source: &str,
    settings: &EvaluationSettings<'_>,
    execution_ctx: &mut ExecutionContext,
    dashboard: &mut Dashboard,
) -> bool {
    let EvaluationSettings {
        config,
        limits,
        spot_account_id,
        event_tx,
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
        bot_id: None,
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
        match gate_signal(snapshot.signal, *limits, config.run_mode, execution_ctx) {
            Ok(()) => warn!(
                target: "risk",
                environment=%config.environment,
                mode=%config.run_mode,
                quote_cap=limits.max_order_quote,
                open_positions=execution_ctx.open_positions,
                "Strategy intent passed risk gate; live orders remain disabled in v1"
            ),
            Err(e) => warn!(target: "risk", error=%e, "Strategy intent rejected by risk gate"),
        }
    }
    dashboard.set_paper_open_positions(execution_ctx.open_positions);
    dashboard.update_market(snapshot, note);
    dashboard.push_log(format!(
        "{} | {} | {} | {}",
        config.market.symbol, config.operation, config.risk_profile, feed_source
    ));
    event_tx
        .send(AppEvent::Refresh(dashboard.clone()))
        .await
        .is_ok()
}

async fn run_evaluation_cycle(
    feed: &mut HybridCandleFeed,
    candle_ts: i64,
    feed_source: &str,
    settings: &EvaluationSettings<'_>,
    execution_ctx: &mut ExecutionContext,
    dashboard: &mut Dashboard,
) -> bool {
    if !feed.ready_for_evaluation(
        settings.config.strategy.sma_slow,
        &settings.config.market.timeframe,
    ) {
        warn!(target: "strategy", candle_ts, feed_source, "Strategy warmup: waiting for contiguous closed candles");
        let snapshot = StrategySnapshot {
            signal: Signal::Warmup,
            close: feed.candles().last().map_or(0.0, |bar| bar.close),
            fast_sma: None,
            slow_sma: None,
            candle_timestamp_ms: candle_ts,
        };
        return apply_strategy_snapshot(
            snapshot,
            None,
            feed_source,
            settings,
            execution_ctx,
            dashboard,
        )
        .await;
    }
    let snapshot = evaluate(
        feed.candles(),
        settings.config.strategy.sma_fast,
        settings.config.strategy.sma_slow,
    );
    let note = jev_note(settings.advisor, &snapshot).await;
    if snapshot.signal != Signal::Warmup {
        feed.mark_evaluated(candle_ts);
    }
    apply_strategy_snapshot(
        snapshot,
        note,
        feed_source,
        settings,
        execution_ctx,
        dashboard,
    )
    .await
}

async fn recv_ws_kline(rx: &mut Option<mpsc::Receiver<ClosedKline>>) -> Option<ClosedKline> {
    match rx {
        Some(channel) => channel.recv().await,
        None => std::future::pending().await,
    }
}

enum MarketWakeup {
    Ws(ClosedKline),
    RestPoll,
}

async fn next_market_wakeup(
    rx: &mut Option<mpsc::Receiver<ClosedKline>>,
    interval: &mut time::Interval,
) -> MarketWakeup {
    loop {
        tokio::select! {
            biased;
            kline = recv_ws_kline(rx) => match kline {
                Some(kline) => return MarketWakeup::Ws(kline),
                None => {
                    warn!(target: "exchanges::ws", "Websocket channel ended; REST polling remains active");
                    *rx = None;
                }
            },
            _ = interval.tick() => return MarketWakeup::RestPoll,
        }
    }
}

async fn fetch_authorized_candles<S: MarketDataSource>(
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

async fn stage_polled_window<P, Fut>(
    polled: BotResult<Vec<mantis_ta::types::Candle>>,
    feed: &mut HybridCandleFeed,
    persist: P,
) -> BotResult<Option<i64>>
where
    P: FnOnce(Vec<mantis_ta::types::Candle>) -> Fut,
    Fut: Future<Output = ()>,
{
    let candles = polled?;
    if candles.is_empty() {
        warn!(target: "market", "Exchange returned no completed candles");
        return Ok(None);
    }
    persist(candles.clone()).await;
    Ok(feed.ingest_rest_window(candles))
}

pub async fn run(config: Config, database: Option<Database>) -> BotResult<()> {
    let registry = load_registry(config.environment)
        .map_err(|e| crate::error::BotError::Configuration(e.to_string()))?;
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
    let ws_for_timeframe = ws_matches_configured_timeframe(&config.market.timeframe);
    let (mut closed_kline_rx, ws_stream_task) = if ws_for_timeframe {
        match market_stream_plan(spot, &config.market.symbol) {
            Ok(plan) => {
                let (closed_kline_tx, closed_kline_rx) = mpsc::channel::<ClosedKline>(64);
                let task =
                    spawn_market_kline_stream(plan, ws_shutdown.clone(), Some(closed_kline_tx));
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
    if persist_market_data_enabled() && database.is_none() {
        warn!(
            target: "persistence",
            "PERSIST_MARKET_DATA=1 but DATABASE_URL is unset or connection failed; ingest disabled"
        );
    }
    if let Some(binance) = catalog().get(&spot.id.exchange) {
        info!(target: "exchanges", notes = %binance.notes, "Exchange capability catalog");
    }
    if let Some(quote) = config.market.symbol.split('/').nth(1) {
        if let Ok(asset) = Asset::new(quote) {
            let template = crate::portfolio::paper_snapshot(&asset);
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
    let mut dashboard = Dashboard::new(config.clone(), limits);
    let (event_tx, event_rx) = mpsc::channel(64);
    let (command_tx, mut command_rx) = mpsc::channel(8);
    let ui_task = tokio::spawn(ui::run(dashboard.clone(), command_tx.clone(), event_rx));

    let spot_account_id = spot.id.clone();
    let evaluation_settings = EvaluationSettings {
        config: &config,
        limits,
        advisor: &advisor,
        spot_account_id: &spot_account_id,
        event_tx: &event_tx,
    };
    let mut execution_ctx = ExecutionContext::default();
    let mut interval = time::interval(Duration::from_secs(config.market.poll_seconds));
    let mut paused = false;
    let mut persist_timeframe_warned = false;
    let mut hybrid_feed = HybridCandleFeed::new(config.market.candle_limit);
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            cmd = command_rx.recv() => match cmd {
                Some(UiCommand::Quit) | None => break,
                Some(UiCommand::Pause) => { paused = true; info!(target: "system", "Trading evaluation paused by operator"); }
                Some(UiCommand::Resume) => { paused = false; info!(target: "system", "Trading evaluation resumed by operator"); }
            },
            market_wakeup = next_market_wakeup(&mut closed_kline_rx, &mut interval), if !paused => match market_wakeup {
              MarketWakeup::Ws(kline) => {
                if persist_market_data_enabled() {
                    if let Some(db) = database.as_ref() {
                        persist_ws_1m_candle(db, &config.market.symbol, kline.candle).await;
                    }
                }
                if let Some(candle_ts) = hybrid_feed.ingest_ws_closed(kline.candle) {
                    if !run_evaluation_cycle(
                        &mut hybrid_feed,
                        candle_ts,
                        "ws",
                        &evaluation_settings,
                        &mut execution_ctx,
                        &mut dashboard,
                    ).await {
                        break;
                    }
                }
              }
              MarketWakeup::RestPoll => {
                let polled = fetch_authorized_candles(exchange.as_ref(), &spot_account_id, &config.market.symbol, &config.market.timeframe, config.market.candle_limit).await;
                let staged = stage_polled_window(polled, &mut hybrid_feed, |candles| async {
                        if persist_market_data_enabled() {
                            if let Some(db) = database.as_ref() {
                                if config.market.timeframe == "1m" {
                                    persist_polled_1m_window(db, &config.market.symbol, candles).await;
                                } else if !persist_timeframe_warned {
                                    persist_timeframe_warned = true;
                                    warn!(
                                        target: "persistence",
                                        timeframe = %config.market.timeframe,
                                        "PERSIST_MARKET_DATA only supports 1m timeframe"
                                    );
                                }
                            }
                        }
                }).await;
                match staged {
                    Ok(Some(candle_ts)) => {
                        if !run_evaluation_cycle(
                            &mut hybrid_feed,
                            candle_ts,
                            "rest",
                            &evaluation_settings,
                            &mut execution_ctx,
                            &mut dashboard,
                        ).await {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!(target: "market", error=%e, "Market data poll failed");
                        let _ = BotError::AmbiguousOrder(e.to_string());
                        dashboard.set_error(e.to_string());
                        let _ = event_tx.send(AppEvent::Refresh(dashboard.clone())).await;
                    }
                }
              }
            },
        }
    }
    drop(command_tx);
    ws_shutdown.cancel();
    if let Some(handle) = ws_stream_task {
        let _ = handle.await;
    }
    if let Err(e) = ui_task.await {
        warn!(target: "ui", error=%e, "TUI task ended with an error");
    }
    info!(target: "system", environment=%config.environment, "Shutdown complete; no live-order action was performed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{RunMode, StrategyConfig},
        exchanges::{ExchangeId, MarketType},
    };
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };

    #[tokio::test]
    async fn evaluation_cycle_publishes_paper_signal_once_per_closed_bar() {
        let config = Config {
            run_mode: RunMode::Paper,
            strategy: StrategyConfig {
                sma_fast: 2,
                sma_slow: 4,
            },
            market: crate::config::MarketConfig {
                timeframe: "1m".into(),
                ..Config::default().market
            },
            ..Config::default()
        };
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let spot_account_id = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            config.environment,
        )
        .unwrap();
        let advisor = None;
        let (event_tx, mut event_rx) = mpsc::channel(2);
        let settings = EvaluationSettings {
            config: &config,
            limits,
            advisor: &advisor,
            spot_account_id: &spot_account_id,
            event_tx: &event_tx,
        };
        let mut feed = HybridCandleFeed::new(10);
        let mut execution_ctx = ExecutionContext::default();
        let mut dashboard = Dashboard::new(config.clone(), limits);
        let candles: Vec<_> = [5.0, 5.0, 5.0, 5.0, 1.0, 10.0]
            .into_iter()
            .enumerate()
            .map(|(i, close)| mantis_ta::types::Candle {
                timestamp: i as i64 * 60_000,
                open: close,
                high: close,
                low: close,
                close,
                volume: 1.0,
            })
            .collect();

        let timestamp = feed.ingest_rest_window(candles.clone()).unwrap();
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "rest",
                &settings,
                &mut execution_ctx,
                &mut dashboard,
            )
            .await
        );
        let Some(AppEvent::Refresh(refresh)) = event_rx.try_recv().ok() else {
            panic!("evaluation must publish a dashboard refresh");
        };
        assert_eq!(refresh.market.as_ref().unwrap().signal, Signal::Buy);
        assert_eq!(
            refresh.market.as_ref().unwrap().candle_timestamp_ms,
            300_000
        );
        assert_eq!(refresh.paper_open_positions, 1);
        assert_eq!(execution_ctx.open_positions, 1);

        assert_eq!(feed.ingest_rest_window(candles), None);
        assert!(event_rx.try_recv().is_err());
    }

    struct CountingMarketSource {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl MarketDataSource for CountingMarketSource {
        async fn candles(
            &self,
            _: &str,
            _: &str,
            _: u32,
        ) -> BotResult<Vec<mantis_ta::types::Candle>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn denied_rest_use_never_calls_market_source() {
        let source = CountingMarketSource {
            calls: AtomicUsize::new(0),
        };
        let futures = ExchangeAccountId::new(
            crate::exchanges::ExchangeId::Binance,
            crate::exchanges::MarketType::Futures,
            "paper-main",
            crate::config::Environment::Dev,
        )
        .unwrap();
        assert!(
            fetch_authorized_candles(&source, &futures, "BTC/USDT", "1m", 10)
                .await
                .is_err()
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 0);

        let spot = ExchangeAccountId {
            market: crate::exchanges::MarketType::Spot,
            ..futures
        };
        assert!(
            fetch_authorized_candles(&source, &spot, "BTC/USDT", "1m", 10)
                .await
                .is_ok()
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }

    struct FailingMarketSource;

    #[async_trait::async_trait]
    impl MarketDataSource for FailingMarketSource {
        async fn candles(
            &self,
            _: &str,
            _: &str,
            _: u32,
        ) -> BotResult<Vec<mantis_ta::types::Candle>> {
            Err(BotError::MarketData("invalid REST candle window".into()))
        }
    }

    #[tokio::test]
    async fn rejected_rest_window_never_reaches_feed_or_persistence() {
        let spot = ExchangeAccountId::new(
            crate::exchanges::ExchangeId::Binance,
            crate::exchanges::MarketType::Spot,
            "paper-main",
            crate::config::Environment::Dev,
        )
        .unwrap();
        let polled =
            fetch_authorized_candles(&FailingMarketSource, &spot, "BTC/USDT", "1m", 10).await;
        let persist_calls = AtomicUsize::new(0);
        let mut feed = HybridCandleFeed::new(10);
        let staged = stage_polled_window(polled, &mut feed, |_| async {
            persist_calls.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert!(staged.is_err());
        assert!(feed.candles().is_empty());
        assert_eq!(persist_calls.load(Ordering::SeqCst), 0);
    }

    struct SequenceMarketSource {
        windows: Mutex<VecDeque<BotResult<Vec<mantis_ta::types::Candle>>>>,
    }

    #[async_trait::async_trait]
    impl MarketDataSource for SequenceMarketSource {
        async fn candles(
            &self,
            _: &str,
            _: &str,
            _: u32,
        ) -> BotResult<Vec<mantis_ta::types::Candle>> {
            self.windows.lock().unwrap().pop_front().unwrap()
        }
    }

    #[tokio::test]
    async fn rest_failure_then_ws_warmup_then_rest_backfill_evaluates_same_bar_once() {
        let config = Config {
            run_mode: RunMode::Paper,
            strategy: StrategyConfig {
                sma_fast: 2,
                sma_slow: 3,
            },
            market: crate::config::MarketConfig {
                timeframe: "1m".into(),
                ..Config::default().market
            },
            ..Config::default()
        };
        let bar = |timestamp: i64, close: f64| mantis_ta::types::Candle {
            timestamp,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        };
        let full = vec![
            bar(0, 5.0),
            bar(60_000, 5.0),
            bar(120_000, 1.0),
            bar(180_000, 10.0),
        ];
        let gap = vec![full[0], full[1], full[3]];
        let source = SequenceMarketSource {
            windows: Mutex::new(VecDeque::from(vec![
                Err(BotError::MarketData("REST unavailable".into())),
                Ok(gap),
                Ok(full.clone()),
            ])),
        };
        let spot = ExchangeAccountId::new(
            crate::exchanges::ExchangeId::Binance,
            crate::exchanges::MarketType::Spot,
            "paper-main",
            crate::config::Environment::Dev,
        )
        .unwrap();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let advisor = None;
        let (event_tx, mut event_rx) = mpsc::channel(4);
        let settings = EvaluationSettings {
            config: &config,
            limits,
            advisor: &advisor,
            spot_account_id: &spot,
            event_tx: &event_tx,
        };
        let mut feed = HybridCandleFeed::new(10);
        let mut dashboard = Dashboard::new(config.clone(), limits);
        let mut execution_ctx = ExecutionContext::default();
        let persist_calls = AtomicUsize::new(0);

        let first = fetch_authorized_candles(&source, &spot, "BTC/USDT", "1m", 10).await;
        assert!(stage_polled_window(first, &mut feed, |_| async {
            persist_calls.fetch_add(1, Ordering::SeqCst);
        })
        .await
        .is_err());
        let ws = Candle {
            timestamp_ms: 180_000,
            open: 10.0,
            high: 10.0,
            low: 10.0,
            close: 10.0,
            volume: 1.0,
        };
        let timestamp = feed.ingest_ws_closed(ws).unwrap();
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "ws",
                &settings,
                &mut execution_ctx,
                &mut dashboard
            )
            .await
        );
        let Some(AppEvent::Refresh(warmup)) = event_rx.try_recv().ok() else {
            panic!("WS warmup refresh missing")
        };
        assert_eq!(warmup.market.unwrap().signal, Signal::Warmup);
        assert_eq!(execution_ctx.open_positions, 0);

        let gap_poll = fetch_authorized_candles(&source, &spot, "BTC/USDT", "1m", 10).await;
        let timestamp = stage_polled_window(gap_poll, &mut feed, |_| async {
            persist_calls.fetch_add(1, Ordering::SeqCst);
        })
        .await
        .unwrap()
        .expect("gap window must retry same timestamp");
        assert_eq!(timestamp, 180_000);
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "rest",
                &settings,
                &mut execution_ctx,
                &mut dashboard
            )
            .await
        );
        let Some(AppEvent::Refresh(gap_warmup)) = event_rx.try_recv().ok() else {
            panic!("gap warmup refresh missing")
        };
        assert_eq!(gap_warmup.market.unwrap().signal, Signal::Warmup);

        let backfill = fetch_authorized_candles(&source, &spot, "BTC/USDT", "1m", 10).await;
        let timestamp = stage_polled_window(backfill, &mut feed, |_| async {
            persist_calls.fetch_add(1, Ordering::SeqCst);
        })
        .await
        .unwrap()
        .expect("backfill must retry same timestamp");
        assert_eq!(timestamp, 180_000);
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "rest",
                &settings,
                &mut execution_ctx,
                &mut dashboard
            )
            .await
        );
        let Some(AppEvent::Refresh(evaluated)) = event_rx.try_recv().ok() else {
            panic!("backfill refresh missing")
        };
        assert_eq!(evaluated.market.unwrap().signal, Signal::Buy);
        assert_eq!(execution_ctx.open_positions, 1);
        assert_eq!(feed.ingest_rest_window(full), None);
        assert!(event_rx.try_recv().is_err());
        assert_eq!(persist_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn late_ws_gap_fill_marks_latest_evaluated_bar_once() {
        let config = Config {
            run_mode: RunMode::Paper,
            strategy: StrategyConfig {
                sma_fast: 2,
                sma_slow: 3,
            },
            market: crate::config::MarketConfig {
                timeframe: "1m".into(),
                ..Config::default().market
            },
            ..Config::default()
        };
        let spot = ExchangeAccountId::new(
            crate::exchanges::ExchangeId::Binance,
            crate::exchanges::MarketType::Spot,
            "paper-main",
            crate::config::Environment::Dev,
        )
        .unwrap();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let advisor = None;
        let (event_tx, mut event_rx) = mpsc::channel(3);
        let settings = EvaluationSettings {
            config: &config,
            limits,
            advisor: &advisor,
            spot_account_id: &spot,
            event_tx: &event_tx,
        };
        let mut feed = HybridCandleFeed::new(10);
        let mut dashboard = Dashboard::new(config.clone(), limits);
        let mut execution_ctx = ExecutionContext::default();
        let bar = |timestamp: i64, close: f64| mantis_ta::types::Candle {
            timestamp,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        };
        let gap = vec![bar(120_000, 5.0), bar(180_000, 5.0), bar(300_000, 10.0)];
        let timestamp = feed.ingest_rest_window(gap).unwrap();
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "rest",
                &settings,
                &mut execution_ctx,
                &mut dashboard
            )
            .await
        );
        let Some(AppEvent::Refresh(warmup)) = event_rx.try_recv().ok() else {
            panic!("warmup refresh missing")
        };
        assert_eq!(warmup.market.unwrap().signal, Signal::Warmup);

        let late = Candle {
            timestamp_ms: 240_000,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        };
        let timestamp = feed.ingest_ws_closed(late).unwrap();
        assert_eq!(timestamp, 300_000);
        assert!(
            run_evaluation_cycle(
                &mut feed,
                timestamp,
                "ws",
                &settings,
                &mut execution_ctx,
                &mut dashboard
            )
            .await
        );
        let Some(AppEvent::Refresh(evaluated)) = event_rx.try_recv().ok() else {
            panic!("evaluation refresh missing")
        };
        let market = evaluated.market.unwrap();
        assert_eq!(market.signal, Signal::Buy);
        assert_eq!(market.candle_timestamp_ms, 300_000);
        assert_eq!(execution_ctx.open_positions, 1);
        assert_eq!(
            feed.ingest_rest_window(vec![
                bar(120_000, 5.0),
                bar(180_000, 5.0),
                bar(240_000, 1.0),
                bar(300_000, 10.0)
            ]),
            None
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn closed_or_absent_ws_channel_keeps_rest_poll_and_dashboard_refresh() {
        for closed_channel in [true, false] {
            let mut rx = if closed_channel {
                let (tx, rx) = mpsc::channel::<ClosedKline>(1);
                drop(tx);
                Some(rx)
            } else {
                None
            };
            let mut interval = time::interval(Duration::from_millis(10));
            assert!(matches!(
                next_market_wakeup(&mut rx, &mut interval).await,
                MarketWakeup::RestPoll
            ));
            assert!(rx.is_none());

            let config = Config {
                run_mode: RunMode::Paper,
                strategy: StrategyConfig {
                    sma_fast: 2,
                    sma_slow: 4,
                },
                market: crate::config::MarketConfig {
                    timeframe: "1m".into(),
                    ..Config::default().market
                },
                ..Config::default()
            };
            let spot = ExchangeAccountId::new(
                crate::exchanges::ExchangeId::Binance,
                crate::exchanges::MarketType::Spot,
                "paper-main",
                crate::config::Environment::Dev,
            )
            .unwrap();
            let bars = [5.0, 5.0, 5.0, 5.0, 1.0, 10.0]
                .into_iter()
                .enumerate()
                .map(|(i, close)| mantis_ta::types::Candle {
                    timestamp: i as i64 * 60_000,
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: 1.0,
                })
                .collect();
            let source = SequenceMarketSource {
                windows: Mutex::new(VecDeque::from(vec![Ok(bars)])),
            };
            let polled = fetch_authorized_candles(&source, &spot, "BTC/USDT", "1m", 10).await;
            let mut feed = HybridCandleFeed::new(10);
            let timestamp = stage_polled_window(polled, &mut feed, |_| async {})
                .await
                .unwrap()
                .unwrap();
            let limits = RiskLimits {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            };
            let advisor = None;
            let (event_tx, mut event_rx) = mpsc::channel(1);
            let settings = EvaluationSettings {
                config: &config,
                limits,
                advisor: &advisor,
                spot_account_id: &spot,
                event_tx: &event_tx,
            };
            let mut dashboard = Dashboard::new(config.clone(), limits);
            let mut execution_ctx = ExecutionContext::default();
            assert!(
                run_evaluation_cycle(
                    &mut feed,
                    timestamp,
                    "rest",
                    &settings,
                    &mut execution_ctx,
                    &mut dashboard
                )
                .await
            );
            let Some(AppEvent::Refresh(refresh)) = event_rx.try_recv().ok() else {
                panic!("REST refresh missing after WS unavailable")
            };
            assert_eq!(refresh.market.unwrap().signal, Signal::Buy);
        }
    }
}
