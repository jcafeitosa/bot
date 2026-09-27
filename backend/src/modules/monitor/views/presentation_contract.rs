#![allow(dead_code)]
//! Public DTO surface; consumers will wire in a later migration slice.

//! DTOs and validation for the monitor ↔ presentation boundary (see `monitor-presentation-contract-sdd`).

use std::fmt;

use utoipa::ToSchema;

pub const LABEL_MAX_BYTES: usize = 128;
pub const NOTICE_MAX_BYTES: usize = 1_024;
pub const ERROR_ADVISORY_MAX_BYTES: usize = 4_096;
pub const LOG_MESSAGE_MAX_BYTES: usize = 4_096;
pub const MAX_LOG_ENTRIES: usize = 10;
pub const DECIMAL_SCALE_MAX: u32 = 18;

const TRUNCATION_ELLIPSIS: &str = "…";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorCommand {
    Pause,
    Resume,
    Refresh,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Dev,
    Prod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Observe,
    Paper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
pub enum MonitorRunState {
    Running,
    Paused,
    Resuming,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSchema)]
pub enum MonitorSignal {
    Warmup,
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    pub coefficient: i128,
    pub scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalError {
    ScaleOutOfRange,
}

impl Decimal {
    pub fn new(coefficient: i128, scale: u32) -> Result<Self, DecimalError> {
        if scale > DECIMAL_SCALE_MAX {
            return Err(DecimalError::ScaleOutOfRange);
        }
        Ok(Self { coefficient, scale })
    }

