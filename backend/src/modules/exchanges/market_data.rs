use crate::core::error::BotResult;
use async_trait::async_trait;
use mantis_ta::types::Candle;

#[async_trait]
pub trait MarketDataSource: Send + Sync {
    async fn candles(&self, symbol: &str, timeframe: &str, limit: u32) -> BotResult<Vec<Candle>>;
}
