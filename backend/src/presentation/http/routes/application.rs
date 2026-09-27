use axum::Json;

use crate::modules::http_bridge::application::{self, SignalsResponse};

#[utoipa::path(
    get,
    path = "/api/v1/application/signals",
    tag = "application",
    responses((status = 200, description = "Trading signal enum", body = SignalsResponse))
)]
pub async fn list_signals() -> Json<SignalsResponse> {
    Json(application::list_trading_signals())
}
