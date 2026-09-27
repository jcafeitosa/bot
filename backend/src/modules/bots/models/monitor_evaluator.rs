use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Which indicator crossover the monitor supervisor runs for a registered strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MonitorEvaluatorKind {
    #[default]
    SmaCross,
    EmaCross,
}

impl MonitorEvaluatorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SmaCross => "sma_cross",
            Self::EmaCross => "ema_cross",
        }
    }
}
