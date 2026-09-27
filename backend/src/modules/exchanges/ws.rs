use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::stream::StreamSubscription;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WsConfig {
    pub connect_timeout_secs: u64,
    pub heartbeat_secs: u64,
    pub max_reconnect_delay_secs: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
            heartbeat_secs: 30,
            max_reconnect_delay_secs: 60,
        }
    }
}

impl WsConfig {
    pub fn validate(&self) -> Result<(), WsError> {
        if self.connect_timeout_secs == 0
            || self.heartbeat_secs == 0
            || self.max_reconnect_delay_secs == 0
            || self.max_reconnect_delay_secs < self.heartbeat_secs
        {
            return Err(WsError::InvalidConfig);
        }
        Ok(())
    }

    pub fn reconnect_delay(&self, attempt: u32) -> Duration {
        let shift = attempt.min(6);
        let seconds = 2u64
            .saturating_pow(shift)
            .min(self.max_reconnect_delay_secs);
        Duration::from_secs(seconds.max(1))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSessionPlan {
    pub endpoint: String,
    pub config: WsConfig,
    pub subscriptions: Vec<StreamSubscription>,
}

impl WsSessionPlan {
    pub fn validate(&self) -> Result<(), WsError> {
        if !(self.endpoint.starts_with("wss://") || self.endpoint.starts_with("ws://localhost")) {
            return Err(WsError::InvalidConfig);
        }
        self.config.validate()?;
        if self.subscriptions.is_empty() {
            return Err(WsError::InvalidConfig);
        }
        for subscription in &self.subscriptions {
            subscription
                .validate()
                .map_err(|_| WsError::InvalidConfig)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum WsError {
    #[error("invalid websocket plan")]
    InvalidConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Environment;
    use crate::modules::exchanges::stream::{StreamKind, StreamSubscription};
    use crate::modules::exchanges::{ExchangeAccountId, ExchangeId, MarketType, Transport};

    #[test]
    fn reconnect_delay_is_bounded() {
        let config = WsConfig::default();
        assert_eq!(config.reconnect_delay(0), Duration::from_secs(1));
        assert_eq!(config.reconnect_delay(99), Duration::from_secs(60));
    }

    #[test]
    fn session_plan_requires_secure_endpoint_and_subscriptions() {
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        let subscription = StreamSubscription {
            account: account.clone(),
            stream: StreamKind::Kline1m,
            symbol: "BTC/USDT".to_owned(),
            transport: Transport::StreamWs,
        };
        let plan = WsSessionPlan {
            endpoint: "http://insecure".to_owned(),
            config: WsConfig::default(),
            subscriptions: vec![subscription],
        };
        assert_eq!(plan.validate().unwrap_err(), WsError::InvalidConfig);
    }
}
