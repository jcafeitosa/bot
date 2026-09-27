use crate::core::database::{
    graph_projection_best_effort, AgentHierarchyProjection, GraphProjectionOutboxMessage,
    GraphProjectionPort, GraphProjectionSync, Neo4jGraph, ProjectedSupervisorKind,
};
use crate::modules::agents::adapters::pg_registry::lifecycle_to_sql;
use crate::modules::agents::models::{AgentDefinition, SupervisorRef};

pub fn agent_definition_to_projection(definition: &AgentDefinition) -> AgentHierarchyProjection {
    let (supervisor_kind, supervisor_owner_id, supervisor_agent_id) = match &definition.supervisor {
        SupervisorRef::Owner(owner) => (
            ProjectedSupervisorKind::Owner,
            Some(owner.as_str().to_string()),
            None,
        ),
        SupervisorRef::Agent(agent) => (
            ProjectedSupervisorKind::Agent,
            None,
            Some(agent.as_str().to_string()),
        ),
    };
    AgentHierarchyProjection {
        agency_id: definition.agency.as_str().to_string(),
        agent_id: definition.id.as_str().to_string(),
        role: definition.role.to_string(),
        lifecycle: lifecycle_to_sql(definition.state).to_string(),
        supervisor_kind,
        supervisor_owner_id,
        supervisor_agent_id,
        updated_at_ms: definition.updated_at_ms,
    }
}

pub async fn best_effort_project_agent_definition(
    sync: GraphProjectionSync<'_>,
    definition: &AgentDefinition,
) {
    if sync.postgres.is_none() && sync.neo4j.is_none() {
        return;
    }
    let projection = agent_definition_to_projection(definition);
    let message = GraphProjectionOutboxMessage::agent_hierarchy(projection);
    graph_projection_best_effort(sync, &[message]).await;
}

#[allow(dead_code)]
pub async fn project_agent_definition_via_port(
    port: &dyn GraphProjectionPort,
    definition: &AgentDefinition,
) -> Result<(), crate::core::database::GraphProjectionError> {
    let projection = agent_definition_to_projection(definition);
    port.project_agent_hierarchy(&projection).await
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::core::database::GraphProjectionError;
    use crate::modules::agents::models::{
        AgencyId, AgentCapabilities, AgentId, AgentLifecycleState, AgentRole, OwnerId,
        SupervisorRef,
    };
    use std::sync::Mutex;

    struct RecordingPort {
        last: Mutex<Option<AgentHierarchyProjection>>,
    }

    #[async_trait::async_trait]
    impl GraphProjectionPort for RecordingPort {
        async fn project_agent_hierarchy(
            &self,
            projection: &AgentHierarchyProjection,
        ) -> Result<(), GraphProjectionError> {
            *self.last.lock().expect("lock") = Some(projection.clone());
            Ok(())
        }

        async fn project_bot_catalog_entry(
            &self,
            _projection: &crate::core::database::BotCatalogProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_bot_promotion(
            &self,
            _projection: &crate::core::database::BotPromotionProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_order_intent(
            &self,
            _projection: &crate::core::database::OrderIntentProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_submitted_edge(
            &self,
            _projection: &crate::core::database::SubmittedEdgeProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }
    }

    fn sample_definition() -> AgentDefinition {
        AgentDefinition {
            id: AgentId::new("worker-1").unwrap(),
            agency: AgencyId::new("acme").unwrap(),
            owner: OwnerId::new("owner-1").unwrap(),
            display_name: "Worker".into(),
            role: AgentRole::Worker,
            supervisor: SupervisorRef::Agent(AgentId::new("ceo-1").unwrap()),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities::default(),
            created_at_ms: 1,
            updated_at_ms: 2,
        }
    }

    #[test]
    fn maps_definition_to_projection_without_display_name() {
        let projection = agent_definition_to_projection(&sample_definition());
        assert_eq!(projection.agent_id, "worker-1");
        assert_eq!(projection.agency_id, "acme");
        assert_eq!(projection.role, "worker");
        assert_eq!(projection.lifecycle, "active");
        assert_eq!(projection.supervisor_kind, ProjectedSupervisorKind::Agent);
        assert_eq!(projection.supervisor_agent_id.as_deref(), Some("ceo-1"));
    }

    #[tokio::test]
    async fn project_via_port_records_snapshot() {
        let port = RecordingPort {
            last: Mutex::new(None),
        };
        project_agent_definition_via_port(&port, &sample_definition())
            .await
            .expect("project");
        let recorded = port.last.lock().expect("lock").clone().expect("recorded");
        assert_eq!(recorded.agent_id, "worker-1");
    }
}

#[cfg(test)]
mod neo4j_integration_tests {
    use super::*;
    use crate::core::database::Neo4jGraph;
    use crate::core::database::{load_agents_stack_from_env, GraphProjectionSync};
    use crate::modules::agents::models::{
        AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState, AgentRole,
        OwnerId, SupervisorRef,
    };

    #[tokio::test]
    async fn neo4j_agent_supervision_chain_after_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let agency = format!(
            "agency-neo4j-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let ceo = AgentDefinition {
            id: AgentId::new("ceo-proj").unwrap(),
            agency: AgencyId::new(&agency).unwrap(),
            owner: OwnerId::new("owner-proj").unwrap(),
            display_name: "CEO".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(OwnerId::new("owner-proj").unwrap()),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities::default(),
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let worker = AgentDefinition {
            id: AgentId::new("worker-proj").unwrap(),
            agency: AgencyId::new(&agency).unwrap(),
            owner: OwnerId::new("owner-proj").unwrap(),
            display_name: "Worker".into(),
            role: AgentRole::Worker,
            supervisor: SupervisorRef::Agent(AgentId::new("ceo-proj").unwrap()),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities::default(),
            created_at_ms: 2,
            updated_at_ms: 2,
        };
        best_effort_project_agent_definition(
            GraphProjectionSync {
                postgres: None,
                neo4j: Some(&graph),
            },
            &ceo,
        )
        .await;
        best_effort_project_agent_definition(
            GraphProjectionSync {
                postgres: None,
                neo4j: Some(&graph),
            },
            &worker,
        )
        .await;
        let chains = graph
            .count_agent_supervision_paths(&agency, "ceo-proj", "worker-proj")
            .await
            .expect("count");
        assert_eq!(chains, 1);
    }
}
