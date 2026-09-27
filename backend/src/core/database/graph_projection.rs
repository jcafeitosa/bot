//! Domain-agnostic graph projection seams (Neo4j adapters live beside `neo4j.rs`).

use thiserror::Error;

/// Subgraph namespace for agent hierarchy projection (F1).
pub const AGENTS_GRAPH_DOMAIN: &str = "agents";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectedSupervisorKind {
    Owner,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHierarchyProjection {
    pub agency_id: String,
    pub agent_id: String,
    pub role: String,
    pub lifecycle: String,
    pub supervisor_kind: ProjectedSupervisorKind,
    pub supervisor_owner_id: Option<String>,
    pub supervisor_agent_id: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Error)]
pub enum GraphProjectionError {
    #[error("graph projection driver error: {0}")]
    Driver(String),
    #[error("invalid projection: {0}")]
    Invalid(String),
}

#[async_trait::async_trait]
pub trait GraphProjectionPort: Send + Sync {
    async fn project_agent_hierarchy(
        &self,
        projection: &AgentHierarchyProjection,
    ) -> Result<(), GraphProjectionError>;
}
