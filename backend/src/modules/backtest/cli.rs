use std::path::PathBuf;

use clap::Parser;
use serde_json::json;

use crate::{
    core::config::{Config, MonitorCli},
    core::error::BotResult,
    core::persistence::{Database, PersistenceError},
    modules::backtest::{rank_bots, rank_strategies, RunId, StrategyDefinition, StrategyVersion},
    modules::backtest::{run_sma_crossover, BacktestConfig, ExitPolicy},
    modules::market::{persist_historical_dataset, Candle, HistoricalDataset, Timeframe},
};

#[derive(Debug, Parser)]
pub struct BacktestCli {
    #[arg(long, default_value = "src/core/config/bot.toml")]
    pub config: PathBuf,
    /// Persist synthetic dataset to PostgreSQL when DATABASE_URL is set.
    #[arg(long)]
    pub persist: bool,
}

pub async fn execute_backtest(cli: &BacktestCli) -> BotResult<serde_json::Value> {
    let monitor = MonitorCli {
        config: cli.config.clone(),
        environment: None,
        operation: None,
        risk_profile: None,
        mode: None,
    };
    let config = Config::load(&monitor)?;
    let timeframe = Timeframe::new(parse_timeframe_minutes(&config.market.timeframe)?)
        .map_err(|e| crate::core::error::BotError::MarketData(e.to_string()))?;
    let strategy = StrategyDefinition {
        id: crate::modules::bots::StrategyId::new("sma-cross")
            .map_err(|e| crate::core::error::BotError::Configuration(e.to_string()))?,
        version: StrategyVersion(1),
        name: "SMA crossover".into(),
        fast_period: config.strategy.sma_fast,
        slow_period: config.strategy.sma_slow,
        evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
    };
    let dataset = synthetic_dataset(
        &config.market.symbol,
        timeframe,
        config.strategy.sma_fast,
        config.strategy.sma_slow,
    )?;
    if cli.persist {
        persist_dataset_if_configured(&dataset).await?;
    }
    let report = run_sma_crossover(
        &dataset,
        &strategy,
        timeframe,
        config.operation,
        BacktestConfig {
            initial_capital_quote: config.risk.max_order_quote * 40.0,
            fee_rate: 0.001,
            slippage_rate: 0.001,
            max_position_quote: config.risk.max_order_quote,
            exits: ExitPolicy {
                stop_loss_pct: None,
                take_profit_pct: None,
                trailing_stop_pct: None,
                move_stop_to_breakeven_pct: None,
            },
        },
        RunId("cli-backtest".into()),
    )
    .map_err(|e| crate::core::error::BotError::Strategy(e.to_string()))?;
    let _strategy_ranks = rank_strategies([report.metrics.clone()])
        .map_err(|e| crate::core::error::BotError::Strategy(format!("strategy ranking: {e}")))?;
    let ranking = rank_bots([report.metrics.clone()])
        .map_err(|e| crate::core::error::BotError::Strategy(format!("ranking: {e}")))?;
    let rank_position = ranking
        .rows
        .iter()
        .position(|row| row.run_id == report.metrics.run_id)
        .map(|idx| idx + 1);
    Ok(json!({
        "bot_id": report.metrics.bot_id.to_string(),
        "bot_id_key": report.metrics.bot_id.as_str(),
        "net_pnl_quote": report.metrics.net_pnl_quote,
        "net_return_pct": report.metrics.net_return_pct,
        "max_drawdown_pct": report.metrics.max_drawdown_pct,
        "trades": report.metrics.trades,
        "wins": report.wins,
        "losses": report.losses,
        "rank": rank_position,
        "timeframe_minutes": timeframe.minutes(),
        "dataset_id": dataset.manifest.dataset_id,
    }))
}

pub async fn run(cli: &BacktestCli) -> BotResult<()> {
    let summary = execute_backtest(cli).await?;
    println!("{summary}");
    Ok(())
}

