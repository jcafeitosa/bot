use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JevResponse {
    pub model: Option<String>,
    pub answers: std::collections::HashMap<String, Value>,
}
