#![allow(dead_code)]
//! HTTP-facing facades so `presentation` does not import domain modules directly.

//! Facades consumed by the HTTP route layer (no direct domain imports in routes):
//! `agents`, `application`, `backtest`, `bots`, `config`, `exchanges`, `monitor`, `orders`,
//! `portfolio`, `providers`, `risk`, `strategy`.

pub mod agents;
pub mod application;
pub mod backtest;
pub mod bots;
pub mod config;
pub mod exchanges;
pub mod monitor;
pub mod orders;
pub mod portfolio;
pub mod providers;
pub mod risk;
pub mod strategy;

#[cfg(test)]
mod bridge_tests {
    use crate::core::config::Config;

    #[tokio::test]
    async fn persist_catalog_bridge_wires_store_seam() {
        use crate::modules::bots::InMemoryBotCatalogStore;
        let config = Config::default();
        let mut store = InMemoryBotCatalogStore::new();
        let out = super::bots::persist_catalog_for_config(&config, &mut store)
            .await
            .expect("persist");
        assert!(out.persisted);
        assert!(!out.bots.is_empty());
    }
}
