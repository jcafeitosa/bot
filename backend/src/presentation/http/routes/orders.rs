use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::modules::http_bridge::orders::{
    order_execution_status, order_reconciliation_poll_response, order_reconciliation_response,
    OrderExecutionStatusResponse, OrderReconciliationPollResponse, OrderReconciliationResponse,
    SubmitOrderHttpRequest, SubmitOrderResponse,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[utoipa::path(
    get,
    path = "/api/v1/orders/execution-status",
    tag = "orders",
    responses(
        (status = 200, description = "Active HTTP order execution seam (read-only)", body = OrderExecutionStatusResponse)
    )
)]
pub async fn execution_status(State(state): State<ApiState>) -> Json<OrderExecutionStatusResponse> {
    let label = state.order_execution_mode().as_api_label();
    Json(order_execution_status(label, state.live_exchange_wired()))
}

#[utoipa::path(
    get,
    path = "/api/v1/orders/reconciliation/{client_order_id}",
    tag = "orders",
    params(
        ("client_order_id" = String, Path, description = "Client order id from POST /orders/submit")
    ),
    responses(
        (status = 200, description = "Reconciliation state when tracked", body = OrderReconciliationResponse),
        (status = 404, description = "Unknown client_order_id", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn reconciliation_status(
    State(state): State<ApiState>,
    Path(client_order_id): Path<String>,
) -> Result<Json<OrderReconciliationResponse>, ApiError> {
    let rec = state
        .order_reconciliation_lookup(&client_order_id)
        .await
        .map_err(ApiError::from_orders_error)?;
    match rec {
        Some(state) => Ok(Json(order_reconciliation_response(
            &client_order_id,
            &state,
        ))),
        None => Err(ApiError::with_code(
            StatusCode::NOT_FOUND,
            "reconciliation_not_found",
            "no reconciliation state for client_order_id",
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/reconciliation/poll",
    tag = "orders",
    responses(
        (status = 200, description = "One reconciliation poll pass over pending client_order_id rows", body = OrderReconciliationPollResponse),
        (status = 401, description = "Admin bearer required when configured", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn reconciliation_poll(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<OrderReconciliationPollResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    let summary = state
        .reconcile_pending_orders_once()
        .await
        .map_err(ApiError::from_orders_error)?;
    Ok(Json(order_reconciliation_poll_response(&summary)))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/submit",
    tag = "orders",
    request_body = SubmitOrderHttpRequest,
    responses(
        (status = 200, description = "Order accepted by risk gate (execution may still be disabled)", body = SubmitOrderResponse),
        (status = 400, description = "Invalid request", body = crate::presentation::http::error::ApiErrorBody),
        (status = 422, description = "Risk rejected", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "Execution disabled (fail-closed)", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "Order durable store unavailable (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn submit_order(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<SubmitOrderHttpRequest>,
) -> Result<Json<SubmitOrderResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state
        .submit_order_http(body)
        .await
        .map_err(ApiError::from_orders_error)
        .map(Json)
}
