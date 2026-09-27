//! Binance Spot testnet order observation for reconciliation (requires credentials + network).

#[allow(unused_imports)]
use ccxt_core::traits::Trading as _;
use ccxt_core::types::OrderStatus;
use ccxt_exchanges::binance::Binance;

use super::binance::build_dev_spot_binance;
use super::binance_spot_testnet_submit::{
    ccxt_runtime, dev_spot_account, map_bot_error, map_config_error, testnet_credentials,
};
use crate::core::config::Environment;
use crate::modules::exchanges::bootstrap::load_registry;
use crate::modules::orders::adapters::RemoteOrderObservation;
use crate::modules::orders::models::OrdersError;

async fn fetch_order_by_client_id_async(
    exchange: &Binance,
    client_order_id: &str,
    symbol: &str,
) -> Result<ccxt_core::types::Order, crate::core::error::BotError> {
    exchange
        .load_markets(false)
        .await
        .map_err(|e| crate::core::error::BotError::Exchange(format!("load_markets failed: {e}")))?;
    exchange
        .fetch_order(client_order_id, symbol)
        .await
        .map_err(|e| crate::core::error::BotError::Exchange(format!("fetch_order failed: {e}")))
}

fn map_order_status(order: &ccxt_core::types::Order) -> RemoteOrderObservation {
    match order.status {
        OrderStatus::Closed => RemoteOrderObservation::Confirmed {
            exchange_order_id: order.id.clone(),
        },
        OrderStatus::Cancelled => RemoteOrderObservation::Divergent {
            reason: "exchange order canceled".into(),
        },
        OrderStatus::Open | OrderStatus::Partial => RemoteOrderObservation::StillPending,
        _ => RemoteOrderObservation::StillPending,
    }
}

/// Observes testnet order state by `client_order_id` (ccxt `fetch_order` id). Errors → still pending.
pub fn observe_testnet_spot_order_by_client_id(
    client_order_id: &str,
    symbol: &str,
) -> Result<RemoteOrderObservation, OrdersError> {
    let key = client_order_id.trim();
    if key.is_empty() {
        return Ok(RemoteOrderObservation::StillPending);
    }
    if symbol.trim().is_empty() {
        return Err(OrdersError::InvalidRequest("symbol is required".into()));
    }
    if testnet_credentials().is_err() {
        return Ok(RemoteOrderObservation::StillPending);
    }
    let credentials = testnet_credentials()?;
    let registry = load_registry(Environment::Dev).map_err(map_config_error)?;
    let account = dev_spot_account(&registry)?;
    let exchange = build_dev_spot_binance(credentials, account).map_err(map_bot_error)?;
    match ccxt_runtime().block_on(fetch_order_by_client_id_async(&exchange, key, symbol)) {
        Ok(order) => Ok(map_order_status(&order)),
        Err(_) => Ok(RemoteOrderObservation::StillPending),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn map_closed_status_to_confirmed() {
        use ccxt_core::types::{OrderSide as CcxtSide, OrderType};
        use rust_decimal::Decimal;
        let order = ccxt_core::types::Order::new(
            "ex-1".into(),
            "BTC/USDT".into(),
            OrderType::Market,
            CcxtSide::Buy,
            Decimal::ONE,
            None,
            OrderStatus::Closed,
        );
        assert_eq!(
            map_order_status(&order),
            RemoteOrderObservation::Confirmed {
                exchange_order_id: "ex-1".into(),
            }
        );
    }

    #[test]
    fn map_cancelled_status_to_divergent() {
        use ccxt_core::types::{OrderSide as CcxtSide, OrderType};
        use rust_decimal::Decimal;
        let order = ccxt_core::types::Order::new(
            "ex-2".into(),
            "BTC/USDT".into(),
            OrderType::Market,
            CcxtSide::Sell,
            Decimal::ONE,
            None,
            OrderStatus::Cancelled,
        );
        assert_eq!(
            map_order_status(&order),
            RemoteOrderObservation::Divergent {
                reason: "exchange order canceled".into(),
            }
        );
    }

    #[test]
    fn observe_without_credentials_stays_pending() {
        with_env_test_lock(|| {
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            let obs =
                observe_testnet_spot_order_by_client_id("cid-1", "BTC/USDT").expect("observe");
            assert_eq!(obs, RemoteOrderObservation::StillPending);
        });
    }
}
