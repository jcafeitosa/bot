use crate::core::config::Environment;
use crate::modules::exchanges::rest::{authorize_rest_use, RestUse};
use crate::modules::exchanges::{ExchangeAccountId, ExchangeError, ExchangeId, MarketType};
use crate::modules::orders::models::OrdersError;

/// Fail-closed REST authorization for order submit (shared policy with `modules::exchanges::rest`).
pub fn gate_order_submit(account: &ExchangeAccountId) -> Result<(), OrdersError> {
    authorize_rest_use(account, RestUse::OrderSubmit).map_err(map_exchange_order_error)
}

fn map_exchange_order_error(error: ExchangeError) -> OrdersError {
    match error {
        ExchangeError::ExecutionDisabled => OrdersError::LiveExchangeNotWired,
        other => OrdersError::InvalidRequest(other.to_string()),
    }
}

pub fn default_dev_spot_account() -> ExchangeAccountId {
    ExchangeAccountId::new(
        ExchangeId::Binance,
        MarketType::Spot,
        "order-gate",
        Environment::Dev,
    )
    .expect("valid dev spot account id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn order_submit_gate_matches_exchange_rest_policy() {
        with_env_test_lock(|| {
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            let account = default_dev_spot_account();
            assert_eq!(
                gate_order_submit(&account).unwrap_err(),
                OrdersError::LiveExchangeNotWired
            );
        });
    }

    #[test]
    fn order_submit_gate_passes_when_recording_seam_enabled() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
            let account = default_dev_spot_account();
            gate_order_submit(&account).expect("recording seam");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }

    #[test]
    fn order_submit_gate_passes_for_testnet_seam_when_credentials_configured() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            let account = default_dev_spot_account();
            gate_order_submit(&account).expect("testnet REST policy");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }
}
