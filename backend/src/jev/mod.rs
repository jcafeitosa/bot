use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    config::JevConfig,
    error::{BotError, BotResult},
    strategy::StrategySnapshot,
};

#[derive(Debug, Clone)]
pub struct JevAdvisor {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    config: JevConfig,
}

#[derive(Debug, Deserialize)]
pub struct JevResponse {
    pub model: Option<String>,
    pub answers: std::collections::HashMap<String, Value>,
}

impl JevAdvisor {
    pub fn from_env(config: JevConfig) -> BotResult<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }
        let api_key = std::env::var("TYPESAFE_API_KEY").map_err(|_| {
            BotError::Configuration(
                "jev.enabled=true requires TYPESAFE_API_KEY in the process environment".into(),
            )
        })?;
        let endpoint = std::env::var("TYPESAFE_ENDPOINT")
            .unwrap_or_else(|_| "https://api.typesafe.ai/v1/systemone".into());
        let parsed = reqwest::Url::parse(&endpoint)
            .map_err(|_| BotError::Configuration("TYPESAFE_ENDPOINT must be a valid URL".into()))?;
        let local_http = parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if parsed.scheme() != "https" && !local_http {
            return Err(BotError::Configuration(
                "TypeSafe endpoint must use HTTPS (HTTP permitted only for localhost)".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.clamp(1, 30)))
            .build()
            .map_err(|e| BotError::Jev(e.to_string()))?;
        Ok(Some(Self {
            client,
            endpoint,
            api_key,
            config,
        }))
    }

    pub async fn review(&self, snapshot: &StrategySnapshot) -> BotResult<Vec<String>> {
        let mut questions = serde_json::Map::new();
        if self.config.market_regime {
            questions.insert("market_regime".into(), json!({"type":"choice","instructions":"Classify the market regime from the provided recent candle and indicators. This is advisory only.","criteria":{"trending":"Directional movement is evident","ranging":"Price is oscillating without direction","volatile":"Large or erratic price movement","unclear":"Insufficient evidence"}}));
        }
        if self.config.signal_review {
            questions.insert("signal_quality".into(), json!({"type":"score","instructions":"Rate how coherent the SMA crossover signal and available market snapshot appear. This is not permission to place an order.","criteria":["Weak or inconsistent","Mixed","Reasonably coherent","Strongly coherent"]}));
        }
        if self.config.ops_triage {
            questions.insert("market_anomaly".into(), json!({"type":"noul","instructions":"Does this market snapshot contain an apparent data anomaly or condition that warrants operator review? This is advisory only."}));
        }
        if questions.is_empty() {
            return Ok(vec![
                "Jev enabled but no evaluation use case selected".into()
            ]);
        }
        let state = json!({
            "symbol": "configured market",
            "timeframe": "configured interval",
            "candle_timestamp_ms": snapshot.candle_timestamp_ms,
            "close": snapshot.close,
            "fast_sma": snapshot.fast_sma,
            "slow_sma": snapshot.slow_sma,
            "signal": format!("{:?}", snapshot.signal),
            "notice": "No API credentials, balances, account identifiers, or private order data are included."
        });
        let body = json!({"state":state,"model":"jev-latest","questions":Value::Object(questions)});
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
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
        let parsed: JevResponse = response
            .json()
            .await
            .map_err(|e| BotError::Jev(format!("invalid TypeSafe response: {e}")))?;
        if let Some(model) = &parsed.model {
            tracing::debug!(target: "jev", model = %model, "TypeSafe response model");
        }
        Ok(parsed
            .answers
            .into_iter()
            .map(|(key, value)| format!("{key}: {}", value))
            .collect())
    }
}
