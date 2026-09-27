use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ExchangeAccountId, ExchangeId, MarketType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    RestConnection,
    StreamConnection,
    OrderGateway,
    MarketDataSubscription,
    UserDataSubscription,
    RateLimitBudget,
    CredentialVaultRef,
    ReconciliationCursor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedResource {
    pub account: ExchangeAccountId,
    pub kind: ResourceKind,
    pub name: String,
    pub enabled: bool,
}

impl ManagedResource {
    pub fn key(&self) -> String {
        format!("{}:{:?}:{}", self.account.key(), self.kind, self.name)
    }
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    resources: BTreeMap<String, ManagedResource>,
}

impl ResourceManager {
    pub fn upsert(&mut self, resource: ManagedResource) {
        self.resources.insert(resource.key(), resource);
    }

    pub fn for_account(&self, account: &ExchangeAccountId) -> Vec<&ManagedResource> {
        self.resources
            .values()
            .filter(|resource| &resource.account == account)
            .collect()
    }

    pub fn defaults_for_account(account: &ExchangeAccountId) -> Vec<ManagedResource> {
        [
            ResourceKind::RestConnection,
            ResourceKind::StreamConnection,
            ResourceKind::OrderGateway,
            ResourceKind::MarketDataSubscription,
            ResourceKind::UserDataSubscription,
            ResourceKind::RateLimitBudget,
            ResourceKind::CredentialVaultRef,
            ResourceKind::ReconciliationCursor,
        ]
        .into_iter()
        .map(|kind| ManagedResource {
            account: account.clone(),
            kind,
            name: match kind {
                ResourceKind::RestConnection => "rest-primary",
                ResourceKind::StreamConnection => "stream-primary",
                ResourceKind::OrderGateway => "order-gateway",
                ResourceKind::MarketDataSubscription => "market-subscriptions",
                ResourceKind::UserDataSubscription => "user-data-subscription",
                ResourceKind::RateLimitBudget => "rate-limit-budget",
                ResourceKind::CredentialVaultRef => "credential-vault-ref",
                ResourceKind::ReconciliationCursor => "reconciliation-cursor",
            }
            .to_owned(),
            enabled: !matches!(kind, ResourceKind::OrderGateway),
        })
        .collect()
    }

    pub fn supported_exchanges() -> Vec<ExchangeId> {
        vec![ExchangeId::Binance]
    }

    pub fn supported_markets() -> Vec<MarketType> {
        vec![MarketType::Spot, MarketType::Futures]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;

    #[test]
    fn every_account_gets_all_managed_resources_with_orders_disabled() {
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Futures,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        let mut manager = ResourceManager::default();
        for resource in ResourceManager::defaults_for_account(&account) {
            manager.upsert(resource);
        }
        let resources = manager.for_account(&account);
        assert_eq!(resources.len(), 8);
        let gateway = resources
            .iter()
            .find(|resource| resource.kind == ResourceKind::OrderGateway)
            .unwrap();
        assert!(!gateway.enabled);
    }
}
