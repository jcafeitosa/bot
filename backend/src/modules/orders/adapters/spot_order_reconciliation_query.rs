//! Exchange observation seam for the reconciliation poller (recording + testnet ccxt).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::spot_order_submit::{live_exchange_submit_backend, LiveExchangeSubmitBackend};
use crate::modules::exchanges::adapters::binance_spot_testnet_reconcile::observe_testnet_spot_order_by_client_id;
use crate::modules::orders::models::{OrderSide, OrdersError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteOrderObservation {
    Confirmed { exchange_order_id: String },
    StillPending,
    Divergent { reason: String },
}

pub trait SpotOrderReconciliationQuery: Send + Sync {
    fn observe(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
    ) -> Result<RemoteOrderObservation, OrdersError>;
}

static RECORDING_CLIENT_BINDINGS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Associates a `client_order_id` with the exchange id returned by the recording submit port.
pub fn recording_bind_client_exchange(client_order_id: &str, exchange_order_id: &str) {
    let key = client_order_id.trim();
    let exchange_id = exchange_order_id.trim();
    if key.is_empty() || exchange_id.is_empty() {
        return;
    }
    RECORDING_CLIENT_BINDINGS
        .lock()
        .expect("recording reconciliation bindings lock")
        .insert(key.to_string(), exchange_id.to_string());
}

#[cfg(test)]
pub fn clear_recording_client_bindings() {
    RECORDING_CLIENT_BINDINGS
        .lock()
        .expect("recording reconciliation bindings lock")
        .clear();
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RecordingSpotOrderReconciliationQuery;

impl SpotOrderReconciliationQuery for RecordingSpotOrderReconciliationQuery {
    fn observe(
        &self,
        client_order_id: &str,
        _symbol: &str,
        _side: OrderSide,
    ) -> Result<RemoteOrderObservation, OrdersError> {
        let key = client_order_id.trim();
        let map = RECORDING_CLIENT_BINDINGS
            .lock()
            .expect("recording reconciliation bindings lock");
        Ok(match map.get(key) {
            Some(exchange_order_id) => RemoteOrderObservation::Confirmed {
                exchange_order_id: exchange_order_id.clone(),
            },
            None => RemoteOrderObservation::StillPending,
        })
    }
}

/// Dispatches observation to the binding map for any wired live submit backend (recording or testnet).
#[derive(Debug, Clone, Copy, Default)]
pub struct LiveExchangeSpotOrderReconciliationQuery;

impl SpotOrderReconciliationQuery for LiveExchangeSpotOrderReconciliationQuery {
    fn observe(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
    ) -> Result<RemoteOrderObservation, OrdersError> {
        match live_exchange_submit_backend() {
            Some(LiveExchangeSubmitBackend::Recording) => {
                RecordingSpotOrderReconciliationQuery.observe(client_order_id, symbol, side)
            }
            Some(LiveExchangeSubmitBackend::Testnet) => {
                let binding =
                    RecordingSpotOrderReconciliationQuery.observe(client_order_id, symbol, side)?;
                if !matches!(binding, RemoteOrderObservation::StillPending) {
                    return Ok(binding);
                }
                observe_testnet_spot_order_by_client_id(client_order_id, symbol)
            }
            None => Ok(RemoteOrderObservation::StillPending),
        }
    }
}

#[cfg(test)]
mod live_exchange_query_tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn live_query_confirms_via_binding_when_testnet_backend_wired() {
        with_env_test_lock(|| {
            clear_recording_client_bindings();
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            recording_bind_client_exchange("tn-cid", "exchange-99");
            let obs = LiveExchangeSpotOrderReconciliationQuery
                .observe("tn-cid", "BTC/USDT", OrderSide::Buy)
                .expect("observe");
            assert_eq!(
                obs,
                RemoteOrderObservation::Confirmed {
                    exchange_order_id: "exchange-99".into(),
                }
            );
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            clear_recording_client_bindings();
        });
    }

    #[test]
    fn live_query_stays_pending_when_no_backend_wired() {
        with_env_test_lock(|| {
            clear_recording_client_bindings();
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            recording_bind_client_exchange("orphan", "x");
            let obs = LiveExchangeSpotOrderReconciliationQuery
                .observe("orphan", "BTC/USDT", OrderSide::Buy)
                .expect("observe");
            assert_eq!(obs, RemoteOrderObservation::StillPending);
            clear_recording_client_bindings();
        });
    }

    #[test]
    fn live_query_testnet_falls_through_to_ccxt_when_binding_missing() {
        with_env_test_lock(|| {
            clear_recording_client_bindings();
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            let obs = LiveExchangeSpotOrderReconciliationQuery
                .observe("no-binding", "BTC/USDT", OrderSide::Buy)
                .expect("observe");
            assert_eq!(obs, RemoteOrderObservation::StillPending);
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }
}
