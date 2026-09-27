#![allow(dead_code)]
use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::models::{
    AgencyId, AgentId, AgentLifecycleState, AgentsError, IdentityEventKind,
};

pub fn pause_agent(
    registry: &mut AgentRegistry,
    agency: &AgencyId,
    id: &AgentId,
    at_ms: i64,
) -> Result<(), AgentsError> {
    let current = registry.get(agency, id)?;
    if current.state == AgentLifecycleState::Retired {
        return Err(AgentsError::Lifecycle(
            "cannot pause a retired agent".into(),
        ));
    }
    if current.state == AgentLifecycleState::Paused {
        return Ok(());
    }
    registry.update_agent(
        id,
        at_ms,
        |a| a.state = AgentLifecycleState::Paused,
        IdentityEventKind::Paused,
    )
}

pub fn resume_agent(
    registry: &mut AgentRegistry,
    agency: &AgencyId,
    id: &AgentId,
    at_ms: i64,
) -> Result<(), AgentsError> {
    let current = registry.get(agency, id)?;
    if current.state == AgentLifecycleState::Retired {
        return Err(AgentsError::Lifecycle(
            "cannot resume a retired agent".into(),
        ));
    }
    if current.state == AgentLifecycleState::Active {
        return Ok(());
    }
    registry.update_agent(
        id,
        at_ms,
        |a| a.state = AgentLifecycleState::Active,
        IdentityEventKind::Resumed,
    )
}

pub fn retire_agent(
    registry: &mut AgentRegistry,
    agency: &AgencyId,
    id: &AgentId,
    at_ms: i64,
) -> Result<(), AgentsError> {
    let current = registry.get(agency, id)?;
    if current.state == AgentLifecycleState::Retired {
        return Ok(());
    }
    registry.update_agent(
        id,
        at_ms,
        |a| a.state = AgentLifecycleState::Retired,
        IdentityEventKind::Retired,
    )
}
