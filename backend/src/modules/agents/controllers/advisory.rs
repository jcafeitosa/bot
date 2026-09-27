use crate::core::error::{BotError, BotResult};
use crate::core::providers::{JevAdvisor, JevReviewInput};
use crate::modules::agents::adapters::jev::delegate_jev_review;
use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::models::{AgencyId, AgentId, AgentLifecycleState, AgentsError};

#[derive(Debug, Clone)]
pub struct AdvisoryStepInput {
    pub agent_id: AgentId,
    pub review: JevReviewInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryStepResult {
    pub agent_id: AgentId,
    pub lines: Vec<String>,
}

pub fn assert_advisory_eligible(
    registry: &AgentRegistry,
    agency: &AgencyId,
    agent_id: &AgentId,
) -> Result<(), AgentsError> {
    let agent = registry.get(agency, agent_id)?;
    if agent.state != AgentLifecycleState::Active {
        return Err(AgentsError::AdvisoryDenied(format!(
            "agent {agent_id} is not active"
        )));
    }
    if !agent.capabilities.consult_jev {
        return Err(AgentsError::AdvisoryDenied(format!(
            "agent {agent_id} does not have consult_jev capability"
        )));
    }
    Ok(())
}

pub async fn run_advisory_step(
    registry: &AgentRegistry,
    agency: &AgencyId,
    advisor: &JevAdvisor,
    input: AdvisoryStepInput,
) -> BotResult<AdvisoryStepResult> {
    assert_advisory_eligible(registry, agency, &input.agent_id).map_err(map_agents_error)?;
    let lines = delegate_jev_review(advisor, &input.review).await?;
    Ok(AdvisoryStepResult {
        agent_id: input.agent_id,
        lines,
    })
}

fn map_agents_error(err: AgentsError) -> BotError {
    match err {
        AgentsError::AdvisoryDenied(msg) => BotError::Jev(msg),
        other => BotError::Configuration(other.to_string()),
    }
}
