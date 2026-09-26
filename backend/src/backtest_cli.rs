use std::path::PathBuf;

use clap::Parser;
use serde_json::json;

use crate::{
    backtest::{run_sma_crossover, BacktestConfig, ExitPolicy},
    config::{Config, MonitorCli},
    domain::{rank_bots, rank_strategies, RunId, StrategyDefinition, StrategyVersion},
    error::BotResult,
    market::{Candle, HistoricalDataset, Timeframe},
    persistence::{Database, PersistenceError},
};

#[derive(Debug, Parser)]
pub struct BacktestCli {
    #[arg(long, default_value = "src/config/bot.toml")]
    pub config: PathBuf,
    /// Persist synthetic dataset to PostgreSQL when DATABASE_URL is set.
    #[arg(long)]
    pub persist: bool,
}

pub async fn run(cli: &BacktestCli) -> BotResult<()> {
    let monitor = MonitorCli {
        config: cli.config.clone(),
        environment: None,
        operation: None,
        risk_profile: None,
        mode: None,
    };
    let config = Config::load(&monitor)?;
    let timeframe = Timeframe::new(parse_timeframe_minutes(&config.market.timeframe)?)
        .map_err(|e| crate::error::BotError::MarketData(e.to_string()))?;
    let strategy = StrategyDefinition {
        id: crate::domain::StrategyId::new("sma-cross")
            .map_err(|e| crate::error::BotError::Configuration(e.to_string()))?,
        version: StrategyVersion(1),
        name: "SMA crossover".into(),
        fast_period: config.strategy.sma_fast,
        slow_period: config.strategy.sma_slow,
    };
    let slow_period = u32::try_from(config.strategy.sma_slow).map_err(|_| {
        crate::error::BotError::Configuration("strategy.sma_slow is too large".into())
    })?;
    let dataset = synthetic_dataset(&config.market.symbol, timeframe, slow_period)?;
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
    .map_err(|e| crate::error::BotError::Strategy(e.to_string()))?;
    let _strategy_ranks = rank_strategies([report.metrics.clone()])
        .map_err(|e| crate::error::BotError::Strategy(format!("strategy ranking: {e}")))?;
    let ranking = rank_bots([report.metrics.clone()])
        .map_err(|e| crate::error::BotError::Strategy(format!("ranking: {e}")))?;
    let rank_position = ranking
        .rows
        .iter()
        .position(|row| row.run_id == report.metrics.run_id)
        .map(|idx| idx + 1);
    println!(
        "{}",
        json!({
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
        })
    );
    Ok(())
}

async fn persist_dataset_if_configured(dataset: &HistoricalDataset) -> BotResult<()> {
    if std::env::var("DATABASE_URL").is_err() {
        return Err(crate::error::BotError::Configuration(
            "--persist requires DATABASE_URL (postgresql://…/trading_bot)".into(),
        ));
    }
    let mantis = dataset.candles.iter().map(|c| (*c).to_mantis()).collect();
    let validated = HistoricalDataset::from_mantis_1m(
        &dataset.manifest.symbol,
        &dataset.manifest.source,
        mantis,
    )
    .map_err(|e| crate::error::BotError::MarketData(e.to_string()))?;
    let db = Database::connect_from_env()
        .await
        .map_err(|e| crate::error::BotError::Configuration(e.to_string()))?;
    db.migrate()
        .await
        .map_err(|e: PersistenceError| crate::error::BotError::Configuration(e.to_string()))?;
    db.persist_dataset(&validated)
        .await
        .map_err(|e: PersistenceError| crate::error::BotError::Configuration(e.to_string()))?;
    eprintln!(
        "persisted dataset {} ({} candles)",
        validated.manifest.dataset_id, validated.manifest.candle_count
    );
    Ok(())
}

fn parse_timeframe_minutes(tf: &str) -> Result<u32, crate::error::BotError> {
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

fn invalid_tf() -> crate::error::BotError {
    crate::error::BotError::Configuration("unsupported timeframe for backtest".into())
}

/// Enough aligned 1m candles so `resample(timeframe)` yields `slow_period + 2` bars (warm-up + execution).
fn synthetic_1m_candle_count(timeframe: Timeframe, slow_period: u32) -> usize {
    let min_resampled = slow_period.saturating_add(2) as usize;
    min_resampled * timeframe.minutes() as usize
}

fn synthetic_dataset(
    symbol: &str,
    timeframe: Timeframe,
    slow_period: u32,
) -> Result<HistoricalDataset, crate::error::BotError> {
    let count = synthetic_1m_candle_count(timeframe, slow_period);
    let closes: Vec<f64> = (0..count)
        .map(|i| {
            if i < 70 {
                100.0 - i as f64 * 0.15
            } else {
                89.5 + (i - 70) as f64 * 0.35
            }
        })
        .collect();
    let candles = closes
        .iter()
        .enumerate()
        .map(|(i, close)| Candle {
            timestamp_ms: i as i64 * 60_000,
            open: *close,
            high: *close * 1.01,
            low: *close * 0.99,
            close: *close,
            volume: 10.0,
        })
        .collect();
    HistoricalDataset::from_1m(symbol, "synthetic-cli", candles)
        .map_err(|e| crate::error::BotError::MarketData(e.to_string()))
}
