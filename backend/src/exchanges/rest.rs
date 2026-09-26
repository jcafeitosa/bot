use super::{ExchangeAccountId, ExchangeError, Transport};

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
    _account: &ExchangeAccountId,
    _operation: RestUse,
) -> Result<(), ExchangeError> {
    // Execution stays disabled until sandbox verification, idempotency,
    // reconciliation, kill-switch, audit and independent review land.
    Err(ExchangeError::ExecutionDisabled)
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
}
