use serde::Deserialize;
use utoipa::ToSchema;

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
    let cli = BacktestCli {
        config: body.config.into(),
        persist: body.persist,
    };
    execute_backtest(&cli).await
}
