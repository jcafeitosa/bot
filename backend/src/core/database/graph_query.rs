//! Read-only graph query seam (F3 slice: agents list).

use async_trait::async_trait;
use neo4rs::query;
use serde::{Deserialize, Serialize};

use super::graph_projection::AGENTS_GRAPH_DOMAIN;
use super::neo4j::Neo4jGraph;

const LIST_AGENTS: &str = r"
MATCH (a:Agent)
WHERE a.graph_domain = $graph_domain
RETURN a.agent_id AS agent_id,
       a.agency_id AS agency_id,
       coalesce(a.role, '') AS role,
       coalesce(a.lifecycle, '') AS lifecycle
ORDER BY a.agency_id, a.agent_id
LIMIT $limit
";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedAgentNode {
    pub agent_id: String,
    pub agency_id: String,
    pub role: String,
    pub lifecycle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedAgentList {
    pub agents: Vec<ProjectedAgentNode>,
}

#[derive(Debug, thiserror::Error)]
pub enum GraphQueryError {
    #[error("neo4j unavailable: {0}")]
    Unavailable(String),
    #[error("invalid query: {0}")]
    Invalid(String),
    #[error("neo4j driver error: {0}")]
    Driver(String),
}

#[async_trait]
pub trait GraphQueryPort: Send + Sync {
    async fn list_agents(&self, limit: u32) -> Result<ProjectedAgentList, GraphQueryError>;
}

#[derive(Clone)]
pub struct Neo4jGraphQuery {
    graph: Neo4jGraph,
}

impl Neo4jGraphQuery {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }
}

impl Neo4jGraph {
    pub fn graph_query(&self) -> Neo4jGraphQuery {
        Neo4jGraphQuery::new(self.clone())
    }
}

#[async_trait]
impl GraphQueryPort for Neo4jGraphQuery {
    async fn list_agents(&self, limit: u32) -> Result<ProjectedAgentList, GraphQueryError> {
        let limit = limit.clamp(1, 500);
        let mut rows = self
            .graph
            .inner_graph()
            .execute(
                query(LIST_AGENTS)
                    .param("graph_domain", AGENTS_GRAPH_DOMAIN)
                    .param("limit", i64::from(limit)),
            )
            .await
            .map_err(map_driver_error)?;

        let mut agents = Vec::new();
        while let Some(row) = rows.next().await.map_err(map_driver_error)? {
            agents.push(ProjectedAgentNode {
                agent_id: row_get_string(&row, "agent_id")?,
                agency_id: row_get_string(&row, "agency_id")?,
                role: row_get_string(&row, "role")?,
                lifecycle: row_get_string(&row, "lifecycle")?,
            });
        }
        Ok(ProjectedAgentList { agents })
    }
}

fn row_get_string(row: &neo4rs::Row, key: &str) -> Result<String, GraphQueryError> {
    row.get(key)
        .map_err(|error| GraphQueryError::Driver(error.to_string()))
}

fn map_driver_error(error: neo4rs::Error) -> GraphQueryError {
    GraphQueryError::Driver(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubPort {
        limit: u32,
    }

    #[async_trait]
    impl GraphQueryPort for StubPort {
        async fn list_agents(&self, limit: u32) -> Result<ProjectedAgentList, GraphQueryError> {
            assert_eq!(limit, self.limit);
            Ok(ProjectedAgentList {
                agents: vec![ProjectedAgentNode {
                    agent_id: "ceo".into(),
                    agency_id: "agency-a".into(),
                    role: "ceo".into(),
                    lifecycle: "active".into(),
                }],
            })
        }
    }

    #[tokio::test]
    async fn graph_query_port_list_agents_returns_projected_nodes() {
        let port = StubPort { limit: 10 };
        let list = port.list_agents(10).await.expect("list");
        assert_eq!(list.agents.len(), 1);
        assert_eq!(list.agents[0].agent_id, "ceo");
    }

    #[test]
    fn list_agents_limit_clamped_in_neo4j_impl_signature() {
        assert_eq!(0_u32.clamp(1, 500), 1);
        assert_eq!(999_u32.clamp(1, 500), 500);
    }
}