async fn persist_dataset_if_configured(dataset: &HistoricalDataset) -> BotResult<()> {
    if std::env::var("DATABASE_URL").is_err() {
        return Err(crate::core::error::BotError::Configuration(
            "--persist requires DATABASE_URL (postgresql://…/trading_bot)".into(),
        ));
    }
    let mantis = dataset.candles.iter().map(|c| (*c).to_mantis()).collect();
    let validated = HistoricalDataset::from_mantis_1m(
        &dataset.manifest.symbol,
        &dataset.manifest.source,
        mantis,
    )
    .map_err(|e| crate::core::error::BotError::MarketData(e.to_string()))?;
    let db = Database::connect_from_env()
        .await
        .map_err(|e| crate::core::error::BotError::Configuration(e.to_string()))?;
    db.migrate().await.map_err(|e: PersistenceError| {
        crate::core::error::BotError::Configuration(e.to_string())
    })?;
    persist_historical_dataset(&db, &validated)
        .await
        .map_err(|e: PersistenceError| {
            crate::core::error::BotError::Configuration(e.to_string())
        })?;
    eprintln!(
        "persisted dataset {} ({} candles)",
        validated.manifest.dataset_id, validated.manifest.candle_count
    );
    Ok(())
}

fn parse_timeframe_minutes(tf: &str) -> Result<u32, crate::core::error::BotError> {
    if let Some(m) = tf.strip_suffix('m') {
        return m.parse().map_err(|_| invalid_tf());
    }
    if let Some(h) = tf.strip_suffix('h') {
        return h
            .parse::<u32>()
            .map_err(|_| invalid_tf())?
            .checked_mul(60)
            .ok_or_else(invalid_tf);
    }
    Err(invalid_tf())
}

fn invalid_tf() -> crate::core::error::BotError {
    crate::core::error::BotError::Configuration("unsupported timeframe for backtest".into())
}

const MAX_SYNTHETIC_1M_CANDLES: usize = 20_000;

/// Baseline through warm-up, one impulse, then enough bars to execute Buy and Sell.
fn synthetic_1m_candle_count(
    timeframe: Timeframe,
    fast_period: usize,
    slow_period: usize,
) -> Result<usize, crate::core::error::BotError> {
    let bars = slow_period
        .checked_add(fast_period)
        .and_then(|n| n.checked_add(3))
        .ok_or_else(fixture_too_large)?;
    let width = usize::try_from(timeframe.minutes()).map_err(|_| fixture_too_large())?;
    let count = bars.checked_mul(width).ok_or_else(fixture_too_large)?;
    let count_i64 = i64::try_from(count).map_err(|_| fixture_too_large())?;
    if count > MAX_SYNTHETIC_1M_CANDLES || count_i64.checked_mul(60_000).is_none() {
        return Err(fixture_too_large());
    }
    Ok(count)
}

fn fixture_too_large() -> crate::core::error::BotError {
    crate::core::error::BotError::Configuration(
        "synthetic backtest fixture exceeds 20,000 1m candles or timestamp range".into(),
    )
}

