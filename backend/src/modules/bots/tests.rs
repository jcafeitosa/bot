use crate::core::config::OperationMode;
use crate::modules::backtest::models::StrategyDefinition;
use crate::modules::bots::{
    build_catalog_from_config, full_ranking, persist_catalog_snapshot, rank_bots, BotCatalogStore,
    BotId, BotIdentity, BotMetrics, BotsError, EvaluationWindow, InMemoryBotCatalogStore, RunId,
    StrategyId, StrategyVersion,
};

fn metric(strategy: &str, timeframe: &str, run: &str, pnl: f64) -> BotMetrics {
    let strategy_id = StrategyId::new(strategy).unwrap();
    let bot_id = BotId::new(&strategy_id, StrategyVersion(1), timeframe, "BTC/USDT").unwrap();
    BotMetrics {
        bot_id,
        timeframe: timeframe.into(),
        symbol: "BTC/USDT".into(),
        strategy_id,
        strategy_version: StrategyVersion(1),
        run_id: RunId(run.into()),
        dataset_hash: "dataset-v1".into(),
        window: EvaluationWindow {
            start_ms: 100,
            end_ms: 200,
        },
        quote_currency: "USDT".into(),
        initial_capital_quote: 1000.0,
        net_pnl_quote: pnl,
        net_return_pct: pnl / 10.0,
        max_drawdown_pct: 5.0,
        trades: 10,
        accuracy_pct: Some(92.0),
    }
}

#[test]
fn identity_is_strategy_version_timeframe_and_symbol() {
    let a = metric("sma-cross", "5m", "r1", 20.0);
    let b = metric("sma-cross", "15m", "r2", 10.0);
    assert_ne!(a.bot_id, b.bot_id);
    let identity = BotIdentity::new(
        StrategyId::new("sma-cross").unwrap(),
        StrategyVersion(1),
        "5m",
        "BTC/USDT",
    )
    .unwrap();
    assert_eq!(identity.bot_id().unwrap(), a.bot_id);
}

#[test]
fn rejects_malformed_market_symbols() {
    let id = StrategyId::new("sma-cross").unwrap();
    for symbol in ["/", "BTC/", "/USDT", "BTC//USDT"] {
        assert!(BotId::new(&id, StrategyVersion(1), "5m", symbol).is_err());
    }
}

#[test]
fn bot_metrics_identity_fields_must_match() {
    let mut row = metric("sma-cross", "5m", "r1", 20.0);
    row.strategy_id = StrategyId::new("other").unwrap();
    assert_eq!(row.validate().unwrap_err(), BotsError::InvalidMetrics);
}

#[test]
fn bot_ranking_orders_by_net_pnl_with_full_rank() {
    let report = full_ranking([
        metric("sma-cross", "5m", "r1", 20.0),
        metric("sma-cross", "15m", "r2", 10.0),
    ])
    .unwrap();
    assert_eq!(report.entries[0].rank, 1);
    assert!(report.entries[0].metrics.net_pnl_quote > report.entries[1].metrics.net_pnl_quote);
    let legacy = rank_bots([
        metric("sma-cross", "5m", "r1", 20.0),
        metric("sma-cross", "15m", "r2", 10.0),
    ])
    .unwrap();
    assert_eq!(legacy.rows.len(), 2);
}

#[test]
fn catalog_lists_strategy_timeframe_combos_for_mode() {
    let strategy = StrategyDefinition {
        id: StrategyId::new("sma-cross").unwrap(),
        version: StrategyVersion(1),
        name: "SMA".into(),
        fast_period: 5,
        slow_period: 20,
    };
    let mut config = crate::core::config::Config {
        operation: OperationMode::Scalper,
        ..Default::default()
    };
    config.market.timeframe = "5m".into();
    config.validate().unwrap();
    let catalog = build_catalog_from_config(&config, &strategy).unwrap();
    assert_eq!(
        catalog.len(),
        OperationMode::Scalper.supported_timeframes().len()
    );
}

#[test]
fn in_memory_catalog_store_round_trip() {
    let strategy = StrategyDefinition {
        id: StrategyId::new("sma-cross").unwrap(),
        version: StrategyVersion(1),
        name: "SMA".into(),
        fast_period: 5,
        slow_period: 20,
    };
    let mut config = crate::core::config::Config {
        operation: OperationMode::DayTrader,
        ..Default::default()
    };
    config.market.timeframe = "15m".into();
    config.validate().unwrap();
    let mut store = InMemoryBotCatalogStore::new();
    let built = persist_catalog_snapshot(&config, &strategy, &mut store).unwrap();
    let loaded = store.load_catalog().unwrap();
    assert_eq!(built.len(), loaded.len());
    assert_eq!(built[0].id, loaded[0].id);
}
