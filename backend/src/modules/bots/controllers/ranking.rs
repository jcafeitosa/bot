use crate::modules::bots::models::{
    rank_bot_metrics, BotMetrics, BotRanking, BotRankingReport, BotsError,
};

pub fn full_ranking(
    rows: impl IntoIterator<Item = BotMetrics>,
) -> Result<BotRankingReport, BotsError> {
    rank_bot_metrics(rows)
}

pub fn rank_bots(rows: impl IntoIterator<Item = BotMetrics>) -> Result<BotRanking, BotsError> {
    full_ranking(rows).map(BotRanking::from)
}
