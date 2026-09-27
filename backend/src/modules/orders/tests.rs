use super::{
    submit_order, AcceptingExecutor, ExchangeSpotExecutor, FailClosedExecutor,
    InMemoryOrderReconciliationLedger, OrderReconciliationLedger, OrderSide, OrdersError,
    PaperLedgerExecutor, ReconciliationState, RecordingExecutor, RecordingSpotOrderSubmitPort,
    ReservedLiveExchangeExecutor, SubmitOrderRequest,
};
use crate::modules::risk::models::RiskLimits;

use crate::core::test_env_lock::with_env_test_lock;

fn sample_request() -> SubmitOrderRequest<'static> {
    SubmitOrderRequest {
        symbol: "BTC/USDT",
        side: OrderSide::Buy,
        quote_amount: 5.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
        paper_fill_unit_price: None,
        client_order_id: None,
    }
}

fn sample_limits() -> RiskLimits {
    RiskLimits {
        max_order_quote: 10.0,
        max_daily_loss_quote: 20.0,
        max_open_positions: 1,
    }
}

#[test]
fn submit_rejects_quote_over_risk_cap() {
    let request = SubmitOrderRequest {
        symbol: "BTC/USDT",
        side: OrderSide::Buy,
        quote_amount: 50.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
        paper_fill_unit_price: None,
        client_order_id: None,
    };
    let err = submit_order(request, sample_limits(), &FailClosedExecutor).unwrap_err();
    assert!(matches!(err, OrdersError::RiskRejected(_)));
}

#[test]
fn submit_returns_execution_disabled_after_risk_passes() {
    let err = submit_order(sample_request(), sample_limits(), &FailClosedExecutor).unwrap_err();
    assert_eq!(err, OrdersError::ExecutionDisabled);
}

#[test]
fn submit_succeeds_when_port_accepts() {
    submit_order(sample_request(), sample_limits(), &AcceptingExecutor).expect("accepting port");
}

#[test]
fn submit_invokes_recording_executor_once_after_risk() {
    let executor = RecordingExecutor::new();
    submit_order(sample_request(), sample_limits(), &executor).expect("risk ok");
    assert_eq!(executor.call_count(), 1);
}

#[test]
fn recording_executor_accumulates_successful_executions() {
    let executor = RecordingExecutor::new();
    submit_order(sample_request(), sample_limits(), &executor).expect("first");
    submit_order(sample_request(), sample_limits(), &executor).expect("second");
    assert_eq!(executor.call_count(), 2);
}

#[test]
fn exchange_spot_executor_records_submit_when_rest_gate_open() {
    with_env_test_lock(|| {
        std::env::remove_var("BINANCE_TESTNET_API_KEY");
        std::env::remove_var("BINANCE_TESTNET_SECRET");
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        RecordingSpotOrderSubmitPort::clear();
        submit_order(sample_request(), sample_limits(), &ExchangeSpotExecutor).expect("spot");
        assert_eq!(RecordingSpotOrderSubmitPort::call_count(), 1);
        RecordingSpotOrderSubmitPort::clear();
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    });
}

#[test]
fn paper_ledger_records_fill_after_risk() {
    PaperLedgerExecutor::clear_ledger();
    submit_order(sample_request(), sample_limits(), &PaperLedgerExecutor).expect("paper");
    let fills = PaperLedgerExecutor::recorded_fills();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].side, OrderSide::Buy);
    PaperLedgerExecutor::clear_ledger();
}

#[test]
fn reserved_live_exchange_port_returns_not_wired_after_risk() {
    let err = submit_order(
        sample_request(),
        sample_limits(),
        &ReservedLiveExchangeExecutor,
    )
    .unwrap_err();
    assert_eq!(err, OrdersError::LiveExchangeNotWired);
}

#[test]
fn exchange_spot_gate_blocks_testnet_submit_without_credentials() {
    with_env_test_lock(|| {
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
        std::env::remove_var("BINANCE_TESTNET_API_KEY");
        std::env::remove_var("BINANCE_TESTNET_SECRET");
        let err =
            submit_order(sample_request(), sample_limits(), &ExchangeSpotExecutor).unwrap_err();
        assert_eq!(err, OrdersError::LiveExchangeNotWired);
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    });
}

