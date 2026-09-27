use super::credentials_env::dev_spot_order_submit_testnet_seam_enabled;
use super::{ExchangeAccountId, ExchangeError, ExchangeId, MarketType, Transport};
use crate::core::config::Environment;

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

fn dev_spot_order_submit_recording_seam_enabled() -> bool {
    crate::core::config::exchange_submit_recording_enabled()
}

pub fn authorize_rest_use(
    account: &ExchangeAccountId,
    operation: RestUse,
) -> Result<(), ExchangeError> {
    if account.exchange == ExchangeId::Binance
        && account.market == MarketType::Spot
        && account.environment == Environment::Dev
    {
        if operation == RestUse::HistoricalBackfill {
            return Ok(());
        }
        if operation == RestUse::OrderSubmit
            && (dev_spot_order_submit_recording_seam_enabled()
                || dev_spot_order_submit_testnet_seam_enabled())
        {
            return Ok(());
        }
    }
    Err(ExchangeError::ExecutionDisabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;
    use crate::core::test_env_lock::with_env_test_lock;
    use crate::modules::exchanges::{ExchangeId, MarketType};

    #[test]
    fn order_rest_paths_stay_disabled() {
        with_env_test_lock(|| {
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
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
        });
    }

    #[test]
    fn order_submit_allowed_for_dev_spot_when_recording_seam_env_set() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
            let account = ExchangeAccountId::new(
                ExchangeId::Binance,
                MarketType::Spot,
                "paper-main",
                Environment::Dev,
            )
            .unwrap();
            assert_eq!(authorize_rest_use(&account, RestUse::OrderSubmit), Ok(()));
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }

    #[test]
    fn order_submit_allowed_for_dev_spot_when_testnet_seam_and_credentials_set() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            let account = ExchangeAccountId::new(
                ExchangeId::Binance,
                MarketType::Spot,
                "paper-main",
                Environment::Dev,
            )
            .unwrap();
            assert_eq!(authorize_rest_use(&account, RestUse::OrderSubmit), Ok(()));
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }

    #[test]
    fn order_submit_stays_disabled_for_testnet_seam_without_credentials() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
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
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }

    #[test]
    fn only_public_spot_dev_backfill_is_allowed() {
        with_env_test_lock(|| {
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
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
        });
    }
}
