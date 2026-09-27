use crate::core::config::{Config, Environment, RunMode};
use crate::modules::application_contracts::Signal;
use crate::modules::monitor::views::presentation_contract::{
    normalize_informative_field, normalize_log_messages, Decimal, DecimalError,
    Environment as ContractEnvironment, MonitorMarketSnapshot, MonitorRunState, MonitorSignal,
    MonitorSnapshot, PersistenceStatus, RunMode as ContractRunMode,
};
use crate::modules::monitor::views::terminal_dashboard::{
    Dashboard, DashboardHeader, MonitorSignalLabel, MonitorState, TerminalMarketRow,
};
use crate::modules::risk::RiskLimits;
use crate::modules::strategy::StrategySnapshot;

const PRICE_DECIMAL_SCALE: u32 = 8;

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

fn persistence_status_from_label(label: &str) -> PersistenceStatus {
    if label.starts_with("HEALTHY") {
        PersistenceStatus::Healthy
    } else if label.starts_with("GAP") {
        PersistenceStatus::Gap
    } else if label.starts_with("DEGRADED") {
        PersistenceStatus::Degraded
    } else {
        PersistenceStatus::Unavailable
    }
}

fn decimal_from_f64(value: f64) -> Result<Decimal, DecimalError> {
    Decimal::from_finite_f64(value, PRICE_DECIMAL_SCALE)
}

fn market_snapshot_from_row(
    row: &TerminalMarketRow,
) -> Result<MonitorMarketSnapshot, DecimalError> {
    Ok(MonitorMarketSnapshot {
        close: decimal_from_f64(row.close)?,
        sma_fast: row.fast_sma.map(decimal_from_f64).transpose()?,
        sma_slow: row.slow_sma.map(decimal_from_f64).transpose()?,
        signal: Some(match row.signal {
            MonitorSignalLabel::Warmup => MonitorSignal::Warmup,
            MonitorSignalLabel::Hold => MonitorSignal::Hold,
            MonitorSignalLabel::Buy => MonitorSignal::Buy,
            MonitorSignalLabel::Sell => MonitorSignal::Sell,
        }),
    })
}

/// Maps the TUI dashboard model to the HTTP-facing `MonitorSnapshot` contract.
pub fn monitor_snapshot_from_dashboard(
    dashboard: &Dashboard,
    revision: u64,
) -> Result<MonitorSnapshot, DecimalError> {
    let market = dashboard
        .market
        .as_ref()
        .map(market_snapshot_from_row)
        .transpose()?;
    Ok(MonitorSnapshot {
        revision,
        environment: if dashboard.header.environment_is_dev {
            ContractEnvironment::Dev
        } else {
            ContractEnvironment::Prod
        },
        run_mode: if dashboard.header.is_paper {
            ContractRunMode::Paper
        } else {
            ContractRunMode::Observe
        },
        symbol: dashboard.header.symbol.clone(),
        operation_label: dashboard.header.operation_label.clone(),
        risk_profile_label: dashboard.header.risk_profile_label.clone(),
        run_state: match dashboard.monitor_state {
            MonitorState::Running => MonitorRunState::Running,
            MonitorState::Paused => MonitorRunState::Paused,
            MonitorState::Resuming => MonitorRunState::Resuming,
        },
        sma_fast_period: dashboard.header.sma_fast as u32,
        sma_slow_period: dashboard.header.sma_slow as u32,
        max_order_quote: decimal_from_f64(dashboard.max_order_quote)?,
        persistence_status: persistence_status_from_label(&dashboard.persistence_status),
        market,
        paper_positions: Some(dashboard.paper_open_positions as u64),
        advisory: normalize_informative_field(
            dashboard.advisor_note.clone(),
            crate::modules::monitor::views::presentation_contract::ERROR_ADVISORY_MAX_BYTES,
        ),
        last_error: normalize_informative_field(
            dashboard.last_error.clone(),
            crate::modules::monitor::views::presentation_contract::ERROR_ADVISORY_MAX_BYTES,
        ),
        logs: normalize_log_messages(dashboard.logs.clone()),
        bot_runtime_enabled: false,
        promoted_bot_id: None,
        promoted_by: None,
    })
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn monitor_snapshot_from_dashboard_maps_header_and_revision() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let dashboard = new_dashboard(&config, limits);
        let mut snapshot = monitor_snapshot_from_dashboard(&dashboard, 3).expect("snapshot");
        crate::modules::bots::enrich_monitor_snapshot_from_shared_runtime(&mut snapshot);
        assert_eq!(snapshot.revision, 3);
        assert_eq!(snapshot.symbol, config.market.symbol);
        assert_eq!(snapshot.run_state, MonitorRunState::Running);
        assert!(!snapshot.bot_runtime_enabled);
        assert!(snapshot.promoted_bot_id.is_none());
    }
    #[test]
    fn monitor_snapshot_maps_gap_persistence_label() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let mut dashboard = new_dashboard(&config, limits);
        dashboard.persistence_status = "GAP · reconciliação externa necessária".into();
        let snapshot = monitor_snapshot_from_dashboard(&dashboard, 1).expect("snapshot");
        assert_eq!(snapshot.persistence_status, PersistenceStatus::Gap);
    }

    #[test]
    fn monitor_snapshot_maps_initial_degraded_persistence_label() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let mut dashboard = new_dashboard(&config, limits);
        dashboard.persistence_status = "DEGRADED · aguardando janela REST inicial".into();
        let snapshot = monitor_snapshot_from_dashboard(&dashboard, 2).expect("snapshot");
        assert_eq!(snapshot.persistence_status, PersistenceStatus::Degraded);
    }

    #[test]
    fn monitor_snapshot_maps_unknown_persistence_label_to_unavailable() {
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let mut dashboard = new_dashboard(&config, limits);
        dashboard.persistence_status = "OFF".into();
        let snapshot = monitor_snapshot_from_dashboard(&dashboard, 3).expect("snapshot");
        assert_eq!(snapshot.persistence_status, PersistenceStatus::Unavailable);
    }

    fn monitor_snapshot_from_dashboard_then_runtime_apply_enriches_promotion() {
        use crate::modules::bots::{
            apply_bot_runtime_to_monitor_snapshot, BotRuntimePort, InMemoryBotRuntime,
            PromoteBotRequest,
        };
        let config = Config::default();
        let limits = RiskLimits {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        };
        let dashboard = new_dashboard(&config, limits);
        let mut snapshot = monitor_snapshot_from_dashboard(&dashboard, 5).expect("snapshot");
        assert!(!snapshot.bot_runtime_enabled);

        let runtime = InMemoryBotRuntime::new();
        runtime
            .promote(PromoteBotRequest {
                bot_id: "sma-cross@1:5m:BTC/USDT".into(),
                promoted_by: "owner-1".into(),
            })
            .expect("promote");
        apply_bot_runtime_to_monitor_snapshot(&mut snapshot, &runtime);
        assert!(snapshot.bot_runtime_enabled);
        assert_eq!(
            snapshot.promoted_bot_id.as_deref(),
            Some("sma-cross@1:5m:BTC/USDT")
        );
    }
}
