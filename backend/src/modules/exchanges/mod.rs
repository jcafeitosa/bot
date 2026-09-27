pub use adapters::account_file;
pub mod adapters;
pub mod controllers;
pub use adapters::binance;
pub mod bootstrap;
pub mod capabilities;
pub use adapters::live;
pub mod market_data;
pub mod models;
pub mod preflight;
pub mod registry;
pub mod resources;
pub mod rest;
pub mod router;
pub mod stream;
pub mod ws;

pub use market_data::MarketDataSource;
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;

    #[test]
    fn account_key_is_stable_and_unique_per_market() {
        let spot = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            Environment::Dev,
        )
        .unwrap();
        let futures = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Futures,
            "main",
            Environment::Dev,
        )
        .unwrap();
        assert_ne!(spot.key(), futures.key());
    }

    #[test]
    fn rejects_empty_account_label() {
        assert_eq!(
            ExchangeAccountId::new(ExchangeId::Binance, MarketType::Spot, " ", Environment::Dev)
                .unwrap_err(),
            ExchangeError::InvalidAccountLabel
        );
    }
}
