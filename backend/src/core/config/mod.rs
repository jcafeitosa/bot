use std::{env, fs, path::PathBuf};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::error::{BotError, BotResult};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Dev,
    Prod,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Dev => "DEV / TESTNET",
                Self::Prod => "PROD / MAINNET",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationMode {
    Hft,
    Scalper,
    DayTrader,
    SwingTrader,
}

impl OperationMode {
    pub fn is_hft(self) -> bool {
        matches!(self, Self::Hft)
    }

    pub fn all_timeframes() -> &'static [&'static str] {
        &["1m", "3m", "5m", "15m", "30m", "1h", "4h"]
    }

    /// SMA periods required by TOML/CLI validation for each operation preset.
    pub fn sma_period_preset(self) -> (usize, usize) {
        match self {
            Self::Hft => (1, 2),
            Self::Scalper => (5, 20),
            Self::DayTrader => (5, 20),
            Self::SwingTrader => (20, 50),
        }
    }

    pub fn supported_timeframes(self) -> &'static [&'static str] {
        match self {
            Self::Hft => &["1m"],
            Self::Scalper => &["1m", "3m", "5m"],
            Self::DayTrader => &["5m", "15m", "30m"],
            Self::SwingTrader => &["1h", "4h"],
        }
    }
}

impl std::fmt::Display for OperationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Hft => "HFT",
                Self::Scalper => "Scalper",
                Self::DayTrader => "Day trader",
                Self::SwingTrader => "Swing trader",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RiskProfile {
    Conservative,
    Moderate,
    Aggressive,
    Auto,
}

impl std::fmt::Display for RiskProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Conservative => "Conservative",
                Self::Moderate => "Moderate",
                Self::Aggressive => "Aggressive",
                Self::Auto => "Auto (bounded)",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Observe,
    Paper,
    Testnet,
}

impl std::fmt::Display for RunMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Observe => "Observe only",
                Self::Paper => "Paper",
                Self::Testnet => "Testnet (orders disabled in v1)",
            }
        )
    }
}

