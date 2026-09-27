//! HTTP API order executor selection (fail-closed by default; no exchange live).

use crate::modules::orders::{
    live_exchange_submit_backend_enabled, AcceptingExecutor, ExchangeSpotExecutor,
    FailClosedExecutor, OrderExecutionPort, OrdersError, PaperLedgerExecutor,
    ReservedLiveExchangeExecutor, SubmitOrderRequest,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HttpOrderExecutionMode {
    #[default]
    Disabled,
    /// Local/dev only: accepts after risk (double port). Not an exchange adapter.
    DevAccept,
    /// Paper ledger after risk (in-process; not exchange REST).
    Paper,
    /// Spot adapter wired (`live_exchange` + `BOT_ORDERS_EXCHANGE_SUBMIT=recording` today).
    LiveExchange,
    /// `live_exchange` without submit backend configured.
    LiveExchangeReserved,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpOrderExecutor {
    mode: HttpOrderExecutionMode,
}

impl HttpOrderExecutor {
    pub fn fail_closed() -> Self {
        Self {
            mode: HttpOrderExecutionMode::Disabled,
        }
    }

    pub fn dev_accept() -> Self {
        Self {
            mode: HttpOrderExecutionMode::DevAccept,
        }
    }

    pub fn paper() -> Self {
        Self {
            mode: HttpOrderExecutionMode::Paper,
        }
    }

    pub fn live_exchange() -> Self {
        Self {
            mode: HttpOrderExecutionMode::LiveExchange,
        }
    }

    pub fn live_exchange_reserved() -> Self {
        Self {
            mode: HttpOrderExecutionMode::LiveExchangeReserved,
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("BOT_ORDERS_EXECUTION") {
            Ok(raw) if raw.trim().eq_ignore_ascii_case("dev_accept") => Self::dev_accept(),
            Ok(raw) if raw.trim().eq_ignore_ascii_case("paper") => {
                tracing::info!(
                    target: "api",
                    value = raw.trim(),
                    "BOT_ORDERS_EXECUTION selects paper ledger executor"
                );
                Self::paper()
            }
            Ok(raw) if raw.trim().eq_ignore_ascii_case("live_exchange") => {
                if live_exchange_submit_backend_enabled() {
                    tracing::info!(
                        target: "api",
                        value = raw.trim(),
                        "BOT_ORDERS_EXECUTION selects wired spot executor (recording backend)"
                    );
                    Self::live_exchange()
                } else {
                    tracing::info!(
                        target: "api",
                        value = raw.trim(),
                        "BOT_ORDERS_EXECUTION selects reserved live exchange seam (not wired)"
                    );
                    Self::live_exchange_reserved()
                }
            }
            Ok(raw) if !raw.trim().is_empty() && !raw.trim().eq_ignore_ascii_case("disabled") => {
                tracing::warn!(
                    target: "api",
                    value = raw.trim(),
                    "unknown BOT_ORDERS_EXECUTION; using fail-closed disabled"
                );
                Self::fail_closed()
            }
            _ => Self::fail_closed(),
        }
    }

    pub fn mode(&self) -> HttpOrderExecutionMode {
        self.mode
    }

    /// `true` when spot submit backend is wired on this executor (recording seam or future testnet REST).
    pub fn live_exchange_wired(&self) -> bool {
        matches!(self.mode, HttpOrderExecutionMode::LiveExchange)
    }
}

impl HttpOrderExecutionMode {
    pub fn as_api_label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::DevAccept => "dev_accept",
            Self::Paper => "paper",
            Self::LiveExchange => "live_exchange",
            Self::LiveExchangeReserved => "live_exchange_reserved",
        }
    }
}

impl OrderExecutionPort for HttpOrderExecutor {
    fn execute(&self, request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        match self.mode {
            HttpOrderExecutionMode::Disabled => FailClosedExecutor.execute(request),
            HttpOrderExecutionMode::DevAccept => AcceptingExecutor.execute(request),
            HttpOrderExecutionMode::Paper => PaperLedgerExecutor.execute(request),
            HttpOrderExecutionMode::LiveExchange => ExchangeSpotExecutor.execute(request),
            HttpOrderExecutionMode::LiveExchangeReserved => {
                ReservedLiveExchangeExecutor.execute(request)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;
    use crate::modules::orders::{OrderSide, OrdersError, PaperLedgerExecutor};

    fn sample_request() -> SubmitOrderRequest<'static> {
        SubmitOrderRequest {
            symbol: "BTC/USDT",
            side: OrderSide::Buy,
            quote_amount: 1.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            paper_fill_unit_price: None,
            client_order_id: None,
        }
    }

    #[test]
    fn disabled_mode_returns_execution_disabled() {
        let err = HttpOrderExecutor::fail_closed()
            .execute(&sample_request())
            .unwrap_err();
        assert!(matches!(err, OrdersError::ExecutionDisabled));
    }

    #[test]
    fn dev_accept_mode_succeeds() {
        HttpOrderExecutor::dev_accept()
            .execute(&sample_request())
            .expect("dev accept");
    }

    #[test]
    fn from_env_live_exchange_uses_reserved_seam() {
        with_env_test_lock(|| {
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::set_var("BOT_ORDERS_EXECUTION", "live_exchange");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            assert_eq!(
                executor.mode(),
                HttpOrderExecutionMode::LiveExchangeReserved
            );
            assert!(matches!(
                executor.execute(&sample_request()).unwrap_err(),
                OrdersError::LiveExchangeNotWired
            ));
        });
    }

    #[test]
    fn from_env_unknown_value_falls_back_to_disabled() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "not_a_mode");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            assert_eq!(executor.mode(), HttpOrderExecutionMode::Disabled);
            assert!(matches!(
                executor.execute(&sample_request()).unwrap_err(),
                OrdersError::ExecutionDisabled
            ));
        });
    }

    #[test]
    fn live_exchange_wired_false_until_adapter() {
        assert!(!HttpOrderExecutor::fail_closed().live_exchange_wired());
        assert!(!HttpOrderExecutor::dev_accept().live_exchange_wired());
        assert!(!HttpOrderExecutor::paper().live_exchange_wired());
        assert!(!HttpOrderExecutor::live_exchange_reserved().live_exchange_wired());
        assert!(HttpOrderExecutor::live_exchange().live_exchange_wired());
    }

    #[test]
    fn from_env_live_exchange_wired_when_recording_backend_configured() {
        use crate::modules::orders::RecordingSpotOrderSubmitPort;
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "live_exchange");
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
            let executor = HttpOrderExecutor::from_env();
            assert_eq!(executor.mode(), HttpOrderExecutionMode::LiveExchange);
            assert!(executor.live_exchange_wired());
            RecordingSpotOrderSubmitPort::clear();
            executor.execute(&sample_request()).expect("wired spot");
            assert_eq!(RecordingSpotOrderSubmitPort::call_count(), 1);
            RecordingSpotOrderSubmitPort::clear();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        });
    }

    #[test]
    fn paper_mode_records_ledger_fill() {
        PaperLedgerExecutor::clear_ledger();
        HttpOrderExecutor::paper()
            .execute(&sample_request())
            .expect("paper execute");
        let fills = PaperLedgerExecutor::recorded_fills();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].symbol, "BTC/USDT");
        assert_eq!(fills[0].quote_amount, 1.0);
        PaperLedgerExecutor::clear_ledger();
    }

    #[test]
    fn from_env_paper_uses_paper_mode() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "paper");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            assert_eq!(executor.mode(), HttpOrderExecutionMode::Paper);
            PaperLedgerExecutor::clear_ledger();
            executor.execute(&sample_request()).expect("paper");
            assert_eq!(PaperLedgerExecutor::recorded_fills().len(), 1);
            PaperLedgerExecutor::clear_ledger();
        });
    }

    #[test]
    fn from_env_dev_accept_enables_accepting_port() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "dev_accept");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            assert_eq!(executor.mode(), HttpOrderExecutionMode::DevAccept);
            executor.execute(&sample_request()).expect("accept");
        });
    }

    #[test]
    fn from_env_live_exchange_wired_when_testnet_backend_and_credentials() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "live_exchange");
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            assert_eq!(executor.mode(), HttpOrderExecutionMode::LiveExchange);
            assert!(executor.live_exchange_wired());
        });
    }

    #[test]
    fn from_env_live_exchange_stays_reserved_when_testnet_backend_unwired() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "live_exchange");
            std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "testnet");
            let executor = HttpOrderExecutor::from_env();
            std::env::remove_var("BOT_ORDERS_EXECUTION");
            std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
            assert_eq!(
                executor.mode(),
                HttpOrderExecutionMode::LiveExchangeReserved
            );
            assert!(!executor.live_exchange_wired());
        });
    }
}
