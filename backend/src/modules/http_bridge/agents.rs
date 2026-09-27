use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::database::PostgresDatabase;
use crate::core::error::BotResult;
use crate::core::providers::{JevAdvisor, JevReviewInput};
use crate::modules::agents::adapters::jev::delegate_jev_review;
use crate::modules::agents::adapters::{AgentIdentityStore, PgAgentIdentityStore};

use crate::modules::agents::{
    assert_advisory_eligible, pause_agent, resume_agent, retire_agent, AdvisoryStepInput, AgencyId,
    AgentCapabilities, AgentId, AgentRegistry, AgentRole, AgentsError, NewAgentSpec, OwnerId,
    SupervisorRef,
};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AgencyQuery {
    pub agency: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SupervisorRefBody {
    Owner { owner_id: String },
    Agent { agent_id: String },
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterAgentRequest {
    pub agency: String,
    pub owner_id: String,
    pub agent_id: String,
    pub display_name: String,
    pub role: AgentRoleBody,
    pub supervisor: SupervisorRefBody,
    #[serde(default)]
    pub consult_jev: bool,
    #[serde(default)]
    pub promote_runtime_bot: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentRoleBody {
    Ceo,
    LevelB,
    LevelA,
    Specialist,
    Worker,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentResponse {
    pub agency: String,
    pub agent_id: String,
    pub owner_id: String,
    pub display_name: String,
    pub role: String,
    pub state: String,
    pub consult_jev: bool,
    pub promote_runtime_bot: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventResponse {
    pub agent_id: String,
    pub agency: String,
    pub kind: String,
    pub at_ms: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub events: Vec<AuditEventResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdvisoryRequest {
    pub agency: String,
    pub signal: String,
    pub close: f64,
    #[serde(default)]
    pub fast_sma: Option<f64>,
    #[serde(default)]
    pub slow_sma: Option<f64>,
    pub candle_timestamp_ms: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdvisoryResponse {
    pub agent_id: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LifecycleResponse {
    pub agent_id: String,
    pub state: String,
}

impl From<AgentRoleBody> for AgentRole {
    fn from(value: AgentRoleBody) -> Self {
        match value {
            AgentRoleBody::Ceo => AgentRole::Ceo,
            AgentRoleBody::LevelB => AgentRole::LevelB,
            AgentRoleBody::LevelA => AgentRole::LevelA,
            AgentRoleBody::Specialist => AgentRole::Specialist,
            AgentRoleBody::Worker => AgentRole::Worker,
        }
    }
}

fn parse_agency(raw: &str) -> Result<AgencyId, AgentsError> {
    AgencyId::new(raw)
}

fn parse_agent(raw: &str) -> Result<AgentId, AgentsError> {
    AgentId::new(raw)
}

fn parse_owner(raw: &str) -> Result<OwnerId, AgentsError> {
    OwnerId::new(raw)
}

fn parse_supervisor(body: SupervisorRefBody) -> Result<SupervisorRef, AgentsError> {
    match body {
        SupervisorRefBody::Owner { owner_id } => Ok(SupervisorRef::Owner(parse_owner(&owner_id)?)),
        SupervisorRefBody::Agent { agent_id } => Ok(SupervisorRef::Agent(parse_agent(&agent_id)?)),
    }
}

fn map_agent(def: &crate::modules::agents::AgentDefinition) -> AgentResponse {
    AgentResponse {
        agency: def.agency.to_string(),
        agent_id: def.id.to_string(),
        owner_id: def.owner.to_string(),
        display_name: def.display_name.clone(),
        role: def.role.to_string(),
        state: format!("{:?}", def.state),
        consult_jev: def.capabilities.consult_jev,
        promote_runtime_bot: def.capabilities.promote_runtime_bot,
        created_at_ms: def.created_at_ms,
        updated_at_ms: def.updated_at_ms,
    }
}

pub fn register_agent(
    registry: &mut AgentRegistry,
    body: RegisterAgentRequest,
) -> Result<AgentResponse, AgentsError> {
    let spec = NewAgentSpec {
        id: parse_agent(&body.agent_id)?,
        agency: parse_agency(&body.agency)?,
        owner: parse_owner(&body.owner_id)?,
        display_name: body.display_name,
        role: body.role.into(),
        supervisor: parse_supervisor(body.supervisor)?,
        capabilities: AgentCapabilities {
            consult_jev: body.consult_jev,
            promote_runtime_bot: body.promote_runtime_bot,
        },
    };
    let at_ms = Utc::now().timestamp_millis();
    let agency_key = body.agency.clone();
    let agent_key = body.agent_id.clone();
    registry.register(spec, at_ms)?;
    let agency = parse_agency(&agency_key)?;
    let id = parse_agent(&agent_key)?;
    Ok(map_agent(registry.get(&agency, &id)?))
}

pub fn list_agents(
    registry: &AgentRegistry,
    agency_raw: &str,
) -> Result<AgentListResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let agents = registry
        .list_agency(&agency)
        .into_iter()
        .map(map_agent)
        .collect();
    Ok(AgentListResponse { agents })
}

pub fn get_agent(
    registry: &AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<AgentResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let id = parse_agent(agent_raw)?;
    Ok(map_agent(registry.get(&agency, &id)?))
}

pub fn audit_log(
    registry: &AgentRegistry,
    agency_raw: &str,
) -> Result<AuditLogResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let events = registry
        .audit_log()
        .iter()
        .filter(|event| event.agency == agency)
        .map(|event| AuditEventResponse {
            agent_id: event.agent_id.to_string(),
            agency: event.agency.to_string(),
            kind: format!("{:?}", event.kind),
            at_ms: event.at_ms,
        })
        .collect();
    Ok(AuditLogResponse { events })
}

pub fn pause(
    registry: &mut AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<LifecycleResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let id = parse_agent(agent_raw)?;
    pause_agent(registry, &agency, &id, Utc::now().timestamp_millis())?;
    Ok(LifecycleResponse {
        agent_id: id.to_string(),
        state: format!("{:?}", registry.get(&agency, &id)?.state),
    })
}

pub fn resume(
    registry: &mut AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<LifecycleResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let id = parse_agent(agent_raw)?;
    resume_agent(registry, &agency, &id, Utc::now().timestamp_millis())?;
    Ok(LifecycleResponse {
        agent_id: id.to_string(),
        state: format!("{:?}", registry.get(&agency, &id)?.state),
    })
}

pub fn retire(
    registry: &mut AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<LifecycleResponse, AgentsError> {
    let agency = parse_agency(agency_raw)?;
    let id = parse_agent(agent_raw)?;
    retire_agent(registry, &agency, &id, Utc::now().timestamp_millis())?;
    Ok(LifecycleResponse {
        agent_id: id.to_string(),
        state: format!("{:?}", registry.get(&agency, &id)?.state),
    })
}

pub fn advisory_prepare(
    registry: &AgentRegistry,
    agent_raw: &str,
    body: AdvisoryRequest,
) -> Result<AdvisoryStepInput, AgentsError> {
    let agency = parse_agency(&body.agency)?;
    let agent_id = parse_agent(agent_raw)?;
    assert_advisory_eligible(registry, &agency, &agent_id)?;
    let signal_label: &'static str = match body.signal.as_str() {
        "warmup" => "warmup",
        "hold" => "hold",
        "buy" => "buy",
        "sell" => "sell",
        other => {
            return Err(AgentsError::InvalidId(format!(
                "invalid signal label: {other}"
            )))
        }
    };
    Ok(AdvisoryStepInput {
        agent_id,
        review: JevReviewInput {
            signal_label,
            close: body.close,
            fast_sma: body.fast_sma,
            slow_sma: body.slow_sma,
            candle_timestamp_ms: body.candle_timestamp_ms,
        },
    })
}

pub async fn advisory_finish(
    advisor: &JevAdvisor,
    input: AdvisoryStepInput,
) -> BotResult<AdvisoryResponse> {
    let lines = delegate_jev_review(advisor, &input.review).await?;
    Ok(AdvisoryResponse {
        agent_id: input.agent_id.to_string(),
        lines,
    })
}

pub fn snapshot_for_persist(
    registry: &AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<
    (
        crate::modules::agents::AgentDefinition,
        crate::modules::agents::IdentityAuditEvent,
    ),
    AgentsError,
> {
    let agency = parse_agency(agency_raw)?;
    let id = parse_agent(agent_raw)?;
    let definition = registry.get(&agency, &id)?.clone();
    let event = registry
        .audit_log()
        .iter()
        .rev()
        .find(|event| event.agent_id == id && event.agency == agency)
        .cloned()
        .ok_or_else(|| AgentsError::Persistence("missing audit event for agent".into()))?;
    Ok((definition, event))
}

pub async fn persist_identity_rows(
    postgres: &PostgresDatabase,
    definition: &crate::modules::agents::AgentDefinition,
    event: &crate::modules::agents::IdentityAuditEvent,
) -> Result<(), AgentsError> {
    let store = PgAgentIdentityStore::new(postgres);
    store
        .upsert_agent(definition)
        .await
        .map_err(AgentsError::Persistence)?;
    store
        .append_event(event)
        .await
        .map_err(AgentsError::Persistence)?;
    Ok(())
}

pub async fn write_through_agent_identity(
    registry: &AgentRegistry,
    postgres: Option<&PostgresDatabase>,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<(), AgentsError> {
    let Some(postgres) = postgres else {
        return Ok(());
    };
    let (definition, event) = snapshot_for_persist(registry, agency_raw, agent_raw)?;
    persist_identity_rows(postgres, &definition, &event).await
}

pub async fn load_agent_identity_snapshot(
    postgres: &PostgresDatabase,
) -> Result<
    (
        Vec<crate::modules::agents::AgentDefinition>,
        Vec<crate::modules::agents::IdentityAuditEvent>,
    ),
    AgentsError,
> {
    let store = PgAgentIdentityStore::new(postgres);
    store
        .load_snapshot()
        .await
        .map_err(AgentsError::Persistence)
}

pub fn apply_agent_identity_snapshot(
    registry: &mut AgentRegistry,
    agents: Vec<crate::modules::agents::AgentDefinition>,
    audit: Vec<crate::modules::agents::IdentityAuditEvent>,
) -> Result<(), AgentsError> {
    if agents.is_empty() || !registry.is_empty() {
        return Ok(());
    }
    registry.restore_from_snapshot(agents, audit)?;
    Ok(())
}

#[cfg(test)]
mod apply_snapshot_tests {
    use super::*;
    use crate::modules::agents::{
        AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState, AgentRegistry,
        AgentRole, NewAgentSpec, OwnerId, SupervisorRef,
    };

    fn agency() -> AgencyId {
        AgencyId::new("acme").unwrap()
    }

    fn owner() -> OwnerId {
        OwnerId::new("owner-1").unwrap()
    }

    #[test]
    fn apply_snapshot_noops_on_empty_agents() {
        let mut registry = AgentRegistry::new();
        assert!(apply_agent_identity_snapshot(&mut registry, vec![], vec![]).is_ok());
        assert!(registry.is_empty());
    }

    #[test]
    fn apply_snapshot_skips_when_registry_not_empty() {
        let mut registry = AgentRegistry::new();
        let spec = NewAgentSpec {
            id: AgentId::new("ceo").unwrap(),
            agency: agency(),
            owner: owner(),
            display_name: "CEO".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(owner()),
            capabilities: AgentCapabilities::default(),
        };
        registry.register(spec, 1).unwrap();
        let from_pg = AgentDefinition {
            id: AgentId::new("from-pg").unwrap(),
            agency: agency(),
            owner: owner(),
            display_name: "PG".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(owner()),
            state: AgentLifecycleState::Active,
            capabilities: AgentCapabilities::default(),
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        assert!(apply_agent_identity_snapshot(&mut registry, vec![from_pg], vec![]).is_ok());
        assert!(registry
            .get(&agency(), &AgentId::new("from-pg").unwrap())
            .is_err());
    }
}

#[cfg(test)]
mod register_tests {
    use super::*;
    use crate::modules::agents::AgentRegistry;

    #[test]
    fn register_agent_maps_promote_runtime_bot_capability() {
        let mut registry = AgentRegistry::new();
        let response = register_agent(
            &mut registry,
            RegisterAgentRequest {
                agency: "acme".into(),
                owner_id: "owner-1".into(),
                agent_id: "promoter".into(),
                display_name: "Promoter".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-1".into(),
                },
                consult_jev: false,
                promote_runtime_bot: true,
            },
        )
        .expect("register");
        assert!(response.promote_runtime_bot);
    }
}

#[cfg(test)]
mod pg_write_through_tests {
    use super::*;
    use crate::modules::agents::models::IdentityEventKind;
    use crate::modules::agents::{AgentLifecycleState, AgentRegistry};

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL 18+ database trading_bot with migrations applied"]
    async fn pg_agent_lifecycle_write_through_round_trip() {
        use crate::core::persistence::Database;

        let db = Database::connect_from_env().await.expect("DATABASE_URL");
        db.migrate().await.expect("migrate");
        let postgres = db.as_postgres();

        let mut registry = AgentRegistry::new();
        register_agent(
            &mut registry,
            RegisterAgentRequest {
                agency: "agency-wt".into(),
                owner_id: "owner-wt".into(),
                agent_id: "ceo-wt".into(),
                display_name: "CEO".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-wt".into(),
                },
                consult_jev: false,
                promote_runtime_bot: true,
            },
        )
        .expect("register");
        write_through_agent_identity(&registry, Some(postgres), "agency-wt", "ceo-wt")
            .await
            .expect("write-through register");

        pause(&mut registry, "agency-wt", "ceo-wt").expect("pause");
        write_through_agent_identity(&registry, Some(postgres), "agency-wt", "ceo-wt")
            .await
            .expect("write-through pause");

        let (agents, audit) = load_agent_identity_snapshot(postgres)
            .await
            .expect("load snapshot");
        let loaded = agents
            .iter()
            .find(|agent| agent.id.as_str() == "ceo-wt")
            .expect("agent row from PG");
        assert_eq!(loaded.state, AgentLifecycleState::Paused);
        assert!(loaded.capabilities.promote_runtime_bot);
        for kind in [IdentityEventKind::Registered, IdentityEventKind::Paused] {
            assert!(
                audit
                    .iter()
                    .any(|event| event.agent_id.as_str() == "ceo-wt" && event.kind == kind),
                "expected {:?} in PG audit",
                kind
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL 18+ database trading_bot with migrations applied"]
    async fn pg_cold_start_apply_snapshot_after_write_through() {
        use crate::core::persistence::Database;

        let db = Database::connect_from_env().await.expect("DATABASE_URL");
        db.migrate().await.expect("migrate");
        let postgres = db.as_postgres();

        let mut registry = AgentRegistry::new();
        register_agent(
            &mut registry,
            RegisterAgentRequest {
                agency: "agency-cold".into(),
                owner_id: "owner-cold".into(),
                agent_id: "ceo-cold".into(),
                display_name: "CEO".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-cold".into(),
                },
                consult_jev: true,
                promote_runtime_bot: false,
            },
        )
        .expect("register");
        write_through_agent_identity(&registry, Some(postgres), "agency-cold", "ceo-cold")
            .await
            .expect("write-through");

        let (agents, audit) = load_agent_identity_snapshot(postgres).await.expect("load");
        let mut cold_registry = AgentRegistry::new();
        apply_agent_identity_snapshot(&mut cold_registry, agents, audit).expect("hydrate");
        let listed = list_agents(&cold_registry, "agency-cold").expect("list");
        assert_eq!(listed.agents.len(), 1);
        assert_eq!(listed.agents[0].agent_id, "ceo-cold");
        assert!(listed.agents[0].consult_jev);
        assert_eq!(listed.agents[0].state, "Active");
    }
}
