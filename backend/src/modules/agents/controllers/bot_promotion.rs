use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::models::{AgencyId, AgentId, AgentLifecycleState, AgentsError};
use crate::modules::bots::models::BotIdentity;

pub fn assert_runtime_promotion_authorized(
    registry: &AgentRegistry,
    agency: &AgencyId,
    promoted_by: &str,
    bot_id: &str,
) -> Result<(), AgentsError> {
    BotIdentity::parse_bot_id(bot_id).map_err(|e| AgentsError::InvalidId(e.to_string()))?;
    let agent_id = AgentId::new(promoted_by)?;
    let agent = registry.get(agency, &agent_id)?;
    if agent.state != AgentLifecycleState::Active {
        return Err(AgentsError::PromotionDenied(format!(
            "agent {agent_id} is not active"
        )));
    }
    if !agent.capabilities.promote_runtime_bot {
        return Err(AgentsError::PromotionDenied(format!(
            "agent {agent_id} does not have promote_runtime_bot capability"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::agents::controllers::pause_agent;
    use crate::modules::agents::models::{
        AgentCapabilities, AgentId, AgentRole, NewAgentSpec, OwnerId, SupervisorRef,
    };
    use crate::modules::agents::AgentRegistry;

    fn agency() -> AgencyId {
        AgencyId::new("agency-a").unwrap()
    }

    #[test]
    fn promotion_requires_active_agent_with_capability() {
        let mut registry = AgentRegistry::new();
        let agency = agency();
        registry
            .register(
                NewAgentSpec {
                    id: AgentId::new("promoter-1").unwrap(),
                    agency: agency.clone(),
                    owner: OwnerId::new("owner-1").unwrap(),
                    display_name: "Promoter CEO".into(),
                    role: AgentRole::Ceo,
                    supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
                    capabilities: AgentCapabilities {
                        consult_jev: false,
                        promote_runtime_bot: true,
                    },
                },
                0,
            )
            .unwrap();
        assert!(assert_runtime_promotion_authorized(
            &registry,
            &agency,
            "promoter-1",
            "sma-cross@1:5m:BTC/USDT"
        )
        .is_ok());
        assert!(assert_runtime_promotion_authorized(
            &registry,
            &agency,
            "unknown",
            "sma-cross@1:5m:BTC/USDT"
        )
        .is_err());
    }

    #[test]
    fn promotion_denied_when_capability_false() {
        let mut registry = AgentRegistry::new();
        let agency = agency();
        registry
            .register(
                NewAgentSpec {
                    id: AgentId::new("no-promote").unwrap(),
                    agency: agency.clone(),
                    owner: OwnerId::new("owner-1").unwrap(),
                    display_name: "CEO without promote".into(),
                    role: AgentRole::Ceo,
                    supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
                    capabilities: AgentCapabilities {
                        consult_jev: false,
                        promote_runtime_bot: false,
                    },
                },
                0,
            )
            .unwrap();
        let err = assert_runtime_promotion_authorized(
            &registry,
            &agency,
            "no-promote",
            "sma-cross@1:15m:BTCUSDT",
        )
        .unwrap_err();
        assert!(matches!(err, AgentsError::PromotionDenied(_)));
    }

    #[test]
    fn promotion_rejects_invalid_bot_id() {
        let registry = AgentRegistry::new();
        let agency = agency();
        let err = assert_runtime_promotion_authorized(
            &registry,
            &agency,
            "promoter-1",
            "not-a-valid-bot-id",
        )
        .unwrap_err();
        assert!(matches!(err, AgentsError::InvalidId(_)));
    }

    #[test]
    fn promotion_denied_when_agent_not_active() {
        let mut registry = AgentRegistry::new();
        let agency = agency();
        let promoter = AgentId::new("promoter-1").unwrap();
        registry
            .register(
                NewAgentSpec {
                    id: promoter.clone(),
                    agency: agency.clone(),
                    owner: OwnerId::new("owner-1").unwrap(),
                    display_name: "Promoter CEO".into(),
                    role: AgentRole::Ceo,
                    supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
                    capabilities: AgentCapabilities {
                        consult_jev: false,
                        promote_runtime_bot: true,
                    },
                },
                0,
            )
            .unwrap();
        pause_agent(&mut registry, &agency, &promoter, 1).expect("pause");
        let err = assert_runtime_promotion_authorized(
            &registry,
            &agency,
            "promoter-1",
            "sma-cross@1:15m:BTCUSDT",
        )
        .unwrap_err();
        assert!(matches!(err, AgentsError::PromotionDenied(_)));
    }
}
