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

pub fn list_trading_signals() -> SignalsResponse {
    let signals = [Signal::Warmup, Signal::Hold, Signal::Buy, Signal::Sell]
        .into_iter()
        .map(|signal| SignalDescriptor {
            label: signal_label(signal).to_owned(),
            signal,
        })
        .collect();
    SignalsResponse { signals }
}
