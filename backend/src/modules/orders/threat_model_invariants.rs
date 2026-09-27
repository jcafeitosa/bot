//! Executable proofs for orders Gate 2 threat model.
//!
//! See `docs/sdd/orders-live-execution-gate2-sdd.md` § Threat model — invariantes verificáveis.

use crate::core::config::Environment;
use crate::core::test_env_lock::with_env_test_lock;
use crate::modules::exchanges::rest::{authorize_rest_use, RestUse};
use crate::modules::exchanges::{ExchangeAccountId, ExchangeError, ExchangeId, MarketType};
use crate::modules::http_bridge::orders::{
    submit_order_http, OrderSideBody, SubmitOrderHttpRequest,
};
use crate::modules::http_bridge::risk::RiskLimitsBody;
use crate::modules::orders::gate_order_submit;
use crate::modules::orders::{submit_order, OrderSide};
use crate::modules::orders::{
    FailClosedExecutor, InMemoryOrderIdempotencyStore, OrdersError, RecordingExecutor,
    SubmitOrderRequest,
};
use crate::modules::risk::models::RiskLimits;

fn prod_spot_account(label: &str) -> ExchangeAccountId {
    ExchangeAccountId::new(
        ExchangeId::Binance,
        MarketType::Spot,
        label,
        Environment::Prod,
    )
    .expect("valid prod spot account id")
}

#[test]
fn g2_threat_model_invariant_prod_spot_order_submit_rest_denied() {
    with_env_test_lock(|| {
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        let account = prod_spot_account("threat-model-prod");
        assert_eq!(
            authorize_rest_use(&account, RestUse::OrderSubmit),
            Err(ExchangeError::ExecutionDisabled)
        );
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    });
}

#[test]
fn g2_threat_model_invariant_gate_order_submit_aligns_with_rest_prod_denial() {
    with_env_test_lock(|| {
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        let account = prod_spot_account("threat-model-gate");
        assert_eq!(
            gate_order_submit(&account).unwrap_err(),
            OrdersError::LiveExchangeNotWired
        );
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    });
}

#[test]
fn g2_threat_model_invariant_client_order_id_replay_executes_once() {
    let store = InMemoryOrderIdempotencyStore::new();
    let executor = RecordingExecutor::new();
    let body = SubmitOrderHttpRequest {
        symbol: "BTC/USDT".into(),
        side: OrderSideBody::Buy,
        quote_amount: 5.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
        limits: RiskLimitsBody {
            max_order_quote: 10.0,
            max_daily_loss_quote: 20.0,
            max_open_positions: 1,
        },
        client_order_id: Some("g2-threat-model-replay".into()),
        paper_fill_unit_price: None,
    };
    submit_order_http(body.clone(), &executor, &store, None).expect("first submit");
    submit_order_http(body, &executor, &store, None).expect("idempotent replay");
    assert_eq!(executor.call_count(), 1);
}

#[test]
fn g2_threat_model_invariant_risk_rejects_before_fail_closed_executor() {
    let request = SubmitOrderRequest {
        symbol: "BTC/USDT",
        side: OrderSide::Buy,
        quote_amount: 50.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
        paper_fill_unit_price: None,
        client_order_id: None,
    };
    let limits = RiskLimits {
        max_order_quote: 10.0,
        max_daily_loss_quote: 20.0,
        max_open_positions: 1,
    };
    let err = submit_order(request, limits, &FailClosedExecutor).unwrap_err();
    assert!(matches!(err, OrdersError::RiskRejected(_)));
}
