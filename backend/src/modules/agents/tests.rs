use crate::core::providers::JevReviewInput;
use crate::modules::agents::models::AgentDefinition;
use crate::modules::agents::{
    assert_advisory_eligible, pause_agent, resume_agent, retire_agent, validate_hierarchy,
    AgencyId, AgentCapabilities, AgentId, AgentLifecycleState, AgentRegistry, AgentRole,
    AgentsError, NewAgentSpec, OwnerId, SupervisorRef,
};

fn agency() -> AgencyId {
    AgencyId::new("acme").unwrap()
}

fn owner() -> OwnerId {
    OwnerId::new("owner-1").unwrap()
}

fn ceo_spec() -> NewAgentSpec {
    NewAgentSpec {
        id: AgentId::new("ceo").unwrap(),
        agency: agency(),
        owner: owner(),
        display_name: "Chief Executive".into(),
        role: AgentRole::Ceo,
        supervisor: SupervisorRef::Owner(owner()),
        capabilities: AgentCapabilities::default(),
    }
}

#[test]
fn registers_ceo_under_owner() {
    let mut registry = AgentRegistry::new();
    registry.register(ceo_spec(), 1).unwrap();
    let agent = registry
        .get(&agency(), &AgentId::new("ceo").unwrap())
        .unwrap();
    assert_eq!(agent.state, AgentLifecycleState::Active);
    assert_eq!(registry.audit_log().len(), 1);
}

#[test]
fn rejects_level_b_without_ceo() {
    let spec = NewAgentSpec {
        id: AgentId::new("ops").unwrap(),
        agency: agency(),
        owner: owner(),
        display_name: "Ops".into(),
        role: AgentRole::LevelB,
        supervisor: SupervisorRef::Agent(AgentId::new("ceo").unwrap()),
        capabilities: AgentCapabilities::default(),
    };
    let definition = AgentDefinition {
        id: spec.id.clone(),
        agency: spec.agency.clone(),
        owner: spec.owner.clone(),
        display_name: spec.display_name.clone(),
        role: spec.role,
        supervisor: spec.supervisor.clone(),
        state: AgentLifecycleState::Active,
        capabilities: spec.capabilities,
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    let err = validate_hierarchy(&definition, &[]).unwrap_err();
    assert_eq!(
        err,
        AgentsError::Hierarchy("supervisor agent ceo does not exist".into())
    );
}

#[test]
fn builds_hierarchy_chain() {
    let mut registry = AgentRegistry::new();
    registry.register(ceo_spec(), 1).unwrap();
    let level_b = NewAgentSpec {
        id: AgentId::new("level-b").unwrap(),
        agency: agency(),
        owner: owner(),
        display_name: "Level B".into(),
        role: AgentRole::LevelB,
        supervisor: SupervisorRef::Agent(AgentId::new("ceo").unwrap()),
        capabilities: AgentCapabilities::default(),
    };
    registry.register(level_b, 2).unwrap();
    assert_eq!(registry.list_agency(&agency()).len(), 2);
}

#[test]
fn lifecycle_pause_and_resume_are_idempotent() {
    let mut registry = AgentRegistry::new();
    registry.register(ceo_spec(), 1).unwrap();
    let id = AgentId::new("ceo").unwrap();
    pause_agent(&mut registry, &agency(), &id, 2).unwrap();
    pause_agent(&mut registry, &agency(), &id, 3).unwrap();
    assert_eq!(
        registry.get(&agency(), &id).unwrap().state,
        AgentLifecycleState::Paused
    );
    resume_agent(&mut registry, &agency(), &id, 4).unwrap();
    resume_agent(&mut registry, &agency(), &id, 5).unwrap();
    assert_eq!(
        registry.get(&agency(), &id).unwrap().state,
        AgentLifecycleState::Active
    );
}

#[test]
fn retired_agent_cannot_be_paused() {
    let mut registry = AgentRegistry::new();
    registry.register(ceo_spec(), 1).unwrap();
    let id = AgentId::new("ceo").unwrap();
    retire_agent(&mut registry, &agency(), &id, 2).unwrap();
    let err = pause_agent(&mut registry, &agency(), &id, 3).unwrap_err();
    assert!(matches!(err, AgentsError::Lifecycle(_)));
}

#[test]
fn advisory_denied_without_consult_jev() {
    let mut registry = AgentRegistry::new();
    registry.register(ceo_spec(), 1).unwrap();
    let id = AgentId::new("ceo").unwrap();
    let err = assert_advisory_eligible(&registry, &agency(), &id).unwrap_err();
    assert!(matches!(err, AgentsError::AdvisoryDenied(_)));
}

#[test]
fn advisory_allowed_when_capability_enabled() {
    let mut registry = AgentRegistry::new();
    let spec = NewAgentSpec {
        id: AgentId::new("analyst").unwrap(),
        agency: agency(),
        owner: owner(),
        display_name: "Analyst".into(),
        role: AgentRole::Ceo,
        supervisor: SupervisorRef::Owner(owner()),
        capabilities: AgentCapabilities { consult_jev: true },
    };
    registry.register(spec, 1).unwrap();
    let id = AgentId::new("analyst").unwrap();
    assert_advisory_eligible(&registry, &agency(), &id).unwrap();
}

#[test]
fn advisory_input_uses_jev_review_shape() {
    let input = JevReviewInput {
        signal_label: "Hold",
        close: 1.0,
        fast_sma: Some(1.1),
        slow_sma: Some(1.0),
        candle_timestamp_ms: 42,
    };
    assert_eq!(input.candle_timestamp_ms, 42);
}
