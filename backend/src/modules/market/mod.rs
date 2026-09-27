//! Market domain: OHLCV models and hybrid live feed orchestration.

pub mod adapters;
pub mod controllers;
pub mod models;

pub use adapters::persist_historical_dataset;

pub use controllers::{ws_matches_configured_timeframe, HybridCandleFeed};
#[allow(unused_imports)]
pub use models::{
    Candle, DatasetManifest, HistoricalDataset, MarketError, Timeframe, BASE_TIMEFRAME_MS,
};
