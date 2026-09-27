#![allow(unused_imports)] // public seam reexports; consumers may use `bots::` or `bots::models::`
//! Versioned trading bots as strategy×timeframe (and symbol) instances.
//!
//! Not [`crate::modules::agents`] (administrative identity) — bots are evaluated executors.

pub mod adapters;
pub mod controllers;
pub mod models;

pub use adapters::{
    apply_bot_runtime_to_monitor_snapshot, bot_runtime_from_env,
    enrich_monitor_snapshot_from_shared_runtime, shared_bot_runtime, BotCatalogBackend,
    BotCatalogStore, BotRuntimePort, FailClosedBotRuntime, InMemoryBotCatalogStore,
    InMemoryBotRuntime, NoopBotCatalogStore, PgBotCatalogStore,
};
pub use controllers::{
    build_bot_definition, build_catalog_from_config, build_catalog_from_monitor_registry,
    enumerate_timeframes_for_mode, full_ranking, monitor_strategy_from_config,
    persist_catalog_snapshot, persist_monitor_catalog_snapshot, rank_bots,
    strategy_evaluation_binding, strategy_evaluation_binding_with_runtime, MonitorStrategyRegistry,
    StrategyEvaluationBinding,
};
pub use models::{
    bot_id_matches_market, rank_bot_metrics, BotDefinition, BotId, BotIdentity, BotMetrics,
    BotPromotionRecord, BotPromotionState, BotRankEntry, BotRanking, BotRankingReport,
    BotRuntimeStatus, BotsError, EvaluationWindow, MonitorEvaluatorKind, PromoteBotRequest, RunId,
    StrategyId, StrategySpec, StrategyVersion,
};

#[cfg(test)]
mod tests;
