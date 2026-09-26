use std::collections::BTreeMap;

use crate::config::Environment;

use super::{ExchangeAccountId, ExchangeError, ExchangeId, MarketType};

#[derive(Debug, Clone)]
pub struct AccountRegistration {
    pub id: ExchangeAccountId,
    pub symbols: Vec<String>,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Default)]
pub struct ExchangeRegistry {
    accounts: BTreeMap<String, AccountRegistration>,
}

impl ExchangeRegistry {
    pub fn register(&mut self, account: AccountRegistration) -> Result<(), ExchangeError> {
        if account.symbols.is_empty() || account.rate_limit_per_minute == 0 {
            return Err(ExchangeError::UnsupportedMarket);
        }
        let key = account.id.key();
        if self.accounts.insert(key, account).is_some() {
            return Err(ExchangeError::UnsupportedMarket);
        }
        Ok(())
    }

    pub fn get(&self, id: &ExchangeAccountId) -> Result<&AccountRegistration, ExchangeError> {
        self.accounts
            .get(&id.key())
            .ok_or(ExchangeError::UnknownAccount)
    }

    pub fn spot_accounts(&self, environment: Environment) -> Vec<&AccountRegistration> {
        self.accounts
            .values()
            .filter(|account| {
                account.id.market == MarketType::Spot && account.id.environment == environment
            })
            .collect()
    }

    pub fn futures_accounts(&self, environment: Environment) -> Vec<&AccountRegistration> {
        self.accounts
            .values()
            .filter(|account| {
                account.id.market == MarketType::Futures && account.id.environment == environment
            })
            .collect()
    }
}

pub fn default_dev_accounts() -> Vec<AccountRegistration> {
    [MarketType::Spot, MarketType::Futures]
        .into_iter()
        .map(|market| AccountRegistration {
            id: ExchangeAccountId {
                exchange: ExchangeId::Binance,
                market,
                account_label: "paper-main".to_owned(),
                environment: Environment::Dev,
            },
            symbols: vec!["BTC/USDT".to_owned()],
            rate_limit_per_minute: 600,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_partitions_spot_and_futures_accounts() {
        let mut registry = ExchangeRegistry::default();
        for account in default_dev_accounts() {
            registry.register(account).unwrap();
        }
        assert_eq!(registry.spot_accounts(Environment::Dev).len(), 1);
        assert_eq!(registry.futures_accounts(Environment::Dev).len(), 1);
    }

    #[test]
    fn registry_rejects_unknown_account_lookup() {
        let registry = ExchangeRegistry::default();
        let missing = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "missing",
            Environment::Dev,
        )
        .unwrap();
        assert_eq!(
            registry.get(&missing).unwrap_err(),
            ExchangeError::UnknownAccount
        );
    }
}
