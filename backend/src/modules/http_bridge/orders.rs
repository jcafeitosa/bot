use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::http_bridge::risk::RiskLimitsBody;
use crate::modules::orders::{
    submit_order, OrderExecutionPort, OrderIdempotencyStore, OrderReconciliationLedger, OrderSide,
    OrdersError, SubmitOrderRequest,
};
use crate::modules::risk::models::RiskLimits;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSideBody {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SubmitOrderHttpRequest {
    pub symbol: String,
    pub side: OrderSideBody,
    pub quote_amount: f64,
    pub estimated_daily_loss: f64,
    pub open_positions: usize,
    pub limits: RiskLimitsBody,
    /// Optional replay key; duplicate completed keys return `accepted: true` without re-executing.
    #[serde(default)]
    pub client_order_id: Option<String>,
    /// Paper mode: quote per base unit for portfolio `positions` (optional).
    #[serde(default)]
    pub paper_fill_unit_price: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitOrderResponse {
    pub accepted: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderReconciliationResponse {
    pub client_order_id: String,
    /// `pending`, `reconciled`, or `divergent`.
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderReconciliationPollResponse {
    pub confirmed: usize,
    pub divergent: usize,
    pub unchanged: usize,
}

pub fn order_reconciliation_poll_response(
    summary: &crate::modules::orders::ReconciliationPollSummary,
) -> OrderReconciliationPollResponse {
    OrderReconciliationPollResponse {
        confirmed: summary.confirmed,
        divergent: summary.divergent,
        unchanged: summary.unchanged,
    }
}

pub fn order_reconciliation_response(
    client_order_id: &str,
    state: &crate::modules::orders::ReconciliationState,
) -> OrderReconciliationResponse {
    use crate::modules::orders::ReconciliationState;
    match state {
        ReconciliationState::Pending => OrderReconciliationResponse {
            client_order_id: client_order_id.to_string(),
            state: "pending".into(),
            exchange_order_id: None,
            reason: None,
        },
        ReconciliationState::Reconciled { exchange_order_id } => OrderReconciliationResponse {
            client_order_id: client_order_id.to_string(),
            state: "reconciled".into(),
            exchange_order_id: Some(exchange_order_id.clone()),
            reason: None,
        },
        ReconciliationState::Divergent { reason } => OrderReconciliationResponse {
            client_order_id: client_order_id.to_string(),
            state: "divergent".into(),
            exchange_order_id: None,
            reason: Some(reason.clone()),
        },
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderExecutionStatusResponse {
    /// Resolved from `BOT_ORDERS_EXECUTION` at process start (`disabled`, `dev_accept`, `paper`, `live_exchange_reserved`).
    pub mode: String,
    /// `true` only when a real exchange adapter is wired (always `false` in this build).
    pub live_exchange_wired: bool,
}

pub fn order_execution_status(
    mode_label: &str,
    live_exchange_wired: bool,
) -> OrderExecutionStatusResponse {
    OrderExecutionStatusResponse {
        mode: mode_label.to_string(),
        live_exchange_wired,
    }
}

pub(crate) fn validate_client_order_id(key: Option<&str>) -> Result<(), OrdersError> {
    match key {
        None => Ok(()),
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed.len() > 128 {
                return Err(OrdersError::InvalidRequest(
                    "client_order_id must be 1..=128 bytes when set".into(),
                ));
            }
            Ok(())
        }
    }
}

pub fn submit_order_http<E: OrderExecutionPort, I: OrderIdempotencyStore>(
    body: SubmitOrderHttpRequest,
    executor: &E,
    idempotency: &I,
    reconciliation: Option<&dyn OrderReconciliationLedger>,
) -> Result<SubmitOrderResponse, OrdersError> {
    let SubmitOrderHttpRequest {
        symbol,
        side,
        quote_amount,
        estimated_daily_loss,
        open_positions,
        limits: limits_body,
        client_order_id,
        paper_fill_unit_price,
    } = body;
    validate_client_order_id(client_order_id.as_deref())?;
    if let Some(key) = client_order_id.as_deref().map(str::trim) {
        if idempotency.is_completed(key) {
            return Ok(SubmitOrderResponse { accepted: true });
        }
    }
    let limits: RiskLimits = limits_body.into();
    let order_side = match side {
        OrderSideBody::Buy => OrderSide::Buy,
        OrderSideBody::Sell => OrderSide::Sell,
    };
    let request = SubmitOrderRequest {
        symbol: &symbol,
        side: order_side,
        quote_amount,
        estimated_daily_loss,
        open_positions,
        paper_fill_unit_price,
        client_order_id: client_order_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    };
    submit_order(request, limits, executor)?;
    if let Some(key) = client_order_id.as_deref().map(str::trim) {
        idempotency.record_completed(key);
        if let Some(ledger) = reconciliation {
            ledger.mark_pending(key, &symbol, order_side)?;
        }
    }
    Ok(SubmitOrderResponse { accepted: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orders::{
        AcceptingExecutor, InMemoryOrderIdempotencyStore, InMemoryOrderReconciliationLedger,
        OrderReconciliationLedger, ReconciliationState, RecordingExecutor,
    };

    fn sample_body(client_order_id: Option<String>) -> SubmitOrderHttpRequest {
        SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 1.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id,
            paper_fill_unit_price: None,
        }
    }

    #[test]
    fn submit_order_http_forwards_client_order_id_to_executor_request() {
        let store = InMemoryOrderIdempotencyStore::new();
        let executor = RecordingExecutor::new();
        let body = sample_body(Some("bridge-cid-1".into()));
        submit_order_http(body, &executor, &store, None).expect("accepted");
        assert_eq!(
            executor.last_client_order_id().as_deref(),
            Some("bridge-cid-1")
        );
    }

    #[test]
    fn order_execution_status_reports_mode_and_not_wired() {
        let status = order_execution_status("disabled", false);
        assert_eq!(status.mode, "disabled");
        assert!(!status.live_exchange_wired);
    }

    #[test]
    fn submit_order_http_records_execution_with_recording_executor() {
        let store = InMemoryOrderIdempotencyStore::new();
        let executor = RecordingExecutor::new();
        submit_order_http(sample_body(None), &executor, &store, None).expect("accepted");
        assert_eq!(executor.call_count(), 1);
    }

    #[test]
    fn paper_fill_unit_price_flows_to_executor_request() {
        use crate::modules::orders::PaperLedgerExecutor;

        PaperLedgerExecutor::clear_ledger();
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 100.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 200.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: None,
            paper_fill_unit_price: Some(50_000.0),
        };
        submit_order_http(
            body,
            &PaperLedgerExecutor,
            &InMemoryOrderIdempotencyStore::new(),
            None,
        )
        .expect("paper submit");
        let fill = PaperLedgerExecutor::recorded_fills()
            .into_iter()
            .next()
            .expect("fill");
        assert_eq!(fill.fill_unit_price, Some(50_000.0));
        PaperLedgerExecutor::clear_ledger();
    }

    #[test]
    fn duplicate_client_order_id_replays_without_second_execute() {
        let store = InMemoryOrderIdempotencyStore::new();
        let executor = RecordingExecutor::new();
        let body = sample_body(Some("order-abc".into()));
        submit_order_http(body.clone(), &executor, &store, None).expect("first");
        submit_order_http(body, &executor, &store, None).expect("replay");
        assert_eq!(executor.call_count(), 1);
    }

    #[test]
    fn submit_order_http_marks_reconciliation_pending_for_client_order_id() {
        let store = InMemoryOrderIdempotencyStore::new();
        let reconciliation = InMemoryOrderReconciliationLedger::new();
        let body = sample_body(Some("recon-cid-1".into()));
        submit_order_http(body, &AcceptingExecutor, &store, Some(&reconciliation))
            .expect("accepted");
        assert_eq!(reconciliation.pending_count(), 1);
        assert_eq!(
            reconciliation.state("recon-cid-1"),
            Some(ReconciliationState::Pending)
        );
    }

    #[test]
    fn invalid_client_order_id_rejected() {
        let store = InMemoryOrderIdempotencyStore::new();
        let err = submit_order_http(
            sample_body(Some("".into())),
            &AcceptingExecutor,
            &store,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, OrdersError::InvalidRequest(_)));
    }
}
