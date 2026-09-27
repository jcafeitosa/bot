//! Dual-store database seam (PostgreSQL 18+ / Timescale / pgvector + Neo4j).
#![allow(dead_code, unused_imports)]

mod bundle;
mod config;
pub mod graph_cli;
mod graph_projection;
pub mod graph_projection_cli;
mod graph_projection_outbox;
mod graph_projection_outbox_worker;
mod graph_query;
pub(crate) mod monitor_bootstrap;
mod neo4j;
mod neo4j_agent_hierarchy;
mod neo4j_bot_projection;
mod neo4j_order_intent;
mod postgres;

pub use bundle::AppDatabases;
pub use config::{load_agents_stack_from_env, postgres_url_from_env, DatabaseConfigError};
pub use graph_cli::{
    GraphCli, GraphCommand, GraphQueryAgentsCli, GraphQueryCli, GraphQueryCommand,
    GraphQuerySupervisionChainCli,
};
pub use graph_projection::{
    AgentHierarchyProjection, BotCatalogProjection, BotPromotionProjection, GraphProjectionError,
    GraphProjectionPort, OrderIntentProjection, ProjectedSupervisorKind, SubmittedEdgeProjection,
    AGENTS_GRAPH_DOMAIN, BOTS_GRAPH_DOMAIN, TRADING_GRAPH_DOMAIN,
};
pub use graph_projection_cli::{
    GraphProjectionCli, GraphProjectionCommand, GraphProjectionDrainCli,
};
pub use graph_projection_outbox::{
    drain_graph_projection_outbox, drain_graph_projection_outbox_with_port,
    enqueue_graph_projection_outbox, enqueue_graph_projection_outbox_tx,
    graph_projection_best_effort, graph_projection_drain_best_effort, DrainSummary,
    GraphProjectionOutboxError, GraphProjectionOutboxMessage, GraphProjectionPayload,
    GraphProjectionSync,
};
pub use graph_projection_outbox_worker::{
    fetch_graph_projection_outbox_stats, graph_projection_outbox_degraded,
    spawn_graph_projection_outbox_drain_worker, GraphProjectionOutboxStats,
};
pub use graph_query::{
    GraphQueryError, GraphQueryPort, ProjectedAgentList, ProjectedAgentNode, ProjectedBotForAgent,
    ProjectedBotsForAgent, ProjectedSupervisionChain, SupervisionChainNode,
};
pub use monitor_bootstrap::{
    bootstrap_monitor_postgres, connect_postgres_for_monitor, postgres_for_cli_persist,
    MonitorBootstrapError,
};
pub use neo4j::Neo4jGraph;
pub use neo4j_agent_hierarchy::Neo4jAgentHierarchyProjector;
pub use neo4j_bot_projection::Neo4jBotProjector;
pub use neo4j_order_intent::Neo4jOrderIntentProjector;
pub use postgres::{DatabaseError, PostgresDatabase};
