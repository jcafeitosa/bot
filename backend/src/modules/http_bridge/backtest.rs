use std::path::PathBuf;

use serde::Deserialize;
use utoipa::ToSchema;

use crate::core::config::{MonitorCli, SystemConfig};
use crate::core::error::BotError;
use crate::modules::backtest::cli::{execute_backtest, BacktestCli};

#[derive(Debug, Deserialize, ToSchema)]
pub struct BacktestRequest {
    #[schema(example = "src/core/config/bot.toml")]
    pub config: String,
    #[serde(default)]
    pub persist: bool,
}

pub async fn run_sma_crossover(body: BacktestRequest) -> Result<serde_json::Value, BotError> {
    let config_path = PathBuf::from(body.config);
    let cli = BacktestCli {
        config: Some(config_path.clone()),
        persist: body.persist,
    };
    let global = MonitorCli {
        config: config_path,
        environment: None,
        operation: None,
        risk_profile: None,
        mode: None,
        system_config: SystemConfig::default_path(),
    };
    execute_backtest(&cli, &global).await
}
