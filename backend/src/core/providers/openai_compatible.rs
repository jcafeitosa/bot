#![allow(dead_code)] // Public OpenAI-compatible seam (9router/proxy); relative paths not used by Jev advisory yet.

use std::time::Duration;

use reqwest::{Client, Url};
use serde_json::Value;

use crate::core::error::{BotError, BotResult};

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl OpenAiCompatibleClient {
    pub fn new(config: OpenAiCompatibleConfig) -> BotResult<Self> {
        validate_https_or_localhost(&config.base_url)?;
        let client = build_http_client(config.timeout_seconds)?;
        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key,
        })
    }

    pub fn from_base_url(
        base_url: String,
        api_key: String,
        timeout_seconds: u64,
    ) -> BotResult<Self> {
        Self::new(OpenAiCompatibleConfig {
            base_url,
            api_key,
            timeout_seconds,
        })
    }

    pub async fn post_json(&self, path: &str, body: Value) -> BotResult<Value> {
        let path = path.trim_start_matches('/');
        let url = format!("{}/{}", self.base_url, path);
        self.post_absolute(&url, body).await
    }

    pub async fn chat_completions(&self, body: Value) -> BotResult<Value> {
        self.post_json("v1/chat/completions", body).await
    }

    pub async fn post_absolute(&self, url: &str, body: Value) -> BotResult<Value> {
        validate_https_or_localhost(url)?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| BotError::Jev(e.to_string()))?;
        if !response.status().is_success() {
            return Err(BotError::Jev(format!(
                "provider HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|e| BotError::Jev(format!("invalid provider JSON response: {e}")))
    }
}

pub fn build_http_client(timeout_seconds: u64) -> BotResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_seconds.clamp(1, 30)))
        .build()
        .map_err(|e| BotError::Jev(e.to_string()))
}

pub fn validate_https_or_localhost(endpoint: &str) -> BotResult<()> {
    let parsed = Url::parse(endpoint)
        .map_err(|_| BotError::Configuration("provider endpoint must be a valid URL".into()))?;
    let local_http = parsed.scheme() == "http"
        && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if parsed.scheme() != "https" && !local_http {
        return Err(BotError::Configuration(
            "provider endpoint must use HTTPS (HTTP permitted only for localhost)".into(),
        ));
    }
    Ok(())
}

pub fn resolve_bearer_api_key() -> BotResult<String> {
    crate::core::config::providers::resolve_bearer_api_key()
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[test]
    fn rejects_non_local_http_endpoint() {
        let err = validate_https_or_localhost("http://example.com/v1")
            .unwrap_err()
            .to_string();
        assert!(err.contains("HTTPS"));
    }

    #[test]
    fn accepts_localhost_http() {
        assert!(validate_https_or_localhost("http://127.0.0.1:8080/advisory").is_ok());
    }

    #[tokio::test]
    async fn post_absolute_sends_bearer_and_json() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/systemone")
                    .header("authorization", "Bearer test-key");
                then.status(200).json_body(json!({"ok": true}));
            })
            .await;

        let client = OpenAiCompatibleClient::from_base_url(server.base_url(), "test-key".into(), 5)
            .expect("client");
        let body = json!({"ping": 1});
        let parsed = client
            .post_absolute(&format!("{}/v1/systemone", server.base_url()), body)
            .await
            .expect("response");
        assert_eq!(parsed["ok"], true);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn chat_completions_posts_to_v1() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/chat/completions");
                then.status(200).json_body(json!({"id": "cmpl-test"}));
            })
            .await;

        let client =
            OpenAiCompatibleClient::from_base_url(server.base_url(), "key".into(), 5).unwrap();
        let out = client
            .chat_completions(json!({"model": "gpt-4"}))
            .await
            .unwrap();
        assert_eq!(out["id"], "cmpl-test");
        mock.assert_async().await;
    }
}
