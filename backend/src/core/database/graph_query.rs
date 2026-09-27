//! Read-only graph query seam (F3: agents list, supervision chain, bots for agent).

use async_trait::async_trait;
use neo4rs::query;
use serde::{Deserialize, Serialize};

use super::graph_projection::{AGENTS_GRAPH_DOMAIN, BOTS_GRAPH_DOMAIN};
use super::neo4j::Neo4jGraph;

fn validate_agency_agent_ids(agency_id: &str, agent_id: &str) -> Result<(), GraphQueryError> {
    if agency_id.trim().is_empty() || agent_id.trim().is_empty() {
        return Err(GraphQueryError::Invalid(
            "agency_id and agent_id are required".into(),
        ));
    }
    Ok(())
}

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

const SUPERVISION_CHAIN: &str = r"
MATCH (target:Agent {agent_id: $agent_id, agency_id: $agency_id})
WHERE target.graph_domain = $graph_domain
MATCH p = (root)-[:SUPERVISES*0..32]->(target)
WHERE NOT (root)<-[:SUPERVISES]-()
WITH p
ORDER BY length(p) DESC
LIMIT 1
UNWIND range(0, length(p)) AS depth
WITH nodes(p)[depth] AS n, depth
RETURN depth,
       CASE WHEN 'Owner' IN labels(n) THEN 'owner' ELSE 'agent' END AS kind,
       coalesce(n.agent_id, n.owner_id, '') AS node_id
ORDER BY depth
";

const BOTS_FOR_AGENT: &str = r"
MATCH (a:Agent {agent_id: $agent_id, agency_id: $agency_id})
WHERE a.graph_domain = $agents_graph_domain
OPTIONAL MATCH (a)-[:PROMOTED_BY]->(b:Bot)
WHERE b.graph_domain = $bots_graph_domain
RETURN b.bot_id AS bot_id,
       coalesce(b.strategy_id, '') AS strategy_id,
       coalesce(b.promotion_state, '') AS promotion_state,
       coalesce(b.operation_mode, '') AS operation_mode
