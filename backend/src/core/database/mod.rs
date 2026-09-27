//! Dual-store database seam (PostgreSQL 18+ / Timescale / pgvector + Neo4j).
#![allow(dead_code, unused_imports)]

mod bundle;
mod config;
mod graph_projection;
pub(crate) mod monitor_bootstrap;
mod neo4j;
mod neo4j_agent_hierarchy;
mod postgres;

pub use bundle::AppDatabases;
pub use config::{load_agents_stack_from_env, postgres_url_from_env, DatabaseConfigError};
pub use graph_projection::{
    AgentHierarchyProjection, GraphProjectionError, GraphProjectionPort, ProjectedSupervisorKind,
    AGENTS_GRAPH_DOMAIN,
};
pub use monitor_bootstrap::{
    bootstrap_monitor_postgres, connect_postgres_for_monitor, postgres_for_cli_persist,
    MonitorBootstrapError,
};
pub use neo4j::Neo4jGraph;
pub use neo4j_agent_hierarchy::Neo4jAgentHierarchyProjector;
pub use postgres::{DatabaseError, PostgresDatabase};
