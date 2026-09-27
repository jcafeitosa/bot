use serde_json::Value;

use crate::core::error::{BotError, BotResult};
use crate::core::providers::jev::models::JevResponse;
use crate::core::providers::openai_compatible::OpenAiCompatibleClient;

pub async fn post_review(
    client: &OpenAiCompatibleClient,
    endpoint: &str,
    body: Value,
) -> BotResult<JevResponse> {
    let value = client.post_absolute(endpoint, body).await?;
    serde_json::from_value(value)
        .map_err(|e| BotError::Jev(format!("invalid TypeSafe response: {e}")))
}
