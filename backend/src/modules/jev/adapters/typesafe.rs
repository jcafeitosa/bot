use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::core::error::{BotError, BotResult};
use crate::modules::jev::models::JevResponse;

pub async fn post_review(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    body: Value,
) -> BotResult<JevResponse> {
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| BotError::Jev(e.to_string()))?;
    if !response.status().is_success() {
        return Err(BotError::Jev(format!(
            "TypeSafe API returned HTTP {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|e| BotError::Jev(format!("invalid TypeSafe response: {e}")))
}

pub fn build_http_client(timeout_seconds: u64) -> BotResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_seconds.clamp(1, 30)))
        .build()
        .map_err(|e| BotError::Jev(e.to_string()))
}
