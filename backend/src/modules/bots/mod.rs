#![allow(unused_imports)] // public seam reexports; consumers may use `bots::` or `bots::models::`
//! Versioned trading bots as strategy×timeframe (and symbol) instances.
//!
//! Not [`crate::modules::agents`] (administrative identity) — bots are evaluated executors.

pub mod adapters;
pub mod controllers;
pub mod models;

pub use adapters::{BotCatalogStore, InMemoryBotCatalogStore, NoopBotCatalogStore};
pub use controllers::{
    build_bot_definition, build_catalog_from_config, enumerate_timeframes_for_mode, full_ranking,
    persist_catalog_snapshot, rank_bots,
};
pub use models::{
    rank_bot_metrics, BotDefinition, BotId, BotIdentity, BotMetrics, BotRankEntry, BotRanking,
    BotRankingReport, BotsError, EvaluationWindow, RunId, StrategyId, StrategySpec,
    StrategyVersion,
};

#[cfg(test)]
mod tests;
