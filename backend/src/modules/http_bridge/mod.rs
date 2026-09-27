#![allow(dead_code)]
//! HTTP-facing facades so `presentation` does not import domain modules directly.

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

    #[test]
    fn persist_catalog_bridge_wires_store_seam() {
        let config = Config::default();
        let out = super::bots::persist_catalog_for_config(&config).expect("persist");
        assert!(out.persisted);
        assert!(!out.bots.is_empty());
    }
}