    /// Converts a finite `f64` into an exact decimal when representable at the given scale.
    pub fn from_finite_f64(value: f64, scale: u32) -> Result<Self, DecimalError> {
        if scale > DECIMAL_SCALE_MAX || !value.is_finite() {
            return Err(DecimalError::ScaleOutOfRange);
        }
        let factor = 10f64.powi(i32::try_from(scale).unwrap_or(i32::MAX));
        let scaled = value * factor;
        let rounded = scaled.round();
        if (scaled - rounded).abs() > 1e-9 {
            return Err(DecimalError::ScaleOutOfRange);
        }
        let coefficient = i128::from(rounded as i64);
        Self::new(coefficient, scale)
    }
}

/// Half-even formatting to a fixed number of fractional digits (presentation only).
pub fn format_decimal_half_even(decimal: &Decimal, presentation_places: u32) -> String {
    let target_scale = presentation_places.min(DECIMAL_SCALE_MAX);
    let mut coefficient = decimal.coefficient;
    let mut scale = decimal.scale;
    while scale > target_scale {
        let digit = (coefficient % 10).unsigned_abs();
        coefficient /= 10;
        let round_up = match digit {
            0..=4 => false,
            6..=9 => true,
            5 => coefficient % 2 != 0,
            _ => false,
        };
        if round_up {
            coefficient += if coefficient.is_negative() { -1 } else { 1 };
        }
        scale -= 1;
    }
    while scale < target_scale {
        coefficient *= 10;
        scale += 1;
    }
    let sign = if coefficient.is_negative() { "-" } else { "" };
    let abs = coefficient.unsigned_abs();
    if target_scale == 0 {
        return format!("{sign}{abs}");
    }
    let divisor = 10u128.pow(target_scale);
    let whole = abs / divisor;
    let frac = abs % divisor;
    format!(
        "{sign}{whole}.{frac:0width$}",
        width = target_scale as usize
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorMarketSnapshot {
    pub close: Decimal,
    pub sma_fast: Option<Decimal>,
    pub sma_slow: Option<Decimal>,
    pub signal: Option<MonitorSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorLogEntry {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub revision: u64,
    pub environment: Environment,
    pub run_mode: RunMode,
    pub symbol: String,
    pub operation_label: String,
    pub risk_profile_label: String,
    pub run_state: MonitorRunState,
    pub sma_fast_period: u32,
    pub sma_slow_period: u32,
    pub max_order_quote: Decimal,
    pub persistence_status: PersistenceStatus,
    pub market: Option<MonitorMarketSnapshot>,
    pub paper_positions: Option<u64>,
    pub advisory: Option<String>,
    pub last_error: Option<String>,
    pub logs: Vec<MonitorLogEntry>,
    pub bot_runtime_enabled: bool,
    pub promoted_bot_id: Option<String>,
    pub promoted_by: Option<String>,
}

impl MonitorSnapshot {
    pub fn initial() -> Self {
        Self {
            revision: 0,
            environment: Environment::Dev,
            run_mode: RunMode::Observe,
            symbol: String::new(),
            operation_label: String::new(),
            risk_profile_label: String::new(),
            run_state: MonitorRunState::Running,
            sma_fast_period: 0,
            sma_slow_period: 0,
            max_order_quote: Decimal {
                coefficient: 0,
                scale: 0,
            },
            persistence_status: PersistenceStatus::Unavailable,
            market: None,
            paper_positions: None,
            advisory: None,
            last_error: None,
            logs: Vec::new(),
            bot_runtime_enabled: false,
            promoted_bot_id: None,
            promoted_by: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorNotice {
    pub severity: NoticeSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorEvent {
    StateChanged { revision: u64 },
    Notice(MonitorNotice),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorSendError {
    Full,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelValidationError {
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotValidationError {
    Label(LabelValidationError),
    LogsTooMany { max: usize, actual: usize },
    Decimal(DecimalError),
}

pub fn validate_bounded_label(value: &str) -> Result<String, LabelValidationError> {
    if value.len() > LABEL_MAX_BYTES {
        return Err(LabelValidationError::TooLong {
            max_bytes: LABEL_MAX_BYTES,
            actual_bytes: value.len(),
        });
    }
    Ok(value.to_owned())
}

pub fn truncate_utf8_informative(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let ellipsis_len = TRUNCATION_ELLIPSIS.len();
    if max_bytes <= ellipsis_len {
        return truncate_utf8_prefix(value, max_bytes);
    }
    let body_budget = max_bytes - ellipsis_len;
    let mut end = body_budget;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATION_ELLIPSIS}", &value[..end])
}

fn truncate_utf8_prefix(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

pub fn normalize_informative_field(value: Option<String>, max_bytes: usize) -> Option<String> {
    value.map(|text| truncate_utf8_informative(&text, max_bytes))
}

pub fn normalize_log_messages(messages: impl IntoIterator<Item = String>) -> Vec<MonitorLogEntry> {
    let mut logs: Vec<MonitorLogEntry> = messages
        .into_iter()
        .map(|message| MonitorLogEntry {
            message: truncate_utf8_informative(&message, LOG_MESSAGE_MAX_BYTES),
        })
        .collect();
    if logs.len() > MAX_LOG_ENTRIES {
        logs = logs.split_off(logs.len() - MAX_LOG_ENTRIES);
    }
    logs
}

pub fn validate_snapshot(snapshot: &MonitorSnapshot) -> Result<(), SnapshotValidationError> {
    validate_bounded_label(&snapshot.symbol).map_err(SnapshotValidationError::Label)?;
    validate_bounded_label(&snapshot.operation_label).map_err(SnapshotValidationError::Label)?;
    validate_bounded_label(&snapshot.risk_profile_label).map_err(SnapshotValidationError::Label)?;
    Decimal::new(
        snapshot.max_order_quote.coefficient,
        snapshot.max_order_quote.scale,
    )
    .map_err(SnapshotValidationError::Decimal)?;
    if let Some(market) = &snapshot.market {
        Decimal::new(market.close.coefficient, market.close.scale)
            .map_err(SnapshotValidationError::Decimal)?;
        for dec in [market.sma_fast, market.sma_slow].into_iter().flatten() {
            Decimal::new(dec.coefficient, dec.scale).map_err(SnapshotValidationError::Decimal)?;
        }
    }
    if snapshot.logs.len() > MAX_LOG_ENTRIES {
        return Err(SnapshotValidationError::LogsTooMany {
            max: MAX_LOG_ENTRIES,
            actual: snapshot.logs.len(),
        });
    }
    for entry in &snapshot.logs {
        if entry.message.len() > LOG_MESSAGE_MAX_BYTES {
            return Err(SnapshotValidationError::Label(
                LabelValidationError::TooLong {
                    max_bytes: LOG_MESSAGE_MAX_BYTES,
                    actual_bytes: entry.message.len(),
                },
            ));
        }
    }
    Ok(())
}

impl fmt::Display for LabelValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong {
                max_bytes,
                actual_bytes,
            } => write!(
                f,
                "label exceeds {max_bytes} UTF-8 bytes (got {actual_bytes})"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_round_trip_preserves_coefficient_and_scale() {
        let cases = [
            Decimal {
                coefficient: 42,
                scale: 0,
            },
            Decimal {
                coefficient: 1_2345,
                scale: 4,
            },
            Decimal {
                coefficient: -9876,
                scale: 2,
            },
            Decimal {
                coefficient: i128::MAX,
                scale: 0,
            },
        ];
        for original in cases {
            let decoded =
                Decimal::new(original.coefficient, original.scale).expect("valid decimal");
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn decimal_rejects_scale_above_eighteen() {
        assert_eq!(Decimal::new(1, 19), Err(DecimalError::ScaleOutOfRange));
    }

    #[test]
    fn half_even_formats_market_and_order_cap_scales() {
        let market = Decimal {
            coefficient: 123_456_789,
            scale: 8,
        };
        assert_eq!(format_decimal_half_even(&market, 4), "1.2346");
        let tie_positive = Decimal {
            coefficient: 125,
            scale: 2,
        };
        assert_eq!(format_decimal_half_even(&tie_positive, 1), "1.2");
        let tie_negative = Decimal {
            coefficient: -125,
            scale: 2,
        };
        assert_eq!(format_decimal_half_even(&tie_negative, 1), "-1.2");
        let cap = Decimal {
            coefficient: 999,
            scale: 2,
        };
        assert_eq!(format_decimal_half_even(&cap, 2), "9.99");
    }

    #[test]
    fn truncate_respects_utf8_boundaries_and_ellipsis_budget() {
        let at_limit = "á".repeat(LABEL_MAX_BYTES / 2);
        assert_eq!(
            truncate_utf8_informative(&at_limit, LABEL_MAX_BYTES).len(),
            at_limit.len()
        );
        let over = format!("{}x", at_limit);
        let truncated = truncate_utf8_informative(&over, LABEL_MAX_BYTES);
        assert!(truncated.len() <= LABEL_MAX_BYTES);
        assert!(truncated.ends_with(TRUNCATION_ELLIPSIS));
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn validate_label_rejects_oversized_symbol() {
        let symbol = "x".repeat(LABEL_MAX_BYTES + 1);
        assert!(matches!(
            validate_bounded_label(&symbol),
            Err(LabelValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn monitor_event_state_changed_carries_revision_only() {
        let event = MonitorEvent::StateChanged { revision: 7 };
        assert!(matches!(event, MonitorEvent::StateChanged { revision: 7 }));
    }

    #[test]
    fn normalize_log_messages_keeps_last_ten_entries() {
        let messages: Vec<String> = (0..12).map(|index| format!("log-{index}")).collect();
        let logs = normalize_log_messages(messages);
        assert_eq!(logs.len(), MAX_LOG_ENTRIES);
        assert_eq!(logs.first().unwrap().message, "log-2");
        assert_eq!(logs.last().unwrap().message, "log-11");
    }
}
