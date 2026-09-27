use sqlx::PgPool;

use super::graph_projection::agent_graph_projection_outbox_message;
use super::persistence::AgentIdentityStore;
use crate::core::database::{
    enqueue_graph_projection_outbox_tx, GraphProjectionOutboxError, PostgresDatabase,
};
use crate::modules::agents::models::{
    AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState, AgentRole,
    IdentityAuditEvent, IdentityEventKind, OwnerId, SupervisorRef,
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

    /// Atomically upserts identity, appends audit event, and enqueues graph outbox (F2.1.3+ agents).
    pub async fn persist_identity_and_enqueue_graph_projection(
        &self,
        definition: &AgentDefinition,
        event: &IdentityAuditEvent,
    ) -> Result<(), String> {
        let message = agent_graph_projection_outbox_message(definition);
        let (supervisor_kind, supervisor_owner_id, supervisor_agent_id) =
            supervisor_columns(&definition.supervisor);
        let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO agent_identities \
             (agent_id, agency_id, owner_id, display_name, role, supervisor_kind, supervisor_owner_id, supervisor_agent_id, lifecycle_state, consult_jev, promote_runtime_bot, created_at_ms, updated_at_ms) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) \
             ON CONFLICT (agent_id) DO UPDATE SET \
             agency_id = EXCLUDED.agency_id, owner_id = EXCLUDED.owner_id, display_name = EXCLUDED.display_name, \
             role = EXCLUDED.role, supervisor_kind = EXCLUDED.supervisor_kind, supervisor_owner_id = EXCLUDED.supervisor_owner_id, \
             supervisor_agent_id = EXCLUDED.supervisor_agent_id, lifecycle_state = EXCLUDED.lifecycle_state, \
             consult_jev = EXCLUDED.consult_jev, promote_runtime_bot = EXCLUDED.promote_runtime_bot, updated_at_ms = EXCLUDED.updated_at_ms",
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
        .bind(definition.capabilities.promote_runtime_bot)
        .bind(definition.created_at_ms)
        .bind(definition.updated_at_ms)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO agent_identity_events (agent_id, agency_id, kind, at_ms) VALUES ($1,$2,$3,$4)",
        )
        .bind(event.agent_id.as_str())
        .bind(event.agency.as_str())
        .bind(event_kind_to_sql(event.kind))
        .bind(event.at_ms)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        enqueue_graph_projection_outbox_tx(&mut tx, &message)
            .await
            .map_err(|error| match error {
                GraphProjectionOutboxError::Store(message) => {
                    format!("graph projection outbox enqueue: {message}")
                }
                GraphProjectionOutboxError::InvalidPayload(message) => {
                    format!("graph projection outbox payload: {message}")
                }
                GraphProjectionOutboxError::Neo4jUnavailable(_) => {
                    "graph projection outbox enqueue: neo4j unavailable".into()
                }
            })?;
        tx.commit().await.map_err(|error| error.to_string())?;
        Ok(())
    }
}

pub fn lifecycle_to_sql(state: AgentLifecycleState) -> &'static str {
    match state {
        AgentLifecycleState::Active => "active",
        AgentLifecycleState::Paused => "paused",
        AgentLifecycleState::Retired => "retired",
    }
}

pub fn lifecycle_from_sql(raw: &str) -> Result<AgentLifecycleState, String> {
    match raw {
        "active" => Ok(AgentLifecycleState::Active),
        "paused" => Ok(AgentLifecycleState::Paused),
        "retired" => Ok(AgentLifecycleState::Retired),
        other => Err(format!("unknown lifecycle_state in database: {other}")),
    }
}