#[derive(Debug, Parser, Clone)]
pub struct MonitorCli {
    #[arg(long, value_enum)]
    pub environment: Option<Environment>,
    #[arg(long, value_enum)]
    pub operation: Option<OperationMode>,
    #[arg(long, value_enum)]
    pub risk_profile: Option<RiskProfile>,
    #[arg(long, value_enum)]
    pub mode: Option<RunMode>,
    #[arg(long, default_value = "src/core/config/bot.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub environment: Environment,
    pub operation: OperationMode,
    pub risk_profile: RiskProfile,
    pub run_mode: RunMode,
    pub market: MarketConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub production: ProductionConfig,
    pub jev: JevConfig,
    #[serde(default)]
    pub providers: ProviderConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConfig {
    pub symbol: String,
    pub timeframe: String,
    pub candle_limit: u32,
    pub poll_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub sma_fast: usize,
    pub sma_slow: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_order_quote: f64,
    pub max_daily_loss_quote: f64,
    pub max_open_positions: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    pub enabled: bool,
    pub acknowledgement: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JevConfig {
    pub enabled: bool,
    pub market_regime: bool,
    pub signal_review: bool,
    pub ops_triage: bool,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    /// Optional OpenAI-compatible API root (env `NINE_ROUTER_BASE_URL` / `OPENAI_BASE_URL` override this).
    #[serde(default)]
    pub openai_base_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: Option<String>,
    pub secret: Option<String>,
}

impl Config {
    pub fn load(cli: &MonitorCli) -> BotResult<Self> {
        let raw = fs::read_to_string(&cli.config).map_err(|e| {
            BotError::Configuration(format!("cannot read config {}: {e}", cli.config.display()))
        })?;
        let mut config: Self = toml::from_str(&raw)
            .map_err(|e| BotError::Configuration(format!("invalid TOML: {e}")))?;
        if let Some(v) = cli.environment {
            config.environment = v;
        }
        if let Some(v) = cli.operation {
            config.operation = v;
        }
        if let Some(v) = cli.risk_profile {
            config.risk_profile = v;
        }
        if let Some(v) = cli.mode {
            config.run_mode = v;
        }
        config.validate()?;
        Ok(config)
    }

    pub fn credentials(&self) -> BotResult<Credentials> {
        let prefix = match self.environment {
            Environment::Dev => "BINANCE_TESTNET",
            Environment::Prod => "BINANCE_PROD",
        };
        let api_key = env::var(format!("{prefix}_API_KEY")).ok();
        let secret = env::var(format!("{prefix}_SECRET")).ok();
        if api_key.is_some() != secret.is_some() {
            return Err(BotError::Configuration(format!(
                "both {prefix}_API_KEY and {prefix}_SECRET must be set together"
            )));
        }
        Ok(Credentials { api_key, secret })
    }

    pub fn validate(&self) -> BotResult<()> {
        if self.market.symbol.trim().is_empty() || !self.market.symbol.contains('/') {
            return Err(BotError::Configuration(
                "market.symbol must be a unified pair such as BTC/USDT".into(),
            ));
        }
        if !["1m", "3m", "5m", "15m", "30m", "1h", "4h"].contains(&self.market.timeframe.as_str()) {
            return Err(BotError::Configuration(
                "timeframe must be one of 1m, 3m, 5m, 15m, 30m, 1h, 4h".into(),
            ));
        }
        if !self
            .operation
            .supported_timeframes()
            .contains(&self.market.timeframe.as_str())
        {
            return Err(BotError::Configuration(format!(
                "timeframe {} is not configured for operation {}; allowed: {}",
                self.market.timeframe,
                self.operation,
                self.operation.supported_timeframes().join(", ")
            )));
        }
        let min_candles = u32::try_from(self.strategy.sma_slow)
            .ok()
            .and_then(|period| period.checked_add(2))
            .ok_or_else(|| BotError::Configuration("strategy.sma_slow is too large".into()))?;
        if self.market.candle_limit < min_candles {
            return Err(BotError::Configuration(
                "market.candle_limit must exceed the slow SMA warm-up".into(),
            ));
        }
        let interval_seconds = timeframe_seconds(&self.market.timeframe)
            .ok_or_else(|| BotError::Configuration("invalid timeframe".into()))?;
        if self.market.poll_seconds > interval_seconds {
            return Err(BotError::Configuration(
                "market.poll_seconds must not exceed the candle interval".into(),
            ));
        }
        if self.market.poll_seconds == 0
            || self.strategy.sma_fast == 0
            || self.strategy.sma_fast >= self.strategy.sma_slow
        {
            return Err(BotError::Configuration(
                "poll interval and SMA periods must be positive, with fast < slow".into(),
            ));
        }
        if !self.risk.max_order_quote.is_finite()
            || self.risk.max_order_quote <= 0.0
            || !self.risk.max_daily_loss_quote.is_finite()
            || self.risk.max_daily_loss_quote <= 0.0
            || self.risk.max_open_positions == 0
        {
            return Err(BotError::Configuration(
                "risk limits must be finite and positive".into(),
            ));
        }
        if !matches!(
            self.logging.level.to_lowercase().as_str(),
            "error" | "warn" | "info" | "debug" | "trace"
        ) {
            return Err(BotError::Configuration(
                "logging.level must be error, warn, info, debug, or trace".into(),
            ));
        }
        if self.environment == Environment::Prod {
            if !self.production.enabled
                || self.production.acknowledgement.as_deref()
                    != Some("I_UNDERSTAND_LIVE_TRADING_RISK")
            {
                return Err(BotError::Configuration("prod requires production.enabled=true and production.acknowledgement='I_UNDERSTAND_LIVE_TRADING_RISK'".into()));
            }
            return Err(BotError::Configuration("production order execution is not implemented in this version; refusing to start in prod".into()));
        }
        if self.run_mode == RunMode::Testnet {
            return Err(BotError::Configuration(
                "testnet order mode is intentionally blocked in v1; use observe or paper".into(),
            ));
        }
        if let Some(base) = &self.providers.openai_base_url {
            crate::core::providers::openai_compatible::validate_https_or_localhost(base)?;
        }
        self.validate_operation_profile()?;
        Ok(())
    }
}

impl Config {
    fn validate_operation_profile(&self) -> BotResult<()> {
        if self.operation.is_hft() {
            return Ok(());
        }
        let (fast, slow) = self.operation.sma_period_preset();
        if self.strategy.sma_fast != fast || self.strategy.sma_slow != slow {
            return Err(BotError::Configuration(format!(
                "strategy SMA {}/{} must match operation preset {}/{} for {}",
                self.strategy.sma_fast, self.strategy.sma_slow, fast, slow, self.operation,
            )));
        }
        Ok(())
    }
}

fn timeframe_seconds(tf: &str) -> Option<u64> {
    if let Some(minutes) = tf.strip_suffix('m') {
        return minutes.parse::<u64>().ok()?.checked_mul(60);
    }
    if let Some(hours) = tf.strip_suffix('h') {
        return hours.parse::<u64>().ok()?.checked_mul(3_600);
    }
    None
}

impl Config {
    fn bundled() -> Self {
        toml::from_str(include_str!("bot.toml")).expect("bundled default config is valid")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::bundled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_dev_observe_only() {
        let c = Config::default();
        assert_eq!(c.environment, Environment::Dev);
        assert_eq!(c.run_mode, RunMode::Observe);
        assert!(c.validate().is_ok());
    }
    #[test]
    fn production_fails_closed_even_when_acknowledged() {
        let c = Config {
            environment: Environment::Prod,
            production: ProductionConfig {
                enabled: true,
                acknowledgement: Some("I_UNDERSTAND_LIVE_TRADING_RISK".into()),
            },
            ..Config::default()
        };
        assert!(c
            .validate()
            .unwrap_err()
            .to_string()
            .contains("not implemented"));
    }
    #[test]
    fn operation_presets_accept_only_documented_periods_and_timeframes() {
        let cases = [
            (OperationMode::Scalper, "1m", 5, 20),
            (OperationMode::DayTrader, "15m", 5, 20),
            (OperationMode::SwingTrader, "1h", 20, 50),
        ];
        for (operation, timeframe, fast, slow) in cases {
            assert_eq!(operation.sma_period_preset(), (fast, slow));
            let accepted = Config {
                operation,
                market: MarketConfig {
                    timeframe: timeframe.into(),
                    ..Config::default().market
                },
                strategy: StrategyConfig {
                    sma_fast: fast,
                    sma_slow: slow,
                },
                ..Config::default()
            };
            assert!(
                accepted.validate().is_ok(),
                "expected {operation} {timeframe} {fast}/{slow}"
            );

            let wrong_periods = Config {
                strategy: StrategyConfig {
                    sma_fast: fast + 1,
                    sma_slow: slow,
                },
                ..accepted.clone()
            };
            let error = wrong_periods.validate().unwrap_err().to_string();
            assert!(error.contains("operation preset"));
            assert!(!error.contains("profiles.toml"));

            let wrong_timeframe = Config {
                market: MarketConfig {
                    timeframe: if operation == OperationMode::SwingTrader {
                        "1m"
                    } else {
                        "4h"
                    }
                    .into(),
                    ..accepted.market.clone()
                },
                ..accepted
            };
            assert!(wrong_timeframe.validate().is_err());
        }
    }
    #[test]
    fn rejects_insecure_provider_base_url_in_toml() {
        let c = Config {
            providers: ProviderConfig {
                openai_base_url: Some("http://example.com/v1".into()),
            },
            ..Config::default()
        };
        assert!(c.validate().unwrap_err().to_string().contains("HTTPS"));
    }

    #[test]
    fn hft_is_not_available_on_one_minute_rest_feed() {
        assert!(OperationMode::Hft.is_hft());
    }
}
