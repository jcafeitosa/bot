use axum::{extract::State, Json};

use crate::modules::http_bridge::providers::{self, ProvidersStatusResponse};
use crate::presentation::http::state::ApiState;

#[utoipa::path(
    get,
    path = "/api/v1/providers/status",
    tag = "providers",
    responses((status = 200, description = "Provider flags from loaded config", body = ProvidersStatusResponse))
)]
pub async fn provider_status(State(state): State<ApiState>) -> Json<ProvidersStatusResponse> {
    Json(providers::status_from_config(state.app_config()))
}
