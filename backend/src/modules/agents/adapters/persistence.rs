use crate::modules::agents::models::{AgentDefinition, IdentityAuditEvent};

/// Durable agent identity rows (PostgreSQL Gate 1 scaffold).
#[async_trait::async_trait]
pub trait AgentIdentityStore {
    async fn upsert_agent(&self, definition: &AgentDefinition) -> Result<(), String>;
    async fn append_event(&self, event: &IdentityAuditEvent) -> Result<(), String>;
}
