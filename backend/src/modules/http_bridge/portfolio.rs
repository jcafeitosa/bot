use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::error::BotError;
use crate::modules::portfolio::{paper_snapshot, Asset};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PaperSnapshotQuery {
    #[param(example = "usdt")]
    pub quote: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaperSnapshotResponse {
    pub quote: String,
    pub as_of_ms: i64,
    pub available: String,
    pub locked: String,
}

pub fn paper_wallet_snapshot(query: PaperSnapshotQuery) -> Result<PaperSnapshotResponse, BotError> {
    let asset =
        Asset::new(query.quote).map_err(|error| BotError::Configuration(error.to_string()))?;
    let snapshot = paper_snapshot(&asset);
    let balance = snapshot
        .balances
        .first()
        .ok_or_else(|| BotError::Configuration("missing balance in paper snapshot".into()))?;
    Ok(PaperSnapshotResponse {
        quote: asset.as_str().to_owned(),
        as_of_ms: snapshot.as_of_ms,
        available: balance.available.to_string(),
        locked: balance.locked.to_string(),
    })
}
