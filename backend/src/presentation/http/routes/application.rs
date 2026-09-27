use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::modules::application_contracts::{signal_label, Signal};

#[derive(Debug, Serialize, ToSchema)]
pub struct SignalDescriptor {
    pub signal: Signal,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignalsResponse {
    pub signals: Vec<SignalDescriptor>,
}

#[utoipa::path(
    get,
    path = "/api/v1/application/signals",
    tag = "application",
    responses((status = 200, description = "Trading signal enum", body = SignalsResponse))
)]
pub async fn list_signals() -> Json<SignalsResponse> {
    let signals = [Signal::Warmup, Signal::Hold, Signal::Buy, Signal::Sell]
        .into_iter()
        .map(|signal| SignalDescriptor {
            label: signal_label(signal).to_owned(),
            signal,
        })
        .collect();
    Json(SignalsResponse { signals })
}