ORDER BY bot_id
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisionChainNode {
    pub depth: u32,
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedSupervisionChain {
    pub agency_id: String,
    pub agent_id: String,
    pub chain: Vec<SupervisionChainNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedBotForAgent {
    pub bot_id: String,
    pub strategy_id: String,
    pub promotion_state: String,
    pub operation_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedBotsForAgent {
    pub agency_id: String,
    pub agent_id: String,
    pub bots: Vec<ProjectedBotForAgent>,
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

    async fn supervision_chain(
        &self,
        agency_id: &str,
        agent_id: &str,
    ) -> Result<ProjectedSupervisionChain, GraphQueryError>;

    async fn bots_for_agent(
        &self,
        agency_id: &str,
        agent_id: &str,
        limit: u32,
    ) -> Result<ProjectedBotsForAgent, GraphQueryError>;
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

    async fn supervision_chain(
        &self,
        agency_id: &str,
        agent_id: &str,
    ) -> Result<ProjectedSupervisionChain, GraphQueryError> {
        validate_agency_agent_ids(agency_id, agent_id)?;
        let mut rows = self
            .graph
            .inner_graph()
            .execute(
                query(SUPERVISION_CHAIN)
                    .param("graph_domain", AGENTS_GRAPH_DOMAIN)
                    .param("agency_id", agency_id)
                    .param("agent_id", agent_id),
            )
            .await
            .map_err(map_driver_error)?;

        let mut chain = Vec::new();
        while let Some(row) = rows.next().await.map_err(map_driver_error)? {
            let depth: i64 = row
                .get("depth")
                .map_err(|error| GraphQueryError::Driver(error.to_string()))?;
            chain.push(SupervisionChainNode {
                depth: u32::try_from(depth).map_err(|_| {
                    GraphQueryError::Driver("supervision chain depth out of range".into())
                })?,
                kind: row_get_string(&row, "kind")?,
                id: row_get_string(&row, "node_id")?,
            });
        }
        if chain.is_empty() {
            return Err(GraphQueryError::Invalid(
                "agent not found in graph projection".into(),
            ));
        }
        Ok(ProjectedSupervisionChain {
            agency_id: agency_id.to_string(),
            agent_id: agent_id.to_string(),
            chain,
        })
    }

    async fn bots_for_agent(
        &self,
        agency_id: &str,
        agent_id: &str,
        limit: u32,
    ) -> Result<ProjectedBotsForAgent, GraphQueryError> {
        validate_agency_agent_ids(agency_id, agent_id)?;
        let limit = limit.clamp(1, 500);
        let agent_exists = self
            .graph
            .inner_graph()
            .execute(
                query(
                    "MATCH (a:Agent {agent_id: $agent_id, agency_id: $agency_id}) \
                     WHERE a.graph_domain = $agents_graph_domain RETURN a.agent_id AS agent_id LIMIT 1",
                )
                .param("agents_graph_domain", AGENTS_GRAPH_DOMAIN)
                .param("agency_id", agency_id)
                .param("agent_id", agent_id),
            )
            .await
            .map_err(map_driver_error)?;
        let mut exists_rows = agent_exists;
        if exists_rows
            .next()
            .await
            .map_err(map_driver_error)?
            .is_none()
        {
            return Err(GraphQueryError::Invalid(
                "agent not found in graph projection".into(),
            ));
        }

        let mut rows = self
            .graph
            .inner_graph()
            .execute(
                query(BOTS_FOR_AGENT)
                    .param("agents_graph_domain", AGENTS_GRAPH_DOMAIN)
                    .param("bots_graph_domain", BOTS_GRAPH_DOMAIN)
                    .param("agency_id", agency_id)
                    .param("agent_id", agent_id)
                    .param("limit", i64::from(limit)),
            )
            .await
            .map_err(map_driver_error)?;

        let mut bots = Vec::new();
        while let Some(row) = rows.next().await.map_err(map_driver_error)? {
            let bot_id = row_get_optional_string(&row, "bot_id")?;
            if bot_id.is_none() {
                continue;
            }
            let bot_id = bot_id.expect("checked");
            if bot_id.is_empty() {
                continue;
            }
            bots.push(ProjectedBotForAgent {
                bot_id,
                strategy_id: row_get_string(&row, "strategy_id")?,
                promotion_state: row_get_string(&row, "promotion_state")?,
                operation_mode: row_get_string(&row, "operation_mode")?,
            });
        }
        Ok(ProjectedBotsForAgent {
            agency_id: agency_id.to_string(),
            agent_id: agent_id.to_string(),
            bots,
        })
    }
}

fn row_get_optional_string(
    row: &neo4rs::Row,
    key: &str,
) -> Result<Option<String>, GraphQueryError> {
    match row.get::<Option<String>>(key) {
        Ok(value) => Ok(value),
        Err(error) => Err(GraphQueryError::Driver(error.to_string())),
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

        async fn supervision_chain(
            &self,
            agency_id: &str,
            agent_id: &str,
        ) -> Result<ProjectedSupervisionChain, GraphQueryError> {
            validate_agency_agent_ids(agency_id, agent_id)?;
            Ok(ProjectedSupervisionChain {
                agency_id: agency_id.to_string(),
                agent_id: agent_id.to_string(),
                chain: vec![
                    SupervisionChainNode {
                        depth: 0,
                        kind: "owner".into(),
                        id: "owner-1".into(),
                    },
                    SupervisionChainNode {
                        depth: 1,
                        kind: "agent".into(),
                        id: agent_id.to_string(),
                    },
                ],
            })
        }

        async fn bots_for_agent(
            &self,
            agency_id: &str,
            agent_id: &str,
            limit: u32,
        ) -> Result<ProjectedBotsForAgent, GraphQueryError> {
            validate_agency_agent_ids(agency_id, agent_id)?;
            assert_eq!(limit, self.limit);
            Ok(ProjectedBotsForAgent {
                agency_id: agency_id.to_string(),
                agent_id: agent_id.to_string(),
                bots: vec![ProjectedBotForAgent {
                    bot_id: "bot-1".into(),
                    strategy_id: "ema_cross".into(),
                    promotion_state: "active".into(),
                    operation_mode: "day_trader".into(),
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

    #[tokio::test]
    async fn graph_query_port_supervision_chain_returns_ordered_nodes() {
        let port = StubPort { limit: 10 };
        let chain = port
            .supervision_chain("agency-a", "worker-1")
            .await
            .expect("chain");
        assert_eq!(chain.chain.len(), 2);
        assert_eq!(chain.chain[0].kind, "owner");
        assert_eq!(chain.chain[1].id, "worker-1");
    }

    #[test]
    fn supervision_chain_rejects_empty_ids() {
        let err = validate_agency_agent_ids("", "ceo").expect_err("err");
        assert!(matches!(err, GraphQueryError::Invalid(_)));
    }

    #[tokio::test]
    async fn graph_query_port_bots_for_agent_returns_projected_bots() {
        let port = StubPort { limit: 10 };
        let list = port
            .bots_for_agent("agency-a", "agent-promoter", 10)
            .await
            .expect("bots");
        assert_eq!(list.bots.len(), 1);
        assert_eq!(list.bots[0].bot_id, "bot-1");
    }

    #[tokio::test]
    async fn neo4j_list_agents_after_local_graph() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = crate::core::database::load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        graph.ping().await.expect("ping");
        let _ = graph
            .graph_query()
            .list_agents(5)
            .await
            .expect("list agents");
    }
}

#[cfg(test)]
mod neo4j_integration_tests {
    use super::*;
    use crate::core::database::{
        load_agents_stack_from_env, AgentHierarchyProjection, BotCatalogProjection,
        BotPromotionProjection, GraphProjectionPort, GraphQueryPort, Neo4jAgentHierarchyProjector,
        Neo4jBotProjector, ProjectedSupervisorKind,
    };

    #[tokio::test]
    async fn neo4j_supervision_chain_query_after_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let projector = Neo4jAgentHierarchyProjector::new(graph.clone());
        let port = Neo4jGraphQuery::new(graph);
        let agency = format!(
            "agency-graph-query-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let ceo = AgentHierarchyProjection {
            agency_id: agency.clone(),
            agent_id: "ceo-chain".into(),
            role: "ceo".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Owner,
            supervisor_owner_id: Some("owner-chain".into()),
            supervisor_agent_id: None,
            updated_at_ms: 1,
        };
        let worker = AgentHierarchyProjection {
            agency_id: agency.clone(),
            agent_id: "worker-chain".into(),
            role: "worker".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Agent,
            supervisor_owner_id: None,
            supervisor_agent_id: Some("ceo-chain".into()),
            updated_at_ms: 2,
        };
        projector
            .project_agent_hierarchy(&ceo)
            .await
            .expect("ceo projection");
        projector
            .project_agent_hierarchy(&worker)
            .await
            .expect("worker projection");
        let chain = port
            .supervision_chain(&agency, "worker-chain")
            .await
            .expect("chain");
        assert_eq!(chain.chain.len(), 3);
        assert_eq!(chain.chain[0].kind, "owner");
        assert_eq!(chain.chain[0].id, "owner-chain");
        assert_eq!(chain.chain[2].id, "worker-chain");
    }

    #[tokio::test]
    async fn neo4j_bots_for_agent_after_catalog_and_promotion_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let agent_projector = Neo4jAgentHierarchyProjector::new(graph.clone());
        let bot_projector = Neo4jBotProjector::new(graph.clone());
        let port = Neo4jGraphQuery::new(graph);
        let agency = format!(
            "agency-bots-query-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let promoter = AgentHierarchyProjection {
            agency_id: agency.clone(),
            agent_id: "agent-promoter".into(),
            role: "ceo".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Owner,
            supervisor_owner_id: Some("owner-bots".into()),
            supervisor_agent_id: None,
            updated_at_ms: 1,
        };
        agent_projector
            .project_agent_hierarchy(&promoter)
            .await
            .expect("agent projection");
        let catalog = BotCatalogProjection {
            bot_id: format!("bot-query-{}", agency),
            strategy_id: "ema_cross".into(),
            strategy_version: 1,
            timeframe: "1m".into(),
            symbol: "BTC/USDT".into(),
            operation_mode: "day_trader".into(),
            updated_at_ms: 2,
        };
        bot_projector
            .project_bot_catalog_entry(&catalog)
            .await
            .expect("catalog");
        let promotion = BotPromotionProjection {
            bot_id: catalog.bot_id.clone(),
            promoted_by_agent_id: "agent-promoter".into(),
            agency_id: Some(agency.clone()),
            promotion_state: "active".into(),
            promoted_at_ms: 3,
        };
        bot_projector
            .project_bot_promotion(&promotion)
            .await
            .expect("promotion");
        let bots = port
            .bots_for_agent(&agency, "agent-promoter", 10)
            .await
            .expect("bots for agent");
        assert_eq!(bots.bots.len(), 1);
        assert_eq!(bots.bots[0].bot_id, catalog.bot_id);
        assert_eq!(bots.bots[0].strategy_id, "ema_cross");
    }
}
