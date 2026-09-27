use std::collections::HashSet;
use std::fmt;

use super::definition::AgentDefinition;
use super::identity::{AgentId, AgentsError, OwnerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Ceo,
    LevelB,
    LevelA,
    Specialist,
    Worker,
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Ceo => "ceo",
            Self::LevelB => "level_b",
            Self::LevelA => "level_a",
            Self::Specialist => "specialist",
            Self::Worker => "worker",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorRef {
    Owner(OwnerId),
    Agent(AgentId),
}

impl AgentRole {
    pub fn required_supervisor_role(self) -> Option<AgentRole> {
        match self {
            AgentRole::Ceo => None,
            AgentRole::LevelB => Some(AgentRole::Ceo),
            AgentRole::LevelA => Some(AgentRole::LevelB),
            AgentRole::Specialist | AgentRole::Worker => Some(AgentRole::LevelA),
        }
    }
}

pub fn validate_hierarchy(
    definition: &AgentDefinition,
    existing: &[AgentDefinition],
) -> Result<(), AgentsError> {
    match (&definition.role, &definition.supervisor) {
        (AgentRole::Ceo, SupervisorRef::Owner(_)) => {}
        (AgentRole::Ceo, SupervisorRef::Agent(_)) => {
            return Err(AgentsError::Hierarchy(
                "CEO must report to the human owner".into(),
            ));
        }
        (role, SupervisorRef::Owner(_)) => {
            return Err(AgentsError::Hierarchy(format!(
                "{role} must report to an agent supervisor, not the owner directly"
            )));
        }
        (role, SupervisorRef::Agent(supervisor_id)) => {
            let expected = role
                .required_supervisor_role()
                .expect("non-CEO roles always require a supervisor role");
            let supervisor = existing
                .iter()
                .find(|a| &a.id == supervisor_id)
                .ok_or_else(|| {
                    AgentsError::Hierarchy(format!(
                        "supervisor agent {supervisor_id} does not exist"
                    ))
                })?;
            if supervisor.agency != definition.agency {
                return Err(AgentsError::Hierarchy(
                    "supervisor must belong to the same agency".into(),
                ));
            }
            if supervisor.role != expected {
                return Err(AgentsError::Hierarchy(format!(
                    "{role} requires supervisor role {expected}, found {supervisor_role}",
                    supervisor_role = supervisor.role
                )));
            }
        }
    }

    if has_supervisor_cycle(definition, existing) {
        return Err(AgentsError::Hierarchy(
            "supervisor chain would form a cycle".into(),
        ));
    }
    Ok(())
}

fn has_supervisor_cycle(definition: &AgentDefinition, existing: &[AgentDefinition]) -> bool {
    let mut visited = HashSet::new();
    let mut current: Option<&AgentId> = match &definition.supervisor {
        SupervisorRef::Owner(_) => return false,
        SupervisorRef::Agent(id) => Some(id),
    };
    while let Some(id) = current {
        if id == &definition.id {
            return true;
        }
        if !visited.insert(id.clone()) {
            return true;
        }
        let next = existing.iter().find(|a| &a.id == id);
        current = match next {
            None => return false,
            Some(agent) => match &agent.supervisor {
                SupervisorRef::Owner(_) => None,
                SupervisorRef::Agent(parent) => Some(parent),
            },
        };
    }
    false
}
