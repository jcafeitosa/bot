use super::{
    rest::RestUse,
    stream::{StreamKind, StreamSubscription},
    ExchangeAccountId, ExchangeError, Transport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketNeed {
    HistoricalBackfill,
    LiveIncremental,
    OrderLifecycle,
    BalanceSnapshot,
    UserData,
}

pub fn route_market_need(
    _account: &ExchangeAccountId,
    need: MarketNeed,
) -> Result<Transport, ExchangeError> {
    match need {
        MarketNeed::HistoricalBackfill
        | MarketNeed::OrderLifecycle
        | MarketNeed::BalanceSnapshot => Ok(Transport::Rest),
        MarketNeed::LiveIncremental | MarketNeed::UserData => Ok(Transport::StreamWs),
    }
}

pub fn rest_use_for_need(need: MarketNeed) -> Option<RestUse> {
    match need {
        MarketNeed::HistoricalBackfill => Some(RestUse::HistoricalBackfill),
        MarketNeed::OrderLifecycle => Some(RestUse::OrderStatus),
        MarketNeed::BalanceSnapshot => Some(RestUse::BalanceSnapshot),
        MarketNeed::LiveIncremental | MarketNeed::UserData => None,
    }
}

pub fn default_live_streams(account: &ExchangeAccountId, symbol: &str) -> Vec<StreamSubscription> {
    [StreamKind::Kline1m, StreamKind::MiniTicker]
        .into_iter()
        .map(|stream| StreamSubscription {
            account: account.clone(),
            stream,
            symbol: symbol.to_owned(),
            transport: Transport::StreamWs,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;
    use crate::exchanges::ExchangeId;

    fn account(market: super::super::MarketType) -> ExchangeAccountId {
        ExchangeAccountId::new(ExchangeId::Binance, market, "paper-main", Environment::Dev).unwrap()
    }

    #[test]
    fn historical_and_orders_use_rest_while_live_uses_stream() {
        use super::super::MarketType;
        let account = account(MarketType::Spot);
        assert_eq!(
            route_market_need(&account, MarketNeed::HistoricalBackfill).unwrap(),
            Transport::Rest
        );
        assert_eq!(
            route_market_need(&account, MarketNeed::LiveIncremental).unwrap(),
            Transport::StreamWs
        );
        assert_eq!(
            route_market_need(&account, MarketNeed::OrderLifecycle).unwrap(),
            Transport::Rest
        );
        assert_eq!(
            route_market_need(&account, MarketNeed::UserData).unwrap(),
            Transport::StreamWs
        );
    }
}
