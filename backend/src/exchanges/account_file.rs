use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{registry::AccountRegistration, ExchangeAccountId, ExchangeError, ExchangeId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeFileConfig {
    pub account_label: String,
    pub symbols: Vec<String>,
    pub rate_limit_per_minute: u32,
    pub rest_base_url: Option<String>,
    pub stream_base_url: Option<String>,
}

impl ExchangeFileConfig {
    fn validate(&self, exchange: ExchangeId) -> Result<(), ExchangeError> {
        if self.account_label.trim().is_empty() || self.symbols.is_empty() {
            return Err(ExchangeError::UnsupportedMarket);
        }
        if let Some(url) = self.rest_base_url.as_deref() {
            require_http_url(url)?;
        }
        if let Some(url) = self.stream_base_url.as_deref() {
            require_ws_url(url)?;
        }
        let _ = exchange;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfigFile {
    pub exchange: ExchangeId,
    pub accounts: BTreeMap<String, ExchangeFileConfig>,
}

impl ExchangeConfigFile {
    pub fn load(path: &Path) -> Result<Self, ExchangeConfigError> {
        let raw = fs::read_to_string(path).map_err(|error| ExchangeConfigError::Read {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?;
        let file: Self = toml::from_str(&raw).map_err(|error| ExchangeConfigError::Parse {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?;
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> Result<(), ExchangeConfigError> {
        if self.accounts.is_empty() {
            return Err(ExchangeConfigError::Empty);
        }
        for (market, account) in &self.accounts {
            if !matches!(market.as_str(), "spot" | "futures") {
                return Err(ExchangeConfigError::UnsupportedMarket(market.clone()));
            }
            account
                .validate(self.exchange)
                .map_err(ExchangeConfigError::Account)?;
        }
        Ok(())
    }

    pub fn registrations(
        &self,
        environment: crate::config::Environment,
    ) -> Result<Vec<AccountRegistration>, ExchangeConfigError> {
        let mut out = Vec::new();
        for (market, account) in &self.accounts {
            let market = match market.as_str() {
                "spot" => super::MarketType::Spot,
                "futures" => super::MarketType::Futures,
                other => return Err(ExchangeConfigError::UnsupportedMarket(other.to_owned())),
            };
            let id = ExchangeAccountId::new(
                self.exchange,
                market,
                account.account_label.clone(),
                environment,
            )
            .map_err(ExchangeConfigError::Account)?;
            out.push(AccountRegistration {
                id,
                symbols: account.symbols.clone(),
                rate_limit_per_minute: account.rate_limit_per_minute.max(1),
            });
        }
        Ok(out)
    }
}

fn require_http_url(url: &str) -> Result<(), ExchangeError> {
    if url.starts_with("https://") || url.starts_with("http://localhost") {
        Ok(())
    } else {
        Err(ExchangeError::UnsupportedMarket)
    }
}

fn require_ws_url(url: &str) -> Result<(), ExchangeError> {
    if url.starts_with("wss://") || url.starts_with("ws://localhost") {
        Ok(())
    } else {
        Err(ExchangeError::UnsupportedMarket)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExchangeConfigError {
    #[error("cannot read {path}: {detail}")]
    Read { path: String, detail: String },
    #[error("cannot parse {path}: {detail}")]
    Parse { path: String, detail: String },
    #[error("exchange config has no accounts")]
    Empty,
    #[error("unsupported market section: {0}")]
    UnsupportedMarket(String),
    #[error(transparent)]
    Account(#[from] ExchangeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;

    #[test]
    fn exchange_config_file_registers_spot_and_futures_accounts() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/exchanges/binance.toml");
        let file = ExchangeConfigFile::load(&path).unwrap();
        let accounts = file.registrations(Environment::Dev).unwrap();
        assert_eq!(accounts.len(), 2);
    }
}
