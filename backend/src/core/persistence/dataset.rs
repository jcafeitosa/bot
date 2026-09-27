//! Neutral row types for market dataset persistence (no `modules::*` imports).

#[derive(Debug, Clone)]
pub struct MarketDatasetManifestRow {
    pub dataset_id: String,
    pub symbol: String,
    pub base_timeframe: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub candle_count: usize,
    pub gap_count: usize,
    pub source: String,
}

#[derive(Debug, Clone, Copy)]
pub struct CandleRow {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct MarketDatasetPersistInput {
    pub manifest: MarketDatasetManifestRow,
    pub candles: Vec<CandleRow>,
}
