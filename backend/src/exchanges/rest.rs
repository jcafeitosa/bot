use super::{ExchangeAccountId, ExchangeError, ExchangeId, MarketType, Transport};
use crate::config::Environment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestUse {
    HistoricalBackfill,
    BalanceSnapshot,
    OrderSubmit,
    OrderStatus,
    OrderCancel,
}

impl RestUse {
    pub fn transport(self) -> Transport {
        Transport::Rest
    }

    pub fn requires_execution_gate(self) -> bool {
        matches!(
            self,
            Self::OrderSubmit | Self::OrderStatus | Self::OrderCancel
        )
    }
}

pub fn authorize_rest_use(
    account: &ExchangeAccountId,
    operation: RestUse,
) -> Result<(), ExchangeError> {
    if account.exchange == ExchangeId::Binance
        && account.market == MarketType::Spot
        && account.environment == Environment::Dev
        && operation == RestUse::HistoricalBackfill
    {
        Ok(())
    } else {
        // Private REST and order paths remain disabled.
        Err(ExchangeError::ExecutionDisabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;
    use crate::exchanges::{ExchangeId, MarketType};

    #[test]
    fn order_rest_paths_stay_disabled() {
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        assert_eq!(
            authorize_rest_use(&account, RestUse::OrderSubmit).unwrap_err(),
            ExchangeError::ExecutionDisabled
        );
    }

    #[test]
    fn only_public_spot_dev_backfill_is_allowed() {
        let spot = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        assert_eq!(
            authorize_rest_use(&spot, RestUse::HistoricalBackfill),
            Ok(())
        );
        for private_use in [
            RestUse::BalanceSnapshot,
            RestUse::OrderSubmit,
            RestUse::OrderStatus,
            RestUse::OrderCancel,
        ] {
            assert_eq!(
                authorize_rest_use(&spot, private_use),
                Err(ExchangeError::ExecutionDisabled)
            );
        }
        let futures = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Futures,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        assert_eq!(
            authorize_rest_use(&futures, RestUse::HistoricalBackfill),
            Err(ExchangeError::ExecutionDisabled)
        );
        let prod = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Prod,
        )
        .unwrap();
        assert_eq!(
            authorize_rest_use(&prod, RestUse::HistoricalBackfill),
            Err(ExchangeError::ExecutionDisabled)
        );
    }
}
