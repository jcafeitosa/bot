use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ExchangeId, MarketType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    SpotRestMarketData,
    SpotStreamMarketData,
    SpotOrders,
    SpotUserData,
    FuturesRestMarketData,
    FuturesStreamMarketData,
    FuturesOrders,
    FuturesUserData,
    Testnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeCapability {
    pub exchange: ExchangeId,
    pub markets: Vec<MarketType>,
    pub capabilities: Vec<Capability>,
    pub sandbox_supported: bool,
    pub notes: String,
}

pub fn catalog() -> BTreeMap<ExchangeId, ExchangeCapability> {
    BTreeMap::from([(
        ExchangeId::Binance,
        ExchangeCapability {
            exchange: ExchangeId::Binance,
            markets: vec![MarketType::Spot, MarketType::Futures],
            capabilities: vec![
                Capability::SpotRestMarketData,
                Capability::SpotStreamMarketData,
                Capability::SpotOrders,
                Capability::SpotUserData,
                Capability::FuturesRestMarketData,
                Capability::FuturesStreamMarketData,
                Capability::FuturesOrders,
                Capability::FuturesUserData,
                Capability::Testnet,
            ],
            sandbox_supported: true,
            notes: "Binance spot REST market data is implemented. Streams and order lifecycle are planned behind verification gates.".to_owned(),
        },
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_tracks_supported_markets_and_resources() {
        let catalog = catalog();
        let binance = catalog.get(&ExchangeId::Binance).unwrap();
        assert!(binance.markets.contains(&MarketType::Spot));
        assert!(binance.markets.contains(&MarketType::Futures));
        assert!(binance.sandbox_supported);
    }
}
