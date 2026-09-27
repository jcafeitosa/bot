use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::error::BotResult;
use crate::core::providers::{JevAdvisor, JevReviewInput};
use crate::modules::agents::adapters::jev::delegate_jev_review;
use crate::core::database::PostgresDatabase;
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

pub async fn persist_agent_snapshot(
    postgres: &PostgresDatabase,
    registry: &AgentRegistry,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<(), AgentsError> {
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
    let store = PgAgentIdentityStore::new(postgres);
    store
        .upsert_agent(&definition)
        .await
        .map_err(AgentsError::Persistence)?;
    store
        .append_event(&event)
        .await
        .map_err(AgentsError::Persistence)?;
    Ok(())
}

pub async fn persist_if_postgres(
    registry: &AgentRegistry,
    postgres: Option<&PostgresDatabase>,
    agency_raw: &str,
    agent_raw: &str,
) -> Result<(), AgentsError> {
    if let Some(pg) = postgres {
        persist_agent_snapshot(pg, registry, agency_raw, agent_raw).await?;
    }
    Ok(())
}
