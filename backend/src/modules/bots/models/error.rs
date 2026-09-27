use std::fmt;

use crate::core::config::OperationMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotsError {
    InvalidId(String),
    InvalidTimeframe(String),
    InvalidTimeframeForMode {
        timeframe: String,
        operation: OperationMode,
    },
    InvalidSymbol(String),
    InvalidStrategy(String),
    InvalidWindow,
    InvalidMetrics,
    DuplicateRun,
    IncompatibleRanking,
    CatalogStore(String),
}

impl fmt::Display for BotsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BotsError {}
