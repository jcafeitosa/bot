use crate::core::config::{Config, Environment, RunMode};
use crate::modules::application_contracts::Signal;
use crate::modules::monitor::views::terminal_dashboard::{
    Dashboard, DashboardHeader, MonitorSignalLabel, TerminalMarketRow,
};
use crate::modules::risk::RiskLimits;
use crate::modules::strategy::StrategySnapshot;

pub fn dashboard_header(config: &Config) -> DashboardHeader {
    DashboardHeader {
        environment_label: config.environment.to_string(),
        environment_is_dev: config.environment == Environment::Dev,
        run_mode_label: format!("{:?}", config.run_mode),
        is_paper: config.run_mode == RunMode::Paper,
        symbol: config.market.symbol.clone(),
        operation_label: config.operation.to_string(),
        risk_profile_label: config.risk_profile.to_string(),
        sma_fast: config.strategy.sma_fast,
        sma_slow: config.strategy.sma_slow,
    }
}

pub fn new_dashboard(config: &Config, limits: RiskLimits) -> Dashboard {
    Dashboard::new(dashboard_header(config), limits.max_order_quote)
}

pub fn terminal_market_row(snapshot: &StrategySnapshot) -> TerminalMarketRow {
    TerminalMarketRow {
        close: snapshot.close,
        fast_sma: snapshot.fast_sma,
        slow_sma: snapshot.slow_sma,
        candle_timestamp_ms: snapshot.candle_timestamp_ms,
        signal: match snapshot.signal {
            Signal::Warmup => MonitorSignalLabel::Warmup,
            Signal::Hold => MonitorSignalLabel::Hold,
            Signal::Buy => MonitorSignalLabel::Buy,
            Signal::Sell => MonitorSignalLabel::Sell,
        },
    }
}
