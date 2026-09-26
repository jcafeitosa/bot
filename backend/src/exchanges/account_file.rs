use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    registry::AccountRegistration, ExchangeAccountId, ExchangeError, ExchangeId, MarketType,
};
use crate::config::Environment;

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
            if market == MarketType::Spot && environment == Environment::Dev {
                require_exact_spot_endpoint(account.rest_base_url.as_deref(), "https", "/")?;
                require_exact_spot_endpoint(account.stream_base_url.as_deref(), "wss", "/ws")?;
            }
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
                rest_base_url: account.rest_base_url.clone(),
                stream_base_url: account.stream_base_url.clone(),
            });
        }
        Ok(out)
    }
}

pub(super) fn require_exact_spot_endpoint(
    configured: Option<&str>,
    scheme: &str,
    path: &str,
) -> Result<(), ExchangeError> {
    let raw = configured.ok_or(ExchangeError::InvalidEndpoint)?;
    let expected = if scheme == "https" {
        "https://testnet.binance.vision"
    } else {
        "wss://testnet.binance.vision/ws"
    };
    if raw != expected && !(scheme == "https" && raw == "https://testnet.binance.vision/") {
        return Err(ExchangeError::InvalidEndpoint);
    }
    let url = reqwest::Url::parse(raw).map_err(|_| ExchangeError::InvalidEndpoint)?;
    if url.scheme() != scheme
        || url.host_str() != Some("testnet.binance.vision")
        || url.port().is_some()
        || url.path() != path
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ExchangeError::InvalidEndpoint);
    }
    Ok(())
}

fn require_http_url(url: &str) -> Result<(), ExchangeError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ExchangeError::InvalidEndpoint)?;
    if (parsed.scheme() == "https"
        || (parsed.scheme() == "http" && parsed.host_str() == Some("localhost")))
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
    {
        Ok(())
    } else {
        Err(ExchangeError::InvalidEndpoint)
    }
}

fn require_ws_url(url: &str) -> Result<(), ExchangeError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ExchangeError::InvalidEndpoint)?;
    if (parsed.scheme() == "wss"
        || (parsed.scheme() == "ws" && parsed.host_str() == Some("localhost")))
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
    {
        Ok(())
    } else {
        Err(ExchangeError::InvalidEndpoint)
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
        let spot = accounts
            .iter()
            .find(|account| account.id.market == super::super::MarketType::Spot)
            .unwrap();
        assert_eq!(
            spot.rest_base_url.as_deref(),
            Some("https://testnet.binance.vision")
        );
        assert_eq!(
            spot.stream_base_url.as_deref(),
            Some("wss://testnet.binance.vision/ws")
        );
    }

    #[test]
    fn dev_spot_rejects_urls_outside_exact_testnet_allowlist() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/exchanges/binance.toml");
        let original = ExchangeConfigFile::load(&path).unwrap();
        for bad_rest in [
            "https://testnet.binance.vision.evil.test",
            "https://api.binance.com",
            "http://localhost:9000",
            "https://testnet.binance.vision@evil.test",
            "https://testnet.binance.vision/path",
            "https://testnet.binance.vision?x=1",
            "https://testnet.binance.vision:443",
        ] {
            let mut file = original.clone();
            file.accounts.get_mut("spot").unwrap().rest_base_url = Some(bad_rest.into());
            assert!(
                file.registrations(Environment::Dev).is_err(),
                "accepted {bad_rest}"
            );
        }
        let mut file = original;
        file.accounts.get_mut("spot").unwrap().stream_base_url = Some("wss://evil.test/ws".into());
        assert!(file.registrations(Environment::Dev).is_err());
    }
}
