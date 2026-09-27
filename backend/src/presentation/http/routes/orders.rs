use axum::Json;

use crate::modules::http_bridge::orders::{self, SubmitOrderHttpRequest, SubmitOrderResponse};
use crate::presentation::http::error::ApiError;

#[utoipa::path(
    post,
    path = "/api/v1/orders/submit",
    tag = "orders",
    request_body = SubmitOrderHttpRequest,
    responses(
        (status = 200, description = "Order accepted by risk gate (execution may still be disabled)", body = SubmitOrderResponse),
        (status = 400, description = "Invalid request", body = crate::presentation::http::error::ApiErrorBody),
        (status = 422, description = "Risk rejected", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "Execution disabled (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn submit_order(
    Json(body): Json<SubmitOrderHttpRequest>,
) -> Result<Json<SubmitOrderResponse>, ApiError> {
    orders::submit_order_http(body)
        .map_err(ApiError::from_orders_error)
        .map(Json)
}
