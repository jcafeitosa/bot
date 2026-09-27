use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::core::error::BotError;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

pub struct ApiError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                error: error.into(),
                code: None,
            },
        }
    }

    pub fn with_code(
        status: StatusCode,
        code: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                error: error.into(),
                code: Some(code.into()),
            },
        }
    }

    pub fn from_bot_error(error: BotError) -> Self {
        match error {
            BotError::Configuration(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "configuration", message)
            }
            BotError::MarketData(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "market_data", message)
            }
            BotError::Strategy(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "strategy", message)
            }
            BotError::RiskRejected(message) => {
                ApiError::with_code(StatusCode::UNPROCESSABLE_ENTITY, "risk_rejected", message)
            }
            BotError::Jev(message) => ApiError::with_code(StatusCode::BAD_GATEWAY, "jev", message),
            BotError::Exchange(message) => {
                ApiError::with_code(StatusCode::BAD_GATEWAY, "exchange", message)
            }
            BotError::AmbiguousOrder(message) => {
                ApiError::with_code(StatusCode::CONFLICT, "ambiguous_order", message)
            }
        }
    }

    pub fn from_agents_error(error: crate::modules::agents::AgentsError) -> Self {
        use crate::modules::agents::AgentsError;
        match error {
            AgentsError::NotFound(message) => {
                ApiError::with_code(StatusCode::NOT_FOUND, "not_found", message)
            }
            AgentsError::Duplicate(message) => {
                ApiError::with_code(StatusCode::CONFLICT, "duplicate", message)
            }
            AgentsError::AgencyMismatch { agent } => ApiError::with_code(
                StatusCode::FORBIDDEN,
                "agency_mismatch",
                format!("agency mismatch for agent {agent}"),
            ),
            AgentsError::AdvisoryDenied(message) => {
                ApiError::with_code(StatusCode::FORBIDDEN, "advisory_denied", message)
            }
            AgentsError::Persistence(message) => ApiError::with_code(
                StatusCode::SERVICE_UNAVAILABLE,
                "persistence_failed",
                message,
            ),
            AgentsError::InvalidId(message)
            | AgentsError::Hierarchy(message)
            | AgentsError::Lifecycle(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "agents", message)
            }
        }
    }

    pub fn monitor_unavailable() -> Self {
        ApiError::with_code(
            StatusCode::SERVICE_UNAVAILABLE,
            "monitor_unavailable",
            "monitor is not attached to this API process",
        )
    }

    pub fn from_orders_error(error: crate::modules::orders::OrdersError) -> Self {
        use crate::modules::orders::OrdersError;
        match error {
            OrdersError::InvalidRequest(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "invalid_order", message)
            }
            OrdersError::RiskRejected(message) => {
                ApiError::with_code(StatusCode::UNPROCESSABLE_ENTITY, "risk_rejected", message)
            }
            OrdersError::ExecutionDisabled => ApiError::with_code(
                StatusCode::SERVICE_UNAVAILABLE,
                "execution_disabled",
                "order execution is disabled in this build",
            ),
        }
    }

    pub fn jev_unavailable() -> Self {
        ApiError::with_code(
            StatusCode::SERVICE_UNAVAILABLE,
            "jev_unavailable",
            "Jev advisory is disabled or not configured in this API process",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
