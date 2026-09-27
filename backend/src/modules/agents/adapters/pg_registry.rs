#![allow(dead_code)]
use sqlx::PgPool;

use super::persistence::AgentIdentityStore;
use crate::core::database::PostgresDatabase;
use crate::modules::agents::models::{
    AgentDefinition, AgentLifecycleState, IdentityAuditEvent, IdentityEventKind, SupervisorRef,
};

#[derive(Clone)]
pub struct PgAgentIdentityStore {
    pool: PgPool,
}

impl PgAgentIdentityStore {
    pub fn new(db: &PostgresDatabase) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }
}

pub fn lifecycle_to_sql(state: AgentLifecycleState) -> &'static str {
    match state {
        AgentLifecycleState::Active => "active",
        AgentLifecycleState::Paused => "paused",
        AgentLifecycleState::Retired => "retired",
    }
}

#[allow(dead_code)]
pub fn lifecycle_from_sql(raw: &str) -> Result<AgentLifecycleState, String> {
    match raw {
        "active" => Ok(AgentLifecycleState::Active),
        "paused" => Ok(AgentLifecycleState::Paused),
        "retired" => Ok(AgentLifecycleState::Retired),
        other => Err(format!("unknown lifecycle_state in database: {other}")),
    }
}

pub fn event_kind_to_sql(kind: IdentityEventKind) -> &'static str {
    match kind {
        IdentityEventKind::Registered => "registered",
        IdentityEventKind::Paused => "paused",
        IdentityEventKind::Resumed => "resumed",
        IdentityEventKind::Retired => "retired",
    }
}

fn supervisor_columns(
    supervisor: &SupervisorRef,
) -> (&'static str, Option<String>, Option<String>) {
    match supervisor {
        SupervisorRef::Owner(owner) => ("owner", Some(owner.as_str().to_string()), None),
        SupervisorRef::Agent(agent) => ("agent", None, Some(agent.as_str().to_string())),
    }
}

#[async_trait::async_trait]
impl AgentIdentityStore for PgAgentIdentityStore {
    async fn upsert_agent(&self, definition: &AgentDefinition) -> Result<(), String> {
        let (supervisor_kind, supervisor_owner_id, supervisor_agent_id) =
            supervisor_columns(&definition.supervisor);
        sqlx::query(
            "INSERT INTO agent_identities \
             (agent_id, agency_id, owner_id, display_name, role, supervisor_kind, supervisor_owner_id, supervisor_agent_id, lifecycle_state, consult_jev, created_at_ms, updated_at_ms) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
             ON CONFLICT (agent_id) DO UPDATE SET \
             agency_id = EXCLUDED.agency_id, owner_id = EXCLUDED.owner_id, display_name = EXCLUDED.display_name, \
             role = EXCLUDED.role, supervisor_kind = EXCLUDED.supervisor_kind, supervisor_owner_id = EXCLUDED.supervisor_owner_id, \
             supervisor_agent_id = EXCLUDED.supervisor_agent_id, lifecycle_state = EXCLUDED.lifecycle_state, \
             consult_jev = EXCLUDED.consult_jev, updated_at_ms = EXCLUDED.updated_at_ms",
        )
        .bind(definition.id.as_str())
        .bind(definition.agency.as_str())
        .bind(definition.owner.as_str())
        .bind(&definition.display_name)
        .bind(definition.role.to_string())
        .bind(supervisor_kind)
        .bind(supervisor_owner_id)
        .bind(supervisor_agent_id)
        .bind(lifecycle_to_sql(definition.state))
        .bind(definition.capabilities.consult_jev)
        .bind(definition.created_at_ms)
        .bind(definition.updated_at_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn append_event(&self, event: &IdentityAuditEvent) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO agent_identity_events (agent_id, agency_id, kind, at_ms) VALUES ($1,$2,$3,$4)",
        )
        .bind(event.agent_id.as_str())
        .bind(event.agency.as_str())
        .bind(event_kind_to_sql(event.kind))
        .bind(event.at_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::agents::models::{AgencyId, AgentId, OwnerId};

    #[test]
    fn lifecycle_sql_round_trip() {
        for state in [
            AgentLifecycleState::Active,
            AgentLifecycleState::Paused,
            AgentLifecycleState::Retired,
        ] {
            let sql = lifecycle_to_sql(state);
            assert_eq!(lifecycle_from_sql(sql).unwrap(), state);
        }
    }

    #[test]
    fn supervisor_owner_maps_to_columns() {
        let owner = OwnerId::new("owner-1").unwrap();
        let (kind, owner_id, agent_id) = supervisor_columns(&SupervisorRef::Owner(owner));
        assert_eq!(kind, "owner");
        assert_eq!(owner_id.as_deref(), Some("owner-1"));
        assert!(agent_id.is_none());
    }

    #[test]
    fn supervisor_agent_maps_to_columns() {
        let agent = AgentId::new("ceo").unwrap();
        let (kind, owner_id, agent_id) = supervisor_columns(&SupervisorRef::Agent(agent));
        assert_eq!(kind, "agent");
        assert!(owner_id.is_none());
        assert_eq!(agent_id.as_deref(), Some("ceo"));
    }
}
