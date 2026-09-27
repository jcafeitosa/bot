use neo4rs::query;

use super::graph_projection::{
    AgentHierarchyProjection, GraphProjectionError, GraphProjectionPort, ProjectedSupervisorKind,
    AGENTS_GRAPH_DOMAIN,
};
use super::neo4j::{Neo4jError, Neo4jGraph};

const MERGE_AGENT_HIERARCHY: &str = r"
MERGE (a:Agent {agent_id: $agent_id, agency_id: $agency_id})
SET a.graph_domain = $graph_domain,
    a.role = $role,
    a.lifecycle = $lifecycle,
    a.updated_at_ms = $updated_at_ms
WITH a
OPTIONAL MATCH ()-[r:SUPERVISES]->(a)
DELETE r
WITH a
FOREACH (_ IN CASE WHEN $supervisor_kind = 'agent' THEN [1] ELSE [] END |
  MERGE (s:Agent {agent_id: $supervisor_agent_id, agency_id: $agency_id})
  SET s.graph_domain = $graph_domain
  MERGE (s)-[:SUPERVISES]->(a)
)
FOREACH (_ IN CASE WHEN $supervisor_kind = 'owner' THEN [1] ELSE [] END |
  MERGE (o:Owner {owner_id: $supervisor_owner_id, agency_id: $agency_id})
  SET o.graph_domain = $graph_domain
  MERGE (o)-[:SUPERVISES]->(a)
)
";

#[derive(Clone)]
pub struct Neo4jAgentHierarchyProjector {
    graph: Neo4jGraph,
}

impl Neo4jAgentHierarchyProjector {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    fn supervisor_kind_sql(kind: ProjectedSupervisorKind) -> &'static str {
        match kind {
            ProjectedSupervisorKind::Owner => "owner",
            ProjectedSupervisorKind::Agent => "agent",
        }
    }
}

impl Neo4jGraph {
    pub fn agent_hierarchy_projector(&self) -> Neo4jAgentHierarchyProjector {
        Neo4jAgentHierarchyProjector::new(self.clone())
    }
}

#[async_trait::async_trait]
impl GraphProjectionPort for Neo4jAgentHierarchyProjector {
    async fn project_agent_hierarchy(
        &self,
        projection: &AgentHierarchyProjection,
    ) -> Result<(), GraphProjectionError> {
        validate_projection(projection)?;
        let supervisor_kind =
            Neo4jAgentHierarchyProjector::supervisor_kind_sql(projection.supervisor_kind);
        let q = query(MERGE_AGENT_HIERARCHY)
            .param("agent_id", projection.agent_id.as_str())
            .param("agency_id", projection.agency_id.as_str())
            .param("graph_domain", AGENTS_GRAPH_DOMAIN)
            .param("role", projection.role.as_str())
            .param("lifecycle", projection.lifecycle.as_str())
            .param("updated_at_ms", projection.updated_at_ms)
            .param("supervisor_kind", supervisor_kind)
            .param(
                "supervisor_agent_id",
                projection.supervisor_agent_id.as_deref().unwrap_or(""),
            )
            .param(
                "supervisor_owner_id",
                projection.supervisor_owner_id.as_deref().unwrap_or(""),
            );
        self.graph
            .run_write(q)
            .await
            .map_err(|error| GraphProjectionError::Driver(error.to_string()))
    }
}

fn validate_projection(projection: &AgentHierarchyProjection) -> Result<(), GraphProjectionError> {
    match projection.supervisor_kind {
        ProjectedSupervisorKind::Owner => {
            if projection
                .supervisor_owner_id
                .as_deref()
                .unwrap_or("")
                .is_empty()
            {
                return Err(GraphProjectionError::Invalid(
                    "owner supervisor requires supervisor_owner_id".into(),
                ));
            }
        }
        ProjectedSupervisorKind::Agent => {
            if projection
                .supervisor_agent_id
                .as_deref()
                .unwrap_or("")
                .is_empty()
            {
                return Err(GraphProjectionError::Invalid(
                    "agent supervisor requires supervisor_agent_id".into(),
                ));
            }
        }
    }
    Ok(())
}
