use serde::{Deserialize, Serialize};

use super::{ExchangeAccountId, Transport};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamKind {
    Kline1m,
    MiniTicker,
    Depth,
    Trade,
    UserData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub account: ExchangeAccountId,
    pub stream: StreamKind,
    pub symbol: String,
    pub event_ms: i64,
    pub payload: serde_json::Value,
}

impl StreamEvent {
    pub fn validate(&self) -> Result<(), StreamError> {
        if self.event_ms < 0 || self.symbol.trim().is_empty() {
            return Err(StreamError::InvalidEvent);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSubscription {
    pub account: ExchangeAccountId,
    pub stream: StreamKind,
    pub symbol: String,
    pub transport: Transport,
}

impl StreamSubscription {
    pub fn validate(&self) -> Result<(), StreamError> {
        if self.symbol.trim().is_empty() {
            return Err(StreamError::InvalidEvent);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum StreamError {
    #[error("invalid stream event")]
    InvalidEvent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;
    use serde_json::json;

    use super::super::{ExchangeId, MarketType};

    #[test]
    fn stream_event_requires_symbol_and_timestamp() {
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        let event = StreamEvent {
            account,
            stream: StreamKind::Kline1m,
            symbol: String::new(),
            event_ms: -1,
            payload: json!({}),
        };
        assert_eq!(event.validate().unwrap_err(), StreamError::InvalidEvent);
    }
}
