#![allow(dead_code)] // Public NVIDIA NIM seam; consumers wire via agents or config.
//! [NVIDIA NIM](https://docs.api.nvidia.com/nim/reference/models-1) — integrate API OpenAI-compatible.
//!
//! O catálogo completo de modelos é dinâmico na API NVIDIA; use `list_models_hint()` apenas como
//! exemplos curados. Consulte a documentação oficial para IDs atuais por categoria.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::config::ProviderConfig;
use crate::core::error::BotResult;
use crate::core::providers::openai_compatible::{
    validate_https_or_localhost, OpenAiCompatibleClient,
};

/// Raiz típica da integrate API (sem `/v1`; o cliente acrescenta `v1/chat/completions`).
pub const DEFAULT_NIM_BASE_URL: &str = "https://integrate.api.nvidia.com";

/// Categorias alinhadas à referência NVIDIA NIM (`models-1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NimModelCategory {
    Llm,
    Retrieval,
    Visual,
    Multimodal,
    Healthcare,
    RouteOptimization,
    ClimateSimulation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NimModelRef {
    pub model_id: String,
    pub category: NimModelCategory,
}

impl NimModelRef {
    pub fn new(model_id: impl Into<String>, category: NimModelCategory) -> Self {
        Self {
            model_id: model_id.into(),
            category,
        }
    }
}

/// Exemplos configuráveis — não substituem listagem dinâmica da NVIDIA.
pub fn list_models_hint() -> Vec<NimModelRef> {
    vec![
        NimModelRef::new("meta/llama-3.1-70b-instruct", NimModelCategory::Llm),
        NimModelRef::new("meta/llama-3.1-8b-instruct", NimModelCategory::Llm),
        NimModelRef::new("nvidia/nemotron-mini-4b-instruct", NimModelCategory::Llm),
        NimModelRef::new("nvidia/nv-embedqa-e5-v5", NimModelCategory::Retrieval),
        NimModelRef::new("microsoft/kosmos-2", NimModelCategory::Visual),
        NimModelRef::new("google/paligemma", NimModelCategory::Multimodal),
        NimModelRef::new("nvidia/clara-train", NimModelCategory::Healthcare),
        NimModelRef::new("nvidia/cuopt", NimModelCategory::RouteOptimization),
        NimModelRef::new("nvidia/fourcastnet", NimModelCategory::ClimateSimulation),
    ]
}

pub fn nim_base_url_from_env() -> Option<String> {
    crate::core::config::providers::nim_base_url_from_env()
}

/// Normaliza base URL: remove barras finais e um sufixo `/v1` duplicado em relação ao cliente OpenAI-compatible.
pub fn normalize_nim_base_url(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if let Some(stripped) = trimmed.strip_suffix("/v1") {
        stripped.trim_end_matches('/').to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn resolve_nim_base_url(toml: &ProviderConfig) -> String {
    nim_base_url_from_env()
        .or_else(|| toml.nim_base_url.clone())
        .map(|b| normalize_nim_base_url(&b))
        .unwrap_or_else(|| DEFAULT_NIM_BASE_URL.to_string())
}

pub fn resolve_nvidia_api_key() -> BotResult<String> {
    crate::core::config::providers::resolve_nvidia_api_key()
}

#[derive(Debug, Clone)]
pub struct NvidiaNimClient {
    inner: OpenAiCompatibleClient,
}

impl NvidiaNimClient {
    pub fn new(base_url: String, api_key: String, timeout_seconds: u64) -> BotResult<Self> {
        let base_url = normalize_nim_base_url(&base_url);
        validate_https_or_localhost(&base_url)?;
        let inner = OpenAiCompatibleClient::from_base_url(base_url, api_key, timeout_seconds)?;
        Ok(Self { inner })
    }

    pub fn from_config(toml: &ProviderConfig, timeout_seconds: u64) -> BotResult<Self> {
        let api_key = resolve_nvidia_api_key()?;
        Self::new(resolve_nim_base_url(toml), api_key, timeout_seconds)
    }

    pub async fn chat_completions(&self, model: &str, messages: Value) -> BotResult<Value> {
        let body = json!({
            "model": model,
            "messages": messages,
        });
        self.inner.chat_completions(body).await
    }
}

/// Seam opcional para futuros “agent brains”; implementação padrão delega a [`NvidiaNimClient`].
#[async_trait]
pub trait NimLlmProvider: Send + Sync {
    async fn chat_completions(&self, model: &str, messages: Value) -> BotResult<Value>;
}

#[async_trait]
impl NimLlmProvider for NvidiaNimClient {
    async fn chat_completions(&self, model: &str, messages: Value) -> BotResult<Value> {
        NvidiaNimClient::chat_completions(self, model, messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[test]
    fn normalizes_trailing_v1_on_base_url() {
        assert_eq!(
            normalize_nim_base_url("https://integrate.api.nvidia.com/v1/"),
            "https://integrate.api.nvidia.com"
        );
    }

    #[test]
    fn rejects_non_local_http_nim_endpoint() {
        let err = NvidiaNimClient::new("http://example.com".into(), "key".into(), 5)
            .unwrap_err()
            .to_string();
        assert!(err.contains("HTTPS"));
    }

    #[tokio::test]
    async fn chat_completions_posts_openai_shape() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/chat/completions")
                    .header("authorization", "Bearer nim-key");
                then.status(200).json_body(json!({
                    "choices": [{"message": {"content": "ok"}}]
                }));
            })
            .await;

        let client = NvidiaNimClient::new(server.base_url(), "nim-key".into(), 5).unwrap();
        let messages = json!([{"role": "user", "content": "hello"}]);
        let out = client
            .chat_completions("meta/llama-3.1-8b-instruct", messages)
            .await
            .unwrap();
        assert_eq!(out["choices"][0]["message"]["content"], "ok");
        mock.assert_async().await;
    }

    #[test]
    fn env_precedes_toml_for_nim_base_url() {
        std::env::set_var("NVIDIA_NIM_BASE_URL", "https://env.nvidia.example/v1");
        let resolved = resolve_nim_base_url(&ProviderConfig {
            openai_base_url: None,
            nim_base_url: Some("https://toml.nvidia.example".into()),
        });
        std::env::remove_var("NVIDIA_NIM_BASE_URL");
        assert_eq!(resolved, "https://env.nvidia.example");
    }
}
