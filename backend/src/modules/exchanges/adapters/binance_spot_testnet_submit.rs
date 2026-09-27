//! Binance Spot testnet market order submit (Gate 2). Requires credentials and network.

use std::sync::LazyLock;

use ccxt_core::types::{Amount, OrderSide as CcxtOrderSide, OrderType, Price};
use ccxt_exchanges::binance::Binance;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tokio::runtime::Runtime;

use super::binance::build_dev_spot_binance;
use crate::core::config::{Credentials, Environment};
use crate::core::error::BotError;
use crate::modules::exchanges::adapters::account_file::ExchangeConfigError;
use crate::modules::exchanges::bootstrap::load_registry;
use crate::modules::exchanges::credentials_env::binance_testnet_credentials_configured;
use crate::modules::orders::models::{OrderSide, OrdersError, SubmitOrderRequest};
use crate::modules::orders::SpotOrderSubmitAck;

pub(crate) fn ccxt_runtime() -> &'static Runtime {
    static RT: LazyLock<Runtime> = LazyLock::new(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("ccxt order submit runtime")
    });
    &RT
}

const REDACTED_CREDENTIAL: &str = "<redacted>";

/// Strips configured testnet credential values from exchange error text before it becomes an HTTP/API message.
pub(crate) fn redact_known_testnet_credentials(message: &str) -> String {
    let mut out = message.to_string();
    for var in ["BINANCE_TESTNET_API_KEY", "BINANCE_TESTNET_SECRET"] {
        if let Ok(value) = std::env::var(var) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                out = out.replace(trimmed, REDACTED_CREDENTIAL);
            }
        }
    }
    out
}

pub(crate) fn map_bot_error(error: BotError) -> OrdersError {
    OrdersError::InvalidRequest(redact_known_testnet_credentials(&error.to_string()))
}

pub(crate) fn map_config_error(error: ExchangeConfigError) -> OrdersError {
    OrdersError::InvalidRequest(error.to_string())
}

pub(crate) fn testnet_credentials() -> Result<Credentials, OrdersError> {
    if !binance_testnet_credentials_configured() {
        return Err(OrdersError::LiveExchangeNotWired);
    }
    Ok(Credentials {
        api_key: std::env::var("BINANCE_TESTNET_API_KEY").ok(),
        secret: std::env::var("BINANCE_TESTNET_SECRET").ok(),
    })
}

pub(crate) fn dev_spot_account(
    registry: &crate::modules::exchanges::registry::ExchangeRegistry,
) -> Result<&crate::modules::exchanges::registry::AccountRegistration, OrdersError> {
    registry
        .spot_accounts(Environment::Dev)
        .into_iter()
        .next()
        .ok_or_else(|| OrdersError::InvalidRequest("no dev spot account registered".into()))
}

fn apply_client_order_id_param(
    params: &mut std::collections::HashMap<String, String>,
    request: &SubmitOrderRequest<'_>,
) {
    if let Some(cid) = request
        .client_order_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        params.insert("newClientOrderId".to_string(), cid.to_string());
    }
}

#[allow(deprecated)]
async fn submit_market_buy_quote_async(
    exchange: &Binance,
    request: &SubmitOrderRequest<'_>,
) -> Result<String, BotError> {
    use std::collections::HashMap;

    exchange
        .load_markets(false)
        .await
        .map_err(|e| BotError::Exchange(format!("load_markets failed: {e}")))?;
    let mut params = HashMap::new();
    apply_client_order_id_param(&mut params, request);
    params.insert("cost".to_string(), request.quote_amount.to_string());
    let amount = Amount::from(Decimal::ONE);
    let order = exchange
        .create_order(
            request.symbol,
            OrderType::Market,
            CcxtOrderSide::Buy,
            amount,
            None::<Price>,
            Some(params),
        )
        .await
        .map_err(|e| BotError::Exchange(format!("create_order failed: {e}")))?;
    Ok(order.id)
}

