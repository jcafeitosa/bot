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

#[derive(Debug)]
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
            AgentsError::PromotionDenied(message) => {
                ApiError::with_code(StatusCode::FORBIDDEN, "promotion_denied", message)
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

    pub fn owner_mismatch() -> Self {
        ApiError::with_code(
            StatusCode::FORBIDDEN,
            "owner_mismatch",
            "owner_id does not match configured BOT_HTTP_OWNER_ID",
        )
    }

    pub fn http_agency_mismatch() -> Self {
        ApiError::with_code(
            StatusCode::FORBIDDEN,
            "http_agency_mismatch",
            "agency does not match configured BOT_HTTP_AGENCY_ID",
        )
    }

    pub fn unauthorized() -> Self {
        ApiError::with_code(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid admin bearer token",
        )
    }

    pub fn monitor_unavailable() -> Self {
        ApiError::with_code(
            StatusCode::SERVICE_UNAVAILABLE,
            "monitor_unavailable",
            "monitor is not attached to this API process",
        )
    }

    pub fn from_bots_error(error: crate::modules::bots::BotsError) -> Self {
        use crate::modules::bots::BotsError;
        match error {
            BotsError::RuntimeDisabled => ApiError::with_code(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime_disabled",
                "bot runtime promotion is disabled in this build",
            ),
            BotsError::RuntimeNotPromoted => ApiError::with_code(
                StatusCode::NOT_FOUND,
                "runtime_not_promoted",
                "no active bot promotion to demote",
            ),
            BotsError::InvalidId(message)
            | BotsError::InvalidTimeframe(message)
            | BotsError::InvalidSymbol(message)
            | BotsError::InvalidStrategy(message) => {
                ApiError::with_code(StatusCode::BAD_REQUEST, "bots", message)
            }
            BotsError::InvalidTimeframeForMode {
                timeframe,
                operation,
            } => ApiError::with_code(
                StatusCode::BAD_REQUEST,
                "bots",
                format!("timeframe {timeframe} invalid for {operation:?}"),
            ),
            BotsError::InvalidWindow | BotsError::InvalidMetrics => ApiError::with_code(
                StatusCode::BAD_REQUEST,
                "bots",
                "invalid bot metrics or evaluation window",
            ),
            BotsError::DuplicateRun => {
                ApiError::with_code(StatusCode::CONFLICT, "bots", "duplicate run")
            }
            BotsError::IncompatibleRanking => ApiError::with_code(
                StatusCode::BAD_REQUEST,
                "bots",
                "incompatible ranking batch",
            ),
            BotsError::CatalogStore(message) => {
                ApiError::with_code(StatusCode::SERVICE_UNAVAILABLE, "catalog_store", message)
            }
        }
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
            OrdersError::LiveExchangeNotWired => ApiError::with_code(
                StatusCode::SERVICE_UNAVAILABLE,
                "live_exchange_not_wired",
                "live exchange order execution is not wired in this build",
            ),
            OrdersError::StoreUnavailable(message) => ApiError::with_code(
                StatusCode::SERVICE_UNAVAILABLE,
                "order_store_unavailable",
                message,
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

#[cfg(test)]
mod orders_error_mapping_tests {
    use super::ApiError;
    use crate::modules::orders::OrdersError;
    use axum::http::StatusCode;

    #[test]
    fn store_unavailable_maps_to_service_unavailable() {
        let api = ApiError::from_orders_error(OrdersError::StoreUnavailable(
            "idempotency lookup: relation missing".into(),
        ));
        assert_eq!(api.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(api.error_code(), Some("order_store_unavailable"));
    }
}

#[cfg(test)]
impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    pub fn error_code(&self) -> Option<&str> {
        self.body.code.as_deref()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
