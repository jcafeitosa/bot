use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::error::BotsError;
use super::metrics::{BotMetrics, EvaluationWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRankEntry {
    pub rank: usize,
    pub metrics: BotMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRankingReport {
    pub window: EvaluationWindow,
    pub dataset_hash: String,
    pub quote_currency: String,
    pub entries: Vec<BotRankEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRanking {
    pub rows: Vec<BotMetrics>,
}

impl From<BotRankingReport> for BotRanking {
    fn from(report: BotRankingReport) -> Self {
        Self {
            rows: report
                .entries
                .into_iter()
                .map(|entry| entry.metrics)
                .collect(),
        }
    }
}

pub fn rank_bot_metrics(
    rows: impl IntoIterator<Item = BotMetrics>,
) -> Result<BotRankingReport, BotsError> {
    let mut unique = BTreeMap::new();
    let mut scope: Option<(EvaluationWindow, String, String)> = None;
    for row in rows {
        row.validate()?;
        let row_scope = (
            row.window.clone(),
            row.quote_currency.to_ascii_uppercase(),
            row.dataset_hash.clone(),
        );
        if scope.as_ref().is_some_and(|current| current != &row_scope) {
            return Err(BotsError::IncompatibleRanking);
        }
        scope = Some(row_scope);
        let key = (
            row.bot_id.clone(),
            row.window.clone(),
            row.dataset_hash.clone(),
        );
        if unique.insert(key, row).is_some() {
            return Err(BotsError::DuplicateRun);
        }
    }
    let mut rows: Vec<_> = unique.into_values().collect();
    rows.sort_by(|a, b| {
        b.net_pnl_quote
            .total_cmp(&a.net_pnl_quote)
            .then_with(|| a.max_drawdown_pct.total_cmp(&b.max_drawdown_pct))
            .then_with(|| a.bot_id.cmp(&b.bot_id))
    });
    let (window, quote_currency, dataset_hash) = scope.ok_or(BotsError::IncompatibleRanking)?;
    let entries = rows
        .into_iter()
        .enumerate()
        .map(|(idx, metrics)| BotRankEntry {
            rank: idx + 1,
            metrics,
        })
        .collect();
    Ok(BotRankingReport {
        window,
        dataset_hash,
        quote_currency,
        entries,
    })
}
