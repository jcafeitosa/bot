use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("invalid configuration: {0}")]
    Configuration(String),
    #[error("market data error: {0}")]
    MarketData(String),
    #[error("strategy error: {0}")]
    Strategy(String),
    #[error("JeV advisor error: {0}")]
    Jev(String),
    #[error("risk gate rejected intent: {0}")]
    RiskRejected(String),
    #[error("exchange operation failed: {0}")]
    Exchange(String),
    #[allow(dead_code)] // Reserved for the disabled order path; kept for the existing error contract.
    #[error("ambiguous order state; automatic retry is blocked: {0}")]
    AmbiguousOrder(String),
}

pub type BotResult<T> = Result<T, BotError>;
