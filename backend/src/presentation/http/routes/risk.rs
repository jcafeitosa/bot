use axum::Json;

use crate::modules::http_bridge::risk::{
    self, GateSignalRequest, GateSignalResponse, ProfileLimitsRequest, ProfileLimitsResponse,
    ValidateIntentRequest, ValidateIntentResponse,
};
use crate::presentation::http::error::ApiError;

#[utoipa::path(
    post,
    path = "/api/v1/risk/profile-limits",
    tag = "risk",
    request_body = ProfileLimitsRequest,
    responses((status = 200, description = "Scaled limits", body = ProfileLimitsResponse))
)]
pub async fn compute_profile_limits(
    Json(body): Json<ProfileLimitsRequest>,
) -> Json<ProfileLimitsResponse> {
    Json(risk::compute_profile_limits(body))
}

#[utoipa::path(
    post,
    path = "/api/v1/risk/validate-intent",
    tag = "risk",
    request_body = ValidateIntentRequest,
    responses(
        (status = 200, description = "Intent accepted", body = ValidateIntentResponse),
        (status = 422, description = "Risk rejected", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn validate_order_intent(
    Json(body): Json<ValidateIntentRequest>,
) -> Result<Json<ValidateIntentResponse>, ApiError> {
    risk::validate_order_intent(body)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/risk/gate-signal",
    tag = "risk",
    request_body = GateSignalRequest,
    responses(
        (status = 200, description = "Signal allowed", body = GateSignalResponse),
        (status = 422, description = "Risk rejected", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn gate_signal(
    Json(body): Json<GateSignalRequest>,
) -> Result<Json<GateSignalResponse>, ApiError> {
    risk::gate_trading_signal(body)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}