#[test]
fn reconciliation_pending_to_reconciled() {
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger
        .mark_pending("cid-1", "BTC/USDT", OrderSide::Buy)
        .expect("pending");
    assert_eq!(ledger.pending_count(), 1);
    let state = ledger
        .confirm_exchange_order("cid-1", "ex-99")
        .expect("reconciled");
    assert_eq!(
        state,
        ReconciliationState::Reconciled {
            exchange_order_id: "ex-99".into()
        }
    );
    assert_eq!(ledger.pending_count(), 0);
}

#[test]
fn reconciliation_seed_entry_restores_pending_count() {
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger.seed_entry("seed-cid", ReconciliationState::Pending);
    assert_eq!(ledger.pending_count(), 1);
    assert_eq!(ledger.state("seed-cid"), Some(ReconciliationState::Pending));
}

#[test]
fn reconciliation_seed_hydrated_row_preserves_symbol_for_poll() {
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger.seed_hydrated_row(
        "hydrate-cid",
        "ETH/USDT",
        OrderSide::Sell,
        ReconciliationState::Pending,
    );
    let pending = ledger.list_pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].client_order_id, "hydrate-cid");
    assert_eq!(pending[0].symbol, "ETH/USDT");
    assert_eq!(pending[0].side, OrderSide::Sell);
}

#[test]
fn reconciliation_poll_confirms_pending_when_recording_binding_exists() {
    use crate::modules::orders::adapters::clear_recording_client_bindings;
    use crate::modules::orders::{
        run_reconciliation_poll_once, RecordingSpotOrderReconciliationQuery,
    };

    clear_recording_client_bindings();
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger
        .mark_pending("poll-cid", "BTC/USDT", OrderSide::Buy)
        .expect("pending");
    crate::modules::orders::recording_bind_client_exchange("poll-cid", "recording-42");
    let summary = run_reconciliation_poll_once(&ledger, &RecordingSpotOrderReconciliationQuery)
        .expect("poll");
    assert_eq!(summary.confirmed, 1);
    assert_eq!(summary.unchanged, 0);
    assert_eq!(
        ledger.state("poll-cid"),
        Some(ReconciliationState::Reconciled {
            exchange_order_id: "recording-42".into()
        })
    );
    clear_recording_client_bindings();
}

#[test]
fn reconciliation_mark_divergent_from_pending() {
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger
        .mark_pending("cid-2", "ETH/USDT", OrderSide::Sell)
        .expect("pending");
    ledger
        .mark_divergent("cid-2", "exchange status canceled")
        .expect("divergent");
    assert_eq!(
        ledger.state("cid-2"),
        Some(ReconciliationState::Divergent {
            reason: "exchange status canceled".into()
        })
    );
}

#[derive(Debug, Clone, Copy)]
struct DivergentPollQuery;

impl crate::modules::orders::adapters::SpotOrderReconciliationQuery for DivergentPollQuery {
    fn observe(
        &self,
        _client_order_id: &str,
        _symbol: &str,
        _side: OrderSide,
    ) -> Result<crate::modules::orders::adapters::RemoteOrderObservation, OrdersError> {
        Ok(
            crate::modules::orders::adapters::RemoteOrderObservation::Divergent {
                reason: "exchange rejected".into(),
            },
        )
    }
}

#[test]
fn reconciliation_poll_marks_divergent_when_query_reports_divergent() {
    use crate::modules::orders::run_reconciliation_poll_once;
    let ledger = InMemoryOrderReconciliationLedger::new();
    ledger
        .mark_pending("div-cid", "BTC/USDT", OrderSide::Buy)
        .expect("pending");
    let summary = run_reconciliation_poll_once(&ledger, &DivergentPollQuery).expect("poll");
    assert_eq!(summary.divergent, 1);
    assert_eq!(summary.confirmed, 0);
    assert_eq!(
        ledger.state("div-cid"),
        Some(ReconciliationState::Divergent {
            reason: "exchange rejected".into(),
        })
    );
}
