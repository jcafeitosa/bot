use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdersError {
    InvalidRequest(String),
    RiskRejected(String),
    ExecutionDisabled,
    LiveExchangeNotWired,
}

impl fmt::Display for OrdersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "invalid order request: {message}"),
            Self::RiskRejected(message) => write!(f, "risk rejected order: {message}"),
            Self::ExecutionDisabled => write!(f, "order execution is disabled in this build"),
            Self::LiveExchangeNotWired => write!(
                f,
                "live exchange order execution is not wired in this build"
            ),
        }
    }
}

impl std::error::Error for OrdersError {}
