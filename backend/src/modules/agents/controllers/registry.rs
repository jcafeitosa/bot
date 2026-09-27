#![allow(dead_code)]
use std::collections::HashMap;

use crate::modules::agents::models::{
    validate_hierarchy, AgencyId, AgentDefinition, AgentId, AgentLifecycleState, AgentsError,
    IdentityAuditEvent, IdentityEventKind, NewAgentSpec,
};

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<AgentId, AgentDefinition>,
    audit: Vec<IdentityAuditEvent>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: NewAgentSpec, at_ms: i64) -> Result<(), AgentsError> {
        if self.agents.contains_key(&spec.id) {
            return Err(AgentsError::Duplicate(spec.id.to_string()));
        }
        let definition = AgentDefinition {
            id: spec.id.clone(),
            agency: spec.agency.clone(),
            owner: spec.owner,
            display_name: spec.display_name,
            role: spec.role,
            supervisor: spec.supervisor,
            state: AgentLifecycleState::Active,
            capabilities: spec.capabilities,
            created_at_ms: at_ms,
            updated_at_ms: at_ms,
        };
        let existing: Vec<AgentDefinition> = self.agents.values().cloned().collect();
        validate_hierarchy(&definition, &existing)?;
        self.agents.insert(definition.id.clone(), definition);
        self.audit.push(IdentityAuditEvent {
            agent_id: spec.id,
            agency: spec.agency,
            kind: IdentityEventKind::Registered,
            at_ms,
        });
        Ok(())
    }

    pub fn get(&self, agency: &AgencyId, id: &AgentId) -> Result<&AgentDefinition, AgentsError> {
        let agent = self
            .agents
            .get(id)
            .ok_or_else(|| AgentsError::NotFound(id.to_string()))?;
        if &agent.agency != agency {
            return Err(AgentsError::AgencyMismatch {
                agent: id.to_string(),
            });
        }
        Ok(agent)
    }

    pub fn list_agency(&self, agency: &AgencyId) -> Vec<&AgentDefinition> {
        self.agents
            .values()
            .filter(|a| &a.agency == agency)
            .collect()
    }

    pub fn audit_log(&self) -> &[IdentityAuditEvent] {
        &self.audit
    }

    pub(crate) fn update_agent(
        &mut self,
        id: &AgentId,
        at_ms: i64,
        mutator: impl FnOnce(&mut AgentDefinition),
        event: IdentityEventKind,
    ) -> Result<(), AgentsError> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| AgentsError::NotFound(id.to_string()))?;
        mutator(agent);
        agent.updated_at_ms = at_ms;
        let agency = agent.agency.clone();
        let agent_id = agent.id.clone();
        self.audit.push(IdentityAuditEvent {
            agent_id,
            agency,
            kind: event,
            at_ms,
        });
        Ok(())
    }
}