#[allow(deprecated)]
async fn submit_market_sell_quote_async(
    exchange: &Binance,
    request: &SubmitOrderRequest<'_>,
) -> Result<String, BotError> {
    use std::collections::HashMap;

    exchange
        .load_markets(false)
        .await
        .map_err(|e| BotError::Exchange(format!("load_markets failed: {e}")))?;
    let ticker = exchange
        .fetch_ticker(request.symbol, ())
        .await
        .map_err(|e| BotError::Exchange(format!("fetch_ticker failed: {e}")))?;
    let last = ticker
        .last
        .ok_or_else(|| BotError::Exchange("ticker has no last price".into()))?;
    let last_f = last
        .0
        .to_f64()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| BotError::Exchange("ticker last price invalid".into()))?;
    let base_amount = request.quote_amount / last_f;
    if !base_amount.is_finite() || base_amount <= 0.0 {
        return Err(BotError::Exchange(
            "quote_amount too small for market sell".into(),
        ));
    }
    let amount_decimal = Decimal::from_f64_retain(base_amount)
        .ok_or_else(|| BotError::Exchange("cannot convert order amount to decimal".into()))?;
    let amount = Amount::from(amount_decimal);
    let mut params = HashMap::new();
    apply_client_order_id_param(&mut params, request);
    let order_params = if params.is_empty() {
        None
    } else {
        Some(params)
    };
    let order = exchange
        .create_order(
            request.symbol,
            OrderType::Market,
            CcxtOrderSide::Sell,
            amount,
            None::<Price>,
            order_params,
        )
        .await
        .map_err(|e| BotError::Exchange(format!("create_order failed: {e}")))?;
    Ok(order.id)
}

/// Submits a market order sized by quote notional against Binance Spot testnet.
pub fn submit_testnet_spot_market_order(
    request: &SubmitOrderRequest<'_>,
) -> Result<SpotOrderSubmitAck, OrdersError> {
    request.validate()?;
    let credentials = testnet_credentials()?;
    let registry = load_registry(Environment::Dev).map_err(map_config_error)?;
    let account = dev_spot_account(&registry)?;
    let exchange = build_dev_spot_binance(credentials, account).map_err(map_bot_error)?;
    let exchange_order_id = ccxt_runtime()
        .block_on(async {
            match request.side {
                OrderSide::Buy => submit_market_buy_quote_async(&exchange, request).await,
                OrderSide::Sell => submit_market_sell_quote_async(&exchange, request).await,
            }
        })
        .map_err(map_bot_error)?;
    Ok(SpotOrderSubmitAck { exchange_order_id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn map_bot_error_redacts_configured_testnet_credentials_from_message() {
        with_env_test_lock(|| {
            std::env::set_var("BINANCE_TESTNET_API_KEY", "marker_key_abc");
            std::env::set_var("BINANCE_TESTNET_SECRET", "marker_secret_xyz");
            let err = BotError::Exchange(
                "create_order failed: marker_key_abc and marker_secret_xyz".into(),
            );
            let mapped = map_bot_error(err);
            let message = mapped.to_string();
            assert!(!message.contains("marker_key_abc"));
            assert!(!message.contains("marker_secret_xyz"));
            assert!(message.contains(REDACTED_CREDENTIAL));
        });
    }

    #[test]
    fn sell_without_credentials_returns_not_wired() {
        with_env_test_lock(|| {
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            let request = SubmitOrderRequest {
                symbol: "BTC/USDT",
                side: OrderSide::Sell,
                quote_amount: 5.0,
                estimated_daily_loss: 0.0,
                open_positions: 0,
                paper_fill_unit_price: None,
                client_order_id: None,
            };
            let err = submit_testnet_spot_market_order(&request).unwrap_err();
            assert_eq!(err, OrdersError::LiveExchangeNotWired);
        });
    }

    #[test]
    #[ignore = "manual: export BINANCE_TESTNET_API_KEY/SECRET then cargo test -- --ignored integration_submits_minimal_market_buy_on_testnet"]
    fn integration_submits_minimal_market_buy_on_testnet() {
        let request = SubmitOrderRequest {
            symbol: "BTC/USDT",
            side: OrderSide::Buy,
            quote_amount: 11.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            paper_fill_unit_price: None,
            client_order_id: None,
        };
        submit_testnet_spot_market_order(&request).expect("testnet market buy");
    }
}
