use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error::BotsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BotPromotionState {
    Active,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BotPromotionRecord {
    pub bot_id: String,
    pub promoted_by: String,
    pub promoted_at_unix_ms: u64,
    pub state: BotPromotionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BotRuntimeStatus {
    pub runtime_enabled: bool,
    pub active: Option<BotPromotionRecord>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PromoteBotRequest {
    pub bot_id: String,
    pub promoted_by: String,
}

impl PromoteBotRequest {
    pub fn validate(&self) -> Result<(), BotsError> {
        if self.bot_id.trim().is_empty() || self.bot_id.len() > 200 {
            return Err(BotsError::InvalidId("bot_id must be 1..=200 bytes".into()));
        }
        if self.promoted_by.trim().is_empty() || self.promoted_by.len() > 120 {
            return Err(BotsError::InvalidId(
                "promoted_by must be 1..=120 bytes".into(),
            ));
        }
        Ok(())
    }
}
