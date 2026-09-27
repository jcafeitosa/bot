use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::modules::application_contracts::Signal;
use crate::modules::backtest::models::{
    BotDefinition, BotMetrics, EvaluationWindow, RunId, StrategyDefinition,
};
use crate::modules::market::{HistoricalDataset, MarketError, Timeframe};
use crate::modules::strategy::evaluate;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExitPolicy {
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub trailing_stop_pct: Option<f64>,
    pub move_stop_to_breakeven_pct: Option<f64>,
}

impl ExitPolicy {
    fn validate(&self) -> Result<(), BacktestError> {
        for value in [
            self.stop_loss_pct,
            self.take_profit_pct,
            self.trailing_stop_pct,
            self.move_stop_to_breakeven_pct,
        ] {
            if value.is_some_and(|v| !v.is_finite() || v <= 0.0 || v >= 100.0) {
                return Err(BacktestError::InvalidConfig);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_capital_quote: f64,
    pub fee_rate: f64,
    pub slippage_rate: f64,
    pub max_position_quote: f64,
    pub exits: ExitPolicy,
}

impl BacktestConfig {
    pub fn validate(&self) -> Result<(), BacktestError> {
        self.exits.validate()?;
        if !self.initial_capital_quote.is_finite()
            || self.initial_capital_quote <= 0.0
            || !self.fee_rate.is_finite()
            || !(0.0..1.0).contains(&self.fee_rate)
            || !self.slippage_rate.is_finite()
            || !(0.0..1.0).contains(&self.slippage_rate)
            || !self.max_position_quote.is_finite()
            || self.max_position_quote <= 0.0
            || self.max_position_quote > self.initial_capital_quote
        {
            return Err(BacktestError::InvalidConfig);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp_ms: i64,
    pub equity_quote: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestReport {
    pub metrics: BotMetrics,
    pub equity_curve: Vec<EquityPoint>,
    pub wins: usize,
    pub losses: usize,
    pub net_profit_quote: f64,
    pub gross_profit_quote: f64,
    pub total_costs_quote: f64,
}

pub fn run_sma_crossover(
    dataset: &HistoricalDataset,
    strategy: &StrategyDefinition,
    timeframe: Timeframe,
    operation: crate::core::config::OperationMode,
    config: BacktestConfig,
    run_id: RunId,
) -> Result<BacktestReport, BacktestError> {
    config.validate()?;
    strategy
        .validate()
        .map_err(|_| BacktestError::InvalidStrategy)?;
    let bot = BotDefinition::new(
        strategy,
        timeframe.as_str(),
        &dataset.manifest.symbol,
        operation,
    )
    .map_err(|_| BacktestError::InvalidBot)?;
    let bars = dataset.resample(timeframe)?;
    let min_history = strategy
        .slow_period
        .checked_add(1)
        .ok_or(BacktestError::InvalidStrategy)?;
    if bars.len() < min_history + 1 {
        return Err(BacktestError::InsufficientHistory);
    }

    let mut cash = config.initial_capital_quote;
    let mut base_position = 0.0;
    let mut entry_cost_basis = 0.0;
    let mut entry_price = 0.0;
    let mut highest_since_entry = 0.0;
    let mut trailing_stop: Option<f64> = None;
    let mut stop_price: Option<f64> = None;
    let mut take_profit_price: Option<f64> = None;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut total_costs = 0.0;
    let mut gross_realized = 0.0;
    let mut equity_curve = Vec::with_capacity(bars.len());

    // A signal is computed at close[i-1]; execution (when possible) occurs at open[i].
    for i in 0..bars.len() {
        if i >= min_history {
            let ta_bars = bars[..i]
                .iter()
                .map(|c| mantis_ta::types::Candle {
                    timestamp: c.timestamp_ms,
                    open: c.open,
                    high: c.high,
                    low: c.low,
                    close: c.close,
                    volume: c.volume,
                })
                .collect::<Vec<_>>();
            let signal = evaluate(&ta_bars, strategy.fast_period, strategy.slow_period).signal;
            let execution_bar = bars[i];
            if signal == Signal::Buy && base_position == 0.0 {
                let spend = config.max_position_quote.min(cash);
                if spend > 0.0 {
                    let fill_price = execution_bar.open * (1.0 + config.slippage_rate);
                    let fee = spend * config.fee_rate;
                    let notional = spend - fee;
                    base_position = notional / fill_price;
                    cash -= spend;
                    entry_cost_basis = spend;
                    entry_price = fill_price;
                    highest_since_entry = fill_price;
                    trailing_stop = None;
                    stop_price = config
                        .exits
                        .stop_loss_pct
                        .map(|pct| fill_price * (1.0 - pct / 100.0));
                    take_profit_price = config
                        .exits
                        .take_profit_pct
                        .map(|pct| fill_price * (1.0 + pct / 100.0));
                    total_costs += fee + (base_position * (fill_price - execution_bar.open));
                }
            } else if signal == Signal::Sell && base_position > 0.0 {
                cash = close_position(
                    cash,
                    &mut base_position,
                    &mut entry_cost_basis,
                    &mut entry_price,
                    &mut highest_since_entry,
                    &mut trailing_stop,
                    &mut stop_price,
                    &mut take_profit_price,
                    execution_bar.open,
                    config.slippage_rate,
                    config,
                    &mut total_costs,
                    &mut gross_realized,
                    &mut wins,
                    &mut losses,
                )?;
            }
        }
        if base_position > 0.0 {
            // Manage open long risk with deterministic stops before the next bar's close.
            let management_bar = bars[i];
            highest_since_entry = highest_since_entry.max(management_bar.high);
            if let Some(trigger) = config.exits.trailing_stop_pct {
                let candidate = management_bar.high * (1.0 - trigger / 100.0);
                trailing_stop = Some(trailing_stop.map_or(candidate, |stop| stop.max(candidate)));
            }
            if let Some(trigger) = config.exits.move_stop_to_breakeven_pct {
                if stop_price.is_some_and(|stop| stop < entry_price)
                    && management_bar.high >= entry_price * (1.0 + trigger / 100.0)
                {
                    stop_price = Some(entry_price);
                }
            }
            let intraday_exit = stop_price.is_some_and(|stop| management_bar.low <= stop)
                || trailing_stop.is_some_and(|stop| management_bar.low <= stop)
                || take_profit_price.is_some_and(|target| management_bar.high >= target);
            if intraday_exit {
                let exit_price = take_profit_price
                    .filter(|target| management_bar.high >= *target)
                    .unwrap_or_else(|| {
                        stop_price
                            .filter(|stop| management_bar.low <= *stop)
                            .or(trailing_stop.filter(|stop| management_bar.low <= *stop))
                            .unwrap_or(management_bar.close)
                    });
                cash = close_position(
                    cash,
                    &mut base_position,
                    &mut entry_cost_basis,
                    &mut entry_price,
                    &mut highest_since_entry,
                    &mut trailing_stop,
                    &mut stop_price,
                    &mut take_profit_price,
                    exit_price,
                    config.slippage_rate,
                    config,
                    &mut total_costs,
                    &mut gross_realized,
                    &mut wins,
                    &mut losses,
                )?;
            }
        }
        let equity = cash + base_position * bars[i].close;
        if !equity.is_finite() {
            return Err(BacktestError::ArithmeticOverflow);
        }
        equity_curve.push(EquityPoint {
            timestamp_ms: bars[i].timestamp_ms,
            equity_quote: equity,
        });
    }
    // Mark open inventory to the final close. Any hypothetical liquidation fee/slippage is charged to net P&L.
    if base_position > 0.0 {
        let final_bar = bars.last().ok_or(BacktestError::InsufficientHistory)?;
        let liquidation_proceeds = base_position * final_bar.close * (1.0 - config.slippage_rate);
        let liquidation_fee = liquidation_proceeds * config.fee_rate;
        cash += liquidation_proceeds - liquidation_fee;
        total_costs += base_position * final_bar.close * config.slippage_rate + liquidation_fee;
    }
    let final_equity = cash;
    let net_pnl = final_equity - config.initial_capital_quote;
    let mut peak = config.initial_capital_quote;
    let max_drawdown_pct = equity_curve
        .iter()
        .map(|point| {
            peak = peak.max(point.equity_quote);
            if peak > 0.0 {
                ((peak - point.equity_quote) / peak * 100.0).clamp(0.0, 100.0)
            } else {
                100.0
            }
        })
        .fold(0.0, f64::max);
    if !net_pnl.is_finite() || !total_costs.is_finite() || !max_drawdown_pct.is_finite() {
        return Err(BacktestError::ArithmeticOverflow);
    }
    let closed_trades = wins + losses;
    let accuracy_pct = (closed_trades > 0).then_some((wins as f64 / closed_trades as f64) * 100.0);
    let window = EvaluationWindow {
        start_ms: bars[0].timestamp_ms,
        end_ms: bars.last().unwrap().timestamp_ms + timeframe.duration_ms(),
    };
    let metrics = BotMetrics {
        bot_id: bot.id,
        timeframe: timeframe.as_str().to_owned(),
        symbol: dataset.manifest.symbol.clone(),
        strategy_id: strategy.id.clone(),
        strategy_version: strategy.version,
        run_id,
        dataset_hash: dataset.manifest.dataset_id.clone(),
        window,
        quote_currency: quote_currency(&dataset.manifest.symbol),
        initial_capital_quote: config.initial_capital_quote,
        net_pnl_quote: net_pnl,
        net_return_pct: net_pnl / config.initial_capital_quote * 100.0,
        max_drawdown_pct,
        trades: closed_trades,
        accuracy_pct,
    };
    metrics
        .validate()
        .map_err(|_| BacktestError::InvalidMetrics)?;
    Ok(BacktestReport {
        metrics,
        equity_curve,
        wins,
        losses,
        net_profit_quote: net_pnl,
        gross_profit_quote: gross_realized,
        total_costs_quote: total_costs,
    })
}

fn quote_currency(symbol: &str) -> String {
    symbol
        .split_once('/')
        .map(|(_, quote)| quote.to_owned())
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn close_position(
    cash: f64,
    base_position: &mut f64,
    entry_cost_basis: &mut f64,
    entry_price: &mut f64,
    highest_since_entry: &mut f64,
    trailing_stop: &mut Option<f64>,
    stop_price: &mut Option<f64>,
    take_profit_price: &mut Option<f64>,
    exit_price_before_slippage: f64,
    slippage: f64,
    config: BacktestConfig,
    total_costs: &mut f64,
    gross_realized: &mut f64,
    wins: &mut usize,
    losses: &mut usize,
) -> Result<f64, BacktestError> {
    let fill_price = exit_price_before_slippage * (1.0 - slippage);
    let proceeds = *base_position * fill_price;
    if ![fill_price, proceeds].iter().all(|v| v.is_finite()) {
        return Err(BacktestError::ArithmeticOverflow);
    }
    let fee = proceeds * config.fee_rate;
    let net_proceeds = proceeds - fee;
    if ![fee, net_proceeds].iter().all(|v| v.is_finite()) {
        return Err(BacktestError::ArithmeticOverflow);
    }
    let realized = net_proceeds - *entry_cost_basis;
    *gross_realized += proceeds - *entry_cost_basis;
    if realized > 0.0 {
        *wins += 1;
    } else {
        *losses += 1;
    }
    *total_costs += fee + *base_position * (exit_price_before_slippage - fill_price).max(0.0);
    *base_position = 0.0;
    *entry_cost_basis = 0.0;
    *entry_price = 0.0;
    *highest_since_entry = 0.0;
    *trailing_stop = None;
    *stop_price = None;
    *take_profit_price = None;
    Ok(cash + net_proceeds)
}

#[derive(Debug, Error)]
pub enum BacktestError {
    #[error(transparent)]
    Market(#[from] MarketError),
    #[error("backtest configuration contains invalid numeric values")]
    InvalidConfig,
    #[error("strategy definition is invalid")]
    InvalidStrategy,
    #[error("strategy is not compatible with bot timeframe or operation")]
    InvalidBot,
    #[error("historical data is insufficient for warm-up and evaluation")]
    InsufficientHistory,
    #[error("backtest arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("generated metrics failed validation")]
    InvalidMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::backtest::models::{StrategyId, StrategyVersion};
    use crate::modules::market::Candle;
    fn candle(timestamp_ms: i64, close: f64) -> Candle {
        Candle {
            timestamp_ms,
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume: 10.0,
        }
    }
    #[test]
    fn backtest_uses_one_minute_base_data_and_emits_valid_bot_metrics() {
        let closes = (0..99)
            .map(|i| {
                if i < 60 {
                    100.0 - i as f64 * 0.2
                } else {
                    88.0 + (i - 60) as f64 * 0.5
                }
            })
            .collect::<Vec<_>>();
        let candles = closes
            .iter()
            .enumerate()
            .map(|(i, close)| candle(i as i64 * 60_000, *close))
            .collect();
        let dataset = HistoricalDataset::from_1m("BTC/USDT", "synthetic", candles).unwrap();
        let strategy = StrategyDefinition {
            id: StrategyId::new("sma-cross").unwrap(),
            version: StrategyVersion(1),
            name: "SMA crossover".into(),
            fast_period: 3,
            slow_period: 8,
        };
        let report = run_sma_crossover(
            &dataset,
            &strategy,
            Timeframe::new(3).unwrap(),
            operation_for_3m(),
            default_config(),
            RunId("test-run".into()),
        )
        .unwrap();
        assert_eq!(report.metrics.timeframe, "3m");
        assert_eq!(report.metrics.symbol, "BTC/USDT");
        assert_eq!(report.equity_curve.len(), 33);
        assert!(report.total_costs_quote > 0.0);
    }
    fn default_config() -> BacktestConfig {
        BacktestConfig {
            initial_capital_quote: 1000.,
            fee_rate: 0.001,
            slippage_rate: 0.001,
            max_position_quote: 100.,
            exits: ExitPolicy {
                stop_loss_pct: None,
                take_profit_pct: None,
                trailing_stop_pct: None,
                move_stop_to_breakeven_pct: None,
            },
        }
    }
    fn operation_for_3m() -> crate::core::config::OperationMode {
        crate::core::config::OperationMode::Scalper
    }

    fn signal_exit_dataset() -> HistoricalDataset {
        let candles = (0..28)
            .map(|i| {
                let mut bar = candle(i * 60_000, if i == 21 { 110.0 } else { 100.0 });
                if i == 22 {
                    bar.open = 101.0;
                    bar.high = 101.0;
                    bar.low = 100.0;
                } else if i == 27 {
                    bar.open = 95.0;
                    bar.low = 95.0;
                }
                bar
            })
            .collect();
        HistoricalDataset::from_1m("BTC/USDT", "signal-cost-test", candles).unwrap()
    }

    fn signal_exit_strategy() -> StrategyDefinition {
        StrategyDefinition {
            id: StrategyId::new("sma-cost-test").unwrap(),
            version: StrategyVersion(1),
            name: "SMA cost test".into(),
            fast_period: 5,
            slow_period: 20,
        }
    }

    fn assert_quote_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "quote amount {actual} differs from expected {expected}"
        );
    }

    #[test]
    fn sell_signal_charges_slippage_at_the_next_open_and_fees_on_effective_proceeds() {
        let dataset = signal_exit_dataset();
        let strategy = signal_exit_strategy();
        let fee_rate = 0.002;
        let spend = 100.0;
        let buy_open = 101.0;
        let sell_open = 95.0;

        for slippage_rate in [0.0, 0.01] {
            let config = BacktestConfig {
                initial_capital_quote: 1_000.0,
                fee_rate,
                slippage_rate,
                max_position_quote: spend,
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
                Timeframe::new(1).unwrap(),
                crate::core::config::OperationMode::Scalper,
                config,
                RunId(format!("sell-slip-{slippage_rate}")),
            )
            .unwrap();

            // The close before Sell is 100; execution is at the next open, 95.
            let buy_fee = spend * fee_rate;
            let units = (spend - buy_fee) / (buy_open * (1.0 + slippage_rate));
            let sell_fill = sell_open * (1.0 - slippage_rate);
            let proceeds = units * sell_fill;
            let sell_fee = proceeds * fee_rate;
            let expected_profit = proceeds - sell_fee - spend;
            let expected_costs = buy_fee
                + units * buy_open * slippage_rate
                + sell_fee
                + units * sell_open * slippage_rate;

            assert_eq!(report.metrics.trades, 1);
            assert_eq!(report.wins, 0);
            assert_eq!(report.losses, 1);
            assert_quote_close(report.gross_profit_quote, proceeds - spend);
            assert_quote_close(report.net_profit_quote, expected_profit);
            assert_quote_close(report.metrics.net_pnl_quote, expected_profit);
            assert_quote_close(report.total_costs_quote, expected_costs);
            assert_quote_close(
                report.equity_curve.last().unwrap().equity_quote,
                config.initial_capital_quote + expected_profit,
            );
        }
    }

    #[test]
    fn intrabar_stop_and_take_profit_keep_their_slippage_and_trigger_bar_timing() {
        let strategy = signal_exit_strategy();
        let fee_rate = 0.002;
        let slippage_rate = 0.01;
        let spend = 100.0;
        let buy_fill = 101.0 * (1.0 + slippage_rate);
        let units = (spend * (1.0 - fee_rate)) / buy_fill;

        for (name, exits, trigger_high, trigger_low, expected_exit_price) in [
            (
                "stop",
                ExitPolicy {
                    stop_loss_pct: Some(3.0),
                    take_profit_pct: None,
                    trailing_stop_pct: None,
                    move_stop_to_breakeven_pct: None,
                },
                100.0,
                95.0,
                buy_fill * 0.97,
            ),
            (
                "take-profit",
                ExitPolicy {
                    stop_loss_pct: None,
                    take_profit_pct: Some(1.0),
                    trailing_stop_pct: None,
                    move_stop_to_breakeven_pct: None,
                },
                105.0,
                99.0,
                buy_fill * 1.01,
            ),
        ] {
            let mut candles = signal_exit_dataset().candles;
            candles[23].high = trigger_high;
            candles[23].low = trigger_low;
            let dataset = HistoricalDataset::from_1m("BTC/USDT", name, candles).unwrap();
            let report = run_sma_crossover(
                &dataset,
                &strategy,
                Timeframe::new(1).unwrap(),
                crate::core::config::OperationMode::Scalper,
                BacktestConfig {
                    initial_capital_quote: 1_000.0,
                    fee_rate,
                    slippage_rate,
                    max_position_quote: spend,
                    exits,
                },
                RunId(name.into()),
            )
            .unwrap();
            let proceeds = units * expected_exit_price * (1.0 - slippage_rate);
            let expected_profit = proceeds * (1.0 - fee_rate) - spend;
            let expected_costs = spend * fee_rate
                + units * 101.0 * slippage_rate
                + proceeds * fee_rate
                + units * expected_exit_price * slippage_rate;

            assert_eq!(report.metrics.trades, 1, "{name}");
            assert_quote_close(report.net_profit_quote, expected_profit);
            assert_quote_close(report.total_costs_quote, expected_costs);
            assert_quote_close(
                report.equity_curve[23].equity_quote,
                1_000.0 + expected_profit,
            );
        }
    }
    #[test]
    fn rejects_insufficient_history() {
        let dataset = HistoricalDataset::from_1m(
            "BTC/USDT",
            "synthetic",
            vec![candle(0, 10.), candle(60_000, 11.)],
        )
        .unwrap();
        let strategy = StrategyDefinition {
            id: StrategyId::new("sma").unwrap(),
            version: StrategyVersion(1),
            name: "SMA".into(),
            fast_period: 1,
            slow_period: 2,
        };
        assert!(matches!(
            run_sma_crossover(
                &dataset,
                &strategy,
                Timeframe::new(1).unwrap(),
                crate::core::config::OperationMode::Scalper,
                BacktestConfig {
                    initial_capital_quote: 100.,
                    fee_rate: 0.,
                    slippage_rate: 0.,
                    max_position_quote: 10.,
                    exits: ExitPolicy {
                        stop_loss_pct: None,
                        take_profit_pct: None,
                        trailing_stop_pct: None,
                        move_stop_to_breakeven_pct: None
                    }
                },
                RunId("r".into())
            ),
            Err(BacktestError::InsufficientHistory)
        ));
    }
    #[test]
    fn exits_use_next_bar_high_low_without_lookahead() {
        let closes = (0..30)
            .map(|i| {
                if i < 10 {
                    100.0
                } else {
                    100.0 + (i - 10) as f64
                }
            })
            .collect::<Vec<_>>();
        let candles: Vec<Candle> = closes
            .iter()
            .enumerate()
            .map(|(i, close)| candle(i as i64 * 60_000, *close))
            .collect();
        let dataset = HistoricalDataset::from_1m("BTC/USDT", "synthetic", candles).unwrap();
        let strategy = StrategyDefinition {
            id: StrategyId::new("trend").unwrap(),
            version: StrategyVersion(1),
            name: "trend".into(),
            fast_period: 1,
            slow_period: 2,
        };
        let no_exits = BacktestConfig {
            exits: ExitPolicy {
                stop_loss_pct: None,
                take_profit_pct: None,
                trailing_stop_pct: None,
                move_stop_to_breakeven_pct: None,
            },
            ..default_config()
        };
        let with_take_profit = BacktestConfig {
            exits: ExitPolicy {
                stop_loss_pct: None,
                take_profit_pct: Some(1.0),
                trailing_stop_pct: None,
                move_stop_to_breakeven_pct: None,
            },
            ..default_config()
        };
        let baseline = run_sma_crossover(
            &dataset,
            &strategy,
            Timeframe::new(1).unwrap(),
            crate::core::config::OperationMode::Scalper,
            no_exits,
            RunId("baseline".into()),
        )
        .unwrap();
        let exits = run_sma_crossover(
            &dataset,
            &strategy,
            Timeframe::new(1).unwrap(),
            crate::core::config::OperationMode::Scalper,
            with_take_profit,
            RunId("take-profit".into()),
        )
        .unwrap();
        assert!(exits.net_profit_quote <= baseline.net_profit_quote + 1e-9);
        assert!(exits.metrics.trades >= baseline.metrics.trades);
    }
}
