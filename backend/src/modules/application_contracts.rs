//! Neutral application types shared across modules (no domain logic).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Warmup,
    Hold,
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotSignal {
    pub bot_id: Option<String>,
    pub signal: Signal,
}