fn synthetic_dataset(
    symbol: &str,
    timeframe: Timeframe,
    fast_period: usize,
    slow_period: usize,
) -> Result<HistoricalDataset, crate::core::error::BotError> {
    let count = synthetic_1m_candle_count(timeframe, fast_period, slow_period)?;
    let width = usize::try_from(timeframe.minutes()).map_err(|_| fixture_too_large())?;
    let impulse_bar = slow_period.checked_add(1).ok_or_else(fixture_too_large)?;
    let mut candles = Vec::new();
    candles
        .try_reserve_exact(count)
        .map_err(|_| fixture_too_large())?;
    for i in 0..count {
        let price = if i / width == impulse_bar {
            110.0
        } else {
            100.0
        };
        let timestamp_ms = i64::try_from(i)
            .ok()
            .and_then(|minute| minute.checked_mul(60_000))
            .ok_or_else(fixture_too_large)?;
        candles.push(Candle {
            timestamp_ms,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 10.0,
        });
    }
    HistoricalDataset::from_1m(symbol, "synthetic-cli", candles)
        .map_err(|e| crate::core::error::BotError::MarketData(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::config::OperationMode, modules::application_contracts::Signal,
        modules::strategy::evaluate,
    };

    #[test]
    fn fixture_closes_one_sell_at_the_next_bar_open() {
        let timeframe = Timeframe::new(3).unwrap();
        let dataset = synthetic_dataset("BTC/USDT", timeframe, 5, 20).unwrap();
        let bars = dataset.resample(timeframe).unwrap();
        let signals: Vec<_> = (21..bars.len())
            .map(|i| {
                let candles = bars[..i]
                    .iter()
                    .copied()
                    .map(Candle::to_mantis)
                    .collect::<Vec<_>>();
                (i, evaluate(&candles, 5, 20).signal)
            })
            .filter(|(_, signal)| matches!(signal, Signal::Buy | Signal::Sell))
            .collect();
        assert_eq!(signals, [(22, Signal::Buy), (27, Signal::Sell)]);
        assert_eq!(bars[22].open, 100.0);
        assert_eq!(bars[27].open, 100.0);

        let strategy = StrategyDefinition {
            id: crate::modules::bots::StrategyId::new("sma-cross").unwrap(),
            version: StrategyVersion(1),
            name: "SMA crossover".into(),
            fast_period: 5,
            slow_period: 20,
            evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
        };
        let config = BacktestConfig {
            initial_capital_quote: 1_000.0,
            fee_rate: 0.0,
            slippage_rate: 0.0,
            max_position_quote: 100.0,
            exits: ExitPolicy {
                stop_loss_pct: None,
                take_profit_pct: None,
                trailing_stop_pct: None,
                move_stop_to_breakeven_pct: None,
            },
        };
        let report = run_sma_crossover(
            &dataset,
            &strategy,
            timeframe,
            OperationMode::Scalper,
            config,
            RunId("fixture-test".into()),
        )
        .unwrap();
        assert_eq!(report.metrics.trades, 1);
        assert_eq!(report.wins + report.losses, 1);
        assert_eq!(report.net_profit_quote, 0.0);

        let mut changed_exit_open = dataset.candles.clone();
        for candle in changed_exit_open.iter_mut().rev().take(3) {
            candle.open = 95.0;
            candle.high = 95.0;
            candle.low = 95.0;
            candle.close = 95.0;
        }
        let changed_exit_open =
            HistoricalDataset::from_1m("BTC/USDT", "synthetic-cli", changed_exit_open).unwrap();
        let altered_bars = changed_exit_open.resample(timeframe).unwrap();
        assert_eq!(altered_bars[26].close, 100.0);
        assert_eq!(altered_bars[27].open, 95.0);
        let altered_report = run_sma_crossover(
            &changed_exit_open,
            &strategy,
            timeframe,
            OperationMode::Scalper,
            config,
            RunId("altered-exit-open-test".into()),
        )
        .unwrap();
        assert_eq!(altered_report.metrics.trades, 1);
        assert_eq!(altered_report.net_profit_quote, -5.0);

        let shorter = HistoricalDataset::from_1m(
            "BTC/USDT",
            "synthetic-cli",
            dataset.candles[..dataset.candles.len() - 3].to_vec(),
        )
        .unwrap();
        let shorter_report = run_sma_crossover(
            &shorter,
            &strategy,
            timeframe,
            OperationMode::Scalper,
            config,
            RunId("short-fixture-test".into()),
        )
        .unwrap();
        assert_eq!(shorter_report.metrics.trades, 0);
    }

    #[test]
    fn fixture_size_is_checked_and_capped_before_allocation() {
        let one_minute = Timeframe::new(1).unwrap();
        let four_hours = Timeframe::new(240).unwrap();
        assert_eq!(
            synthetic_1m_candle_count(four_hours, 20, 50).unwrap(),
            17_520
        );
        assert_eq!(
            synthetic_1m_candle_count(one_minute, 1, 19_996).unwrap(),
            20_000
        );
        assert!(matches!(
            synthetic_1m_candle_count(one_minute, 1, 19_997),
            Err(crate::core::error::BotError::Configuration(_))
        ));
        assert!(matches!(
            synthetic_1m_candle_count(four_hours, usize::MAX, 50),
            Err(crate::core::error::BotError::Configuration(_))
        ));
    }
}
