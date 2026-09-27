//! Domain-agnostic graph projection seams (Neo4j adapters live beside `neo4j.rs`).

use thiserror::Error;

/// Subgraph namespace for agent hierarchy projection (F1).
pub const AGENTS_GRAPH_DOMAIN: &str = "agents";

/// Subgraph namespace for bot catalog and runtime promotion (F2).
pub const BOTS_GRAPH_DOMAIN: &str = "bots";

/// Subgraph namespace for redacted order lineage (F3 / G2 orders).
pub const TRADING_GRAPH_DOMAIN: &str = "trading";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotCatalogProjection {
    pub bot_id: String,
    pub strategy_id: String,
    pub strategy_version: u32,
    pub timeframe: String,
    pub symbol: String,
    pub operation_mode: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotPromotionProjection {
    pub bot_id: String,
    pub promoted_by_agent_id: String,
    pub agency_id: Option<String>,
    pub promotion_state: String,
    pub promoted_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntentProjection {
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub status: String,
    pub execution_mode: String,
    pub submitted_at_ms: i64,
}

/// Lineage edge `(:Bot)-[:SUBMITTED]->(:OrderIntent)` (F3.1); no secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedEdgeProjection {
    pub bot_id: String,
    pub client_order_id: String,
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

    async fn project_bot_catalog_entry(
        &self,
        projection: &BotCatalogProjection,
    ) -> Result<(), GraphProjectionError>;

    async fn project_bot_promotion(
        &self,
        projection: &BotPromotionProjection,
    ) -> Result<(), GraphProjectionError>;

    async fn project_order_intent(
        &self,
        projection: &OrderIntentProjection,
    ) -> Result<(), GraphProjectionError>;

    async fn project_submitted_edge(
        &self,
        projection: &SubmittedEdgeProjection,
    ) -> Result<(), GraphProjectionError>;
}
