//! Sync spot submit seam behind `ExchangeSpotExecutor` (recording + testnet REST).

use std::sync::Mutex;

use crate::modules::exchanges::adapters::binance_spot_testnet_submit::submit_testnet_spot_market_order;
use crate::modules::exchanges::credentials_env::dev_spot_order_submit_testnet_seam_enabled;
use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

static SUBMIT_CALLS: Mutex<u32> = Mutex::new(0);
static LAST_SUBMIT_ACK: Mutex<Option<SpotOrderSubmitAck>> = Mutex::new(None);

#[cfg(test)]
static RECORDING_SUBMIT_SERIAL: Mutex<()> = Mutex::new(());

#[cfg(test)]
fn with_recording_submit_serial<R>(f: impl FnOnce() -> R) -> R {
    let _guard = RECORDING_SUBMIT_SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f()
}

#[cfg(not(test))]
fn with_recording_submit_serial<R>(f: impl FnOnce() -> R) -> R {
    f()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotOrderSubmitAck {
    pub exchange_order_id: String,
}

/// Last spot submit ack (recording/testnet); consumed by HTTP reconciliation after submit.
pub fn take_last_spot_submit_ack() -> Option<SpotOrderSubmitAck> {
    LAST_SUBMIT_ACK.lock().expect("spot submit ack lock").take()
}

fn store_last_submit_ack(ack: SpotOrderSubmitAck) {
    *LAST_SUBMIT_ACK.lock().expect("spot submit ack lock") = Some(ack);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveExchangeSubmitBackend {
    Recording,
    Testnet,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RecordingSpotOrderSubmitPort;

impl RecordingSpotOrderSubmitPort {
    pub fn submit(request: &SubmitOrderRequest<'_>) -> Result<SpotOrderSubmitAck, OrdersError> {
        with_recording_submit_serial(|| {
            let _ = (request.symbol, request.side, request.quote_amount);
            let mut guard = SUBMIT_CALLS.lock().expect("spot submit lock");
            *guard += 1;
            Ok(SpotOrderSubmitAck {
                exchange_order_id: format!("recording-{}", *guard),
            })
        })
    }

    pub fn call_count() -> u32 {
        with_recording_submit_serial(|| *SUBMIT_CALLS.lock().expect("spot submit lock"))
    }

    pub fn clear() {
        with_recording_submit_serial(|| {
            *SUBMIT_CALLS.lock().expect("spot submit lock") = 0;
            *LAST_SUBMIT_ACK.lock().expect("spot submit ack lock") = None;
        });
    }
}

/// Resolved from `BOT_ORDERS_EXCHANGE_SUBMIT` (`recording` or `testnet` + credentials).
pub fn live_exchange_submit_backend() -> Option<LiveExchangeSubmitBackend> {
    match std::env::var("BOT_ORDERS_EXCHANGE_SUBMIT") {
        Ok(raw) if raw.trim().eq_ignore_ascii_case("recording") => {
            Some(LiveExchangeSubmitBackend::Recording)
        }
        Ok(raw) if raw.trim().eq_ignore_ascii_case("testnet") => {
            if dev_spot_order_submit_testnet_seam_enabled() {
                Some(LiveExchangeSubmitBackend::Testnet)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn live_exchange_submit_backend_enabled() -> bool {
    live_exchange_submit_backend().is_some()
}

pub fn submit_spot_order(
    request: &SubmitOrderRequest<'_>,
) -> Result<SpotOrderSubmitAck, OrdersError> {
    let ack = match live_exchange_submit_backend() {
        Some(LiveExchangeSubmitBackend::Recording) => {
            RecordingSpotOrderSubmitPort::submit(request)?
        }
        Some(LiveExchangeSubmitBackend::Testnet) => submit_testnet_spot_market_order(request)?,
        None => return Err(OrdersError::LiveExchangeNotWired),
    };
    store_last_submit_ack(ack.clone());
    Ok(ack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;
    use crate::modules::orders::models::OrderSide;

    #[test]
    fn testnet_backend_wired_when_credentials_configured() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            assert_eq!(
                live_exchange_submit_backend(),
                Some(LiveExchangeSubmitBackend::Testnet)
            );
            assert!(live_exchange_submit_backend_enabled());
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }

    #[test]
    fn recording_submit_returns_deterministic_exchange_order_id() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
            RecordingSpotOrderSubmitPort::clear();
            let request = SubmitOrderRequest {
                symbol: "BTC/USDT",
                side: OrderSide::Buy,
                quote_amount: 1.0,
                estimated_daily_loss: 0.0,
                open_positions: 0,
                paper_fill_unit_price: None,
                client_order_id: None,
            };
            let ack = submit_spot_order(&request).expect("recording submit");
            assert_eq!(ack.exchange_order_id, "recording-1");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            RecordingSpotOrderSubmitPort::clear();
        });
    }

    #[test]
    fn testnet_backend_unwired_without_credentials() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            assert_eq!(live_exchange_submit_backend(), None);
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }
}
