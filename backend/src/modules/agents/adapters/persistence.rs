use crate::modules::agents::models::{AgentDefinition, IdentityAuditEvent};

/// Durable agent identity rows (PostgreSQL Gate 1 scaffold).
#[async_trait::async_trait]
pub trait AgentIdentityStore {
    #[allow(dead_code)] // production write uses `persist_identity_and_enqueue_graph_projection`
    async fn upsert_agent(&self, definition: &AgentDefinition) -> Result<(), String>;
    #[allow(dead_code)]
    async fn append_event(&self, event: &IdentityAuditEvent) -> Result<(), String>;
    async fn load_snapshot(
        &self,
    ) -> Result<(Vec<AgentDefinition>, Vec<IdentityAuditEvent>), String>;
}