pub fn role_from_sql(raw: &str) -> Result<AgentRole, String> {
    match raw {
        "ceo" => Ok(AgentRole::Ceo),
        "level_b" => Ok(AgentRole::LevelB),
        "level_a" => Ok(AgentRole::LevelA),
        "specialist" => Ok(AgentRole::Specialist),
        "worker" => Ok(AgentRole::Worker),
        other => Err(format!("unknown role in database: {other}")),
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

pub fn event_kind_from_sql(raw: &str) -> Result<IdentityEventKind, String> {
    match raw {
        "registered" => Ok(IdentityEventKind::Registered),
        "paused" => Ok(IdentityEventKind::Paused),
        "resumed" => Ok(IdentityEventKind::Resumed),
        "retired" => Ok(IdentityEventKind::Retired),
        other => Err(format!("unknown identity event kind in database: {other}")),
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

fn supervisor_from_row(
    kind: &str,
    owner_id: Option<String>,
    agent_id: Option<String>,
) -> Result<SupervisorRef, String> {
    match kind {
        "owner" => {
            let raw = owner_id.ok_or_else(|| "supervisor_owner_id missing".to_string())?;
            Ok(SupervisorRef::Owner(
                OwnerId::new(raw).map_err(|e| e.to_string())?,
            ))
        }
        "agent" => {
            let raw = agent_id.ok_or_else(|| "supervisor_agent_id missing".to_string())?;
            Ok(SupervisorRef::Agent(
                AgentId::new(raw).map_err(|e| e.to_string())?,
            ))
        }
        other => Err(format!("unknown supervisor_kind in database: {other}")),
    }
}

#[derive(sqlx::FromRow)]
struct AgentIdentityRow {
    agent_id: String,
    agency_id: String,
    owner_id: String,
    display_name: String,
    role: String,
    supervisor_kind: String,
    supervisor_owner_id: Option<String>,
    supervisor_agent_id: Option<String>,
    lifecycle_state: String,
    consult_jev: bool,
    promote_runtime_bot: bool,
    created_at_ms: i64,
    updated_at_ms: i64,
}

impl AgentIdentityRow {
    fn into_definition(self) -> Result<AgentDefinition, String> {
        Ok(AgentDefinition {
            id: AgentId::new(self.agent_id).map_err(|e| e.to_string())?,
            agency: AgencyId::new(self.agency_id).map_err(|e| e.to_string())?,
            owner: OwnerId::new(self.owner_id).map_err(|e| e.to_string())?,
            display_name: self.display_name,
            role: role_from_sql(&self.role)?,
            supervisor: supervisor_from_row(
                &self.supervisor_kind,
                self.supervisor_owner_id,
                self.supervisor_agent_id,
            )?,
            state: lifecycle_from_sql(&self.lifecycle_state)?,
            capabilities: AgentCapabilities {
                consult_jev: self.consult_jev,
                promote_runtime_bot: self.promote_runtime_bot,
            },
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AgentIdentityEventRow {
    agent_id: String,
    agency_id: String,
    kind: String,
    at_ms: i64,
}

impl AgentIdentityEventRow {
    fn into_event(self) -> Result<IdentityAuditEvent, String> {
        Ok(IdentityAuditEvent {
            agent_id: AgentId::new(self.agent_id).map_err(|e| e.to_string())?,
            agency: AgencyId::new(self.agency_id).map_err(|e| e.to_string())?,
            kind: event_kind_from_sql(&self.kind)?,
            at_ms: self.at_ms,
        })
    }
}

#[async_trait::async_trait]
impl AgentIdentityStore for PgAgentIdentityStore {
    async fn upsert_agent(&self, definition: &AgentDefinition) -> Result<(), String> {
        let (supervisor_kind, supervisor_owner_id, supervisor_agent_id) =
            supervisor_columns(&definition.supervisor);
        sqlx::query(
            "INSERT INTO agent_identities \
             (agent_id, agency_id, owner_id, display_name, role, supervisor_kind, supervisor_owner_id, supervisor_agent_id, lifecycle_state, consult_jev, promote_runtime_bot, created_at_ms, updated_at_ms) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) \
             ON CONFLICT (agent_id) DO UPDATE SET \
             agency_id = EXCLUDED.agency_id, owner_id = EXCLUDED.owner_id, display_name = EXCLUDED.display_name, \
             role = EXCLUDED.role, supervisor_kind = EXCLUDED.supervisor_kind, supervisor_owner_id = EXCLUDED.supervisor_owner_id, \
             supervisor_agent_id = EXCLUDED.supervisor_agent_id, lifecycle_state = EXCLUDED.lifecycle_state, \
             consult_jev = EXCLUDED.consult_jev, promote_runtime_bot = EXCLUDED.promote_runtime_bot, updated_at_ms = EXCLUDED.updated_at_ms",
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
        .bind(definition.capabilities.promote_runtime_bot)
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

    async fn load_snapshot(
        &self,
    ) -> Result<(Vec<AgentDefinition>, Vec<IdentityAuditEvent>), String> {
        let rows = sqlx::query_as::<_, AgentIdentityRow>(
            "SELECT agent_id, agency_id, owner_id, display_name, role, supervisor_kind, \
             supervisor_owner_id, supervisor_agent_id, lifecycle_state, consult_jev, promote_runtime_bot, created_at_ms, updated_at_ms \
             FROM agent_identities ORDER BY created_at_ms ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        let agents = rows
            .into_iter()
            .map(AgentIdentityRow::into_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let event_rows = sqlx::query_as::<_, AgentIdentityEventRow>(
            "SELECT agent_id, agency_id, kind, at_ms FROM agent_identity_events ORDER BY event_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        let audit = event_rows
            .into_iter()
            .map(AgentIdentityEventRow::into_event)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((agents, audit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::agents::models::{
        AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState, AgentRole,
        IdentityAuditEvent, IdentityEventKind, OwnerId, SupervisorRef,
    };

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
    #[tokio::test]
    async fn pg_agent_identity_and_graph_projection_same_transaction() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        use crate::modules::agents::adapters::graph_projection::agent_graph_projection_outbox_message;
        let store = PgAgentIdentityStore::new(db.as_postgres());
        let owner = OwnerId::new("owner-outbox-tx").unwrap();
        let agency = AgencyId::new("agency-outbox-tx").unwrap();
        let agent_id = format!(
            "agent-outbox-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let definition = AgentDefinition {
            id: AgentId::new(&agent_id).unwrap(),
            agency: agency.clone(),
            owner: owner.clone(),
            display_name: "Agent".into(),
            role: AgentRole::Worker,
            supervisor: SupervisorRef::Owner(owner),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities::default(),
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let event = IdentityAuditEvent {
            agent_id: definition.id.clone(),
            agency: agency.clone(),
            kind: IdentityEventKind::Registered,
            at_ms: 1,
        };
        let message = agent_graph_projection_outbox_message(&definition);
        store
            .persist_identity_and_enqueue_graph_projection(&definition, &event)
            .await
            .expect("persist tx");
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM graph_projection_outbox WHERE idempotency_key = $1 AND status = 'pending'",
        )
        .bind(&message.idempotency_key)
        .fetch_one(db.pool())
        .await
        .expect("outbox count");
        assert_eq!(pending, 1);
    }

    #[tokio::test]
    async fn pg_identity_snapshot_round_trip() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgAgentIdentityStore::new(db.as_postgres());
        let owner = OwnerId::new("owner-pg").unwrap();
        let agency = AgencyId::new("agency-pg").unwrap();
        let definition = AgentDefinition {
            id: AgentId::new("ceo-pg").unwrap(),
            agency: agency.clone(),
            owner: owner.clone(),
            display_name: "CEO".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(owner),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities {
                consult_jev: true,
                promote_runtime_bot: true,
            },
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let event = IdentityAuditEvent {
            agent_id: definition.id.clone(),
            agency: agency.clone(),
            kind: IdentityEventKind::Registered,
            at_ms: 1,
        };
        store
            .persist_identity_and_enqueue_graph_projection(&definition, &event)
            .await
            .expect("register persist");

        let mut paused = definition.clone();
        paused.state = AgentLifecycleState::Paused;
        paused.updated_at_ms = 2;
        store
            .persist_identity_and_enqueue_graph_projection(
                &paused,
                &IdentityAuditEvent {
                    agent_id: definition.id.clone(),
                    agency: agency.clone(),
                    kind: IdentityEventKind::Paused,
                    at_ms: 2,
                },
            )
            .await
            .expect("pause persist");

        let mut resumed = paused.clone();
        resumed.state = AgentLifecycleState::Active;
        resumed.updated_at_ms = 3;
        store
            .persist_identity_and_enqueue_graph_projection(
                &resumed,
                &IdentityAuditEvent {
                    agent_id: definition.id.clone(),
                    agency: agency.clone(),
                    kind: IdentityEventKind::Resumed,
                    at_ms: 3,
                },
            )
            .await
            .expect("resume persist");

        let mut retired = resumed.clone();
        retired.state = AgentLifecycleState::Retired;
        retired.updated_at_ms = 4;
        store
            .persist_identity_and_enqueue_graph_projection(
                &retired,
                &IdentityAuditEvent {
                    agent_id: definition.id.clone(),
                    agency: agency.clone(),
                    kind: IdentityEventKind::Retired,
                    at_ms: 4,
                },
            )
            .await
            .expect("retire persist");

        let (agents, audit) = store.load_snapshot().await.expect("load");
        let loaded = agents
            .iter()
            .find(|agent| agent.id == definition.id)
            .expect("round-tripped agent row");
        assert!(loaded.capabilities.consult_jev);
        assert!(loaded.capabilities.promote_runtime_bot);
        assert_eq!(loaded.state, AgentLifecycleState::Retired);
        for kind in [
            IdentityEventKind::Registered,
            IdentityEventKind::Paused,
            IdentityEventKind::Resumed,
            IdentityEventKind::Retired,
        ] {
            assert!(
                audit
                    .iter()
                    .any(|event| event.agent_id == definition.id && event.kind == kind),
                "expected {:?} audit event for ceo-pg",
                kind
            );
        }
    }
}
