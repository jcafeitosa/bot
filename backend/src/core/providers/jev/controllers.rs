use serde_json::{json, Value};

use crate::{
    core::config::JevConfig,
    core::error::BotResult,
    core::providers::jev::adapters::typesafe::post_review,
    core::providers::jev::models::JevReviewInput,
    core::providers::openai_compatible::{self, OpenAiCompatibleClient},
};

#[derive(Debug, Clone)]
pub struct JevAdvisor {
    client: OpenAiCompatibleClient,
    endpoint: String,
    config: JevConfig,
}

impl JevAdvisor {
    pub fn from_env(config: JevConfig) -> BotResult<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }
        let api_key = openai_compatible::resolve_bearer_api_key()?;
        let endpoint = crate::core::config::providers::typesafe_endpoint();
        openai_compatible::validate_https_or_localhost(&endpoint)?;
        let client = OpenAiCompatibleClient::from_base_url(
            endpoint.clone(),
            api_key,
            config.timeout_seconds,
        )?;
        Ok(Some(Self {
            client,
            endpoint,
            config,
        }))
    }

    pub async fn review(&self, input: &JevReviewInput) -> BotResult<Vec<String>> {
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
            "candle_timestamp_ms": input.candle_timestamp_ms,
            "close": input.close,
            "fast_sma": input.fast_sma,
            "slow_sma": input.slow_sma,
            "signal": input.signal_label,
            "notice": "No API credentials, balances, account identifiers, or private order data are included."
        });
        let body = json!({"state":state,"model":crate::core::config::providers::typesafe_model(),"questions":Value::Object(questions)});
        let parsed = post_review(&self.client, &self.endpoint, body).await?;
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
