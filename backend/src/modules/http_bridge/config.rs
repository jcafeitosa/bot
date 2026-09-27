use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::config::{Config, MonitorCli};
use crate::core::error::BotError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigSnapshotQuery {
    #[schema(example = "src/core/config/bot.toml")]
    pub config: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigSnapshotResponse {
    pub environment: String,
    pub operation: String,
    pub risk_profile: String,
    pub run_mode: String,
    pub symbol: String,
    pub timeframe: String,
    pub sma_fast: usize,
    pub sma_slow: usize,
    pub jev_enabled: bool,
    pub production_enabled: bool,
}

pub fn load_config_snapshot(
    query: ConfigSnapshotQuery,
) -> Result<ConfigSnapshotResponse, BotError> {
    let cli = MonitorCli {
        config: PathBuf::from(query.config),
        environment: None,
        operation: None,
        risk_profile: None,
        mode: None,
    };
    let config = Config::load(&cli)?;
    Ok(map_config(&config))
}

pub fn map_config(config: &Config) -> ConfigSnapshotResponse {
    ConfigSnapshotResponse {
        environment: format!("{:?}", config.environment),
        operation: format!("{:?}", config.operation),
        risk_profile: format!("{:?}", config.risk_profile),
        run_mode: format!("{:?}", config.run_mode),
        symbol: config.market.symbol.clone(),
        timeframe: config.market.timeframe.clone(),
        sma_fast: config.strategy.sma_fast,
        sma_slow: config.strategy.sma_slow,
        jev_enabled: config.jev.enabled,
        production_enabled: config.production.enabled,
    }
}
