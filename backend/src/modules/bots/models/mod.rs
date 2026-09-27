mod definition;
mod error;
mod identity;
mod metrics;
mod monitor_evaluator;
mod ranking;
mod run;
mod runtime;

pub use definition::{BotDefinition, StrategySpec};
pub use error::BotsError;
pub use identity::{bot_id_matches_market, BotId, BotIdentity, StrategyId, StrategyVersion};
pub use metrics::{BotMetrics, EvaluationWindow};
pub use monitor_evaluator::MonitorEvaluatorKind;
pub use ranking::{rank_bot_metrics, BotRankEntry, BotRanking, BotRankingReport};
pub use run::RunId;
pub use runtime::{BotPromotionRecord, BotPromotionState, BotRuntimeStatus, PromoteBotRequest};
