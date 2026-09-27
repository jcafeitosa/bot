//! Neutral application types shared across modules (no domain logic).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Signal {
    Warmup,
    Hold,
    Buy,
    Sell,
}

pub fn signal_label(signal: Signal) -> &'static str {
    match signal {
        Signal::Warmup => "warmup",
        Signal::Hold => "hold",
        Signal::Buy => "buy",
        Signal::Sell => "sell",
    }
}

/// Optional simulation/backtest label on a signal — not `agents::AgentId` or future `modules/bots`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotSignal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>,
    pub signal: Signal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_round_trips_json() {
        let signal = Signal::Hold;
        let encoded = serde_json::to_string(&signal).unwrap();
        let decoded: Signal = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, Signal::Hold);
    }

    #[test]
    fn signal_label_maps_buy() {
        assert_eq!(signal_label(Signal::Buy), "buy");
    }

    #[test]
    fn bot_signal_serializes_optional_bot_id() {
        let with_id = BotSignal {
            bot_id: Some("alpha".into()),
            signal: Signal::Buy,
        };
        let without_id = BotSignal {
            bot_id: None,
            signal: Signal::Sell,
        };
        assert!(serde_json::to_string(&with_id).unwrap().contains("alpha"));
        assert!(!serde_json::to_string(&without_id)
            .unwrap()
            .contains("bot_id"));
    }
}
