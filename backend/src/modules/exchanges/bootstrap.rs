use std::fs;
use std::path::Path;

use crate::core::config::Environment;
use crate::core::error::BotError;

use super::account_file::{ExchangeConfigError, ExchangeConfigFile};
use super::registry::{AccountRegistration, ExchangeRegistry};
use super::MarketType;

const EXCHANGE_CONFIG_DIR: &str = "src/core/config/exchanges";

pub fn load_registry(environment: Environment) -> Result<ExchangeRegistry, ExchangeConfigError> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(EXCHANGE_CONFIG_DIR);
    let mut registry = ExchangeRegistry::default();
    let entries = fs::read_dir(&dir).map_err(|error| ExchangeConfigError::Read {
        path: dir.display().to_string(),
        detail: error.to_string(),
    })?;
    let mut loaded = 0usize;
    for entry in entries {
        let path = entry
            .map_err(|error| ExchangeConfigError::Read {
                path: dir.display().to_string(),
                detail: error.to_string(),
            })?
            .path();
        if path
            .file_name()
            .is_some_and(|name| name == "credentials.toml")
        {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "toml") {
            let file = ExchangeConfigFile::load(&path)?;
            for account in file.registrations(environment)? {
                registry.register(account)?;
            }
            loaded += 1;
        }
    }
    if loaded == 0 {
        return Err(ExchangeConfigError::Empty);
    }
    Ok(registry)
}

pub fn spot_account_for_symbol<'a>(
    registry: &'a ExchangeRegistry,
    environment: Environment,
    symbol: &str,
) -> Result<&'a AccountRegistration, BotError> {
    registry
        .spot_accounts(environment)
        .into_iter()
        .find(|account| account.symbols.iter().any(|s| s == symbol))
        .ok_or_else(|| {
            BotError::Configuration(format!(
                "market.symbol {symbol} is not listed on any registered spot account in {EXCHANGE_CONFIG_DIR}"
            ))
        })
}

pub fn registered_market_types(
    registry: &ExchangeRegistry,
    environment: Environment,
) -> Vec<MarketType> {
    let mut markets = Vec::new();
    if !registry.spot_accounts(environment).is_empty() {
        markets.push(MarketType::Spot);
    }
    if !registry.futures_accounts(environment).is_empty() {
        markets.push(MarketType::Futures);
    }
    markets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;

    #[test]
    fn loads_binance_accounts_from_config_dir() {
        let registry = load_registry(Environment::Dev).expect("registry");
        assert!(!registry.spot_accounts(Environment::Dev).is_empty());
        assert!(spot_account_for_symbol(&registry, Environment::Dev, "BTC/USDT").is_ok());
    }
}
