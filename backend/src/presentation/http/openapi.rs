use utoipa::OpenApi;

use super::routes::{application, backtest, exchanges, health, meta, monitor, portfolio};
use crate::modules::application_contracts::Signal;
use crate::modules::config_api::{OperationMode, RiskProfile};
use crate::modules::exchanges::capabilities::{Capability, ExchangeCapability};
use crate::modules::http_bridge::{risk, strategy};
use crate::presentation::http::error::ApiErrorBody;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::healthz,
        health::readyz,
        meta::meta,
        application::list_signals,
        exchanges::catalog,
        exchanges::routing_matrix,
        crate::presentation::http::routes::risk::compute_profile_limits,
        crate::presentation::http::routes::risk::validate_order_intent,
        crate::presentation::http::routes::strategy::sma_periods,
        portfolio::paper_wallet,
        backtest::run_sma_backtest,
        monitor::snapshot,
        monitor::post_command,
    ),
    components(schemas(
        ApiErrorBody,
        health::HealthResponse,
        health::ReadyResponse,
        meta::MetaResponse,
        application::SignalDescriptor,
        application::SignalsResponse,
        Signal,
        exchanges::ExchangeCatalogResponse,
        exchanges::ExchangeRoutingResponse,
        exchanges::RoutingRow,
        ExchangeCapability,
        Capability,
        risk::ProfileLimitsRequest,
        risk::ProfileLimitsResponse,
        risk::ValidateIntentRequest,
        risk::ValidateIntentResponse,
        risk::RiskLimitsBody,
        risk::OrderIntentBody,
        RiskProfile,
        strategy::StrategyPeriodsResponse,
        strategy::PeriodsQuery,
        OperationMode,
        portfolio::PaperSnapshotResponse,
        portfolio::PaperSnapshotQuery,
        backtest::BacktestRequest,
        monitor::MonitorSnapshotResponse,
        monitor::MonitorCommandRequest,
        monitor::MonitorCommandResponse,
        monitor::MonitorCommandName,
    )),
    tags(
        (name = "system", description = "Health and metadata"),
        (name = "application", description = "Shared application contracts"),
        (name = "exchanges", description = "Exchange catalog and routing"),
        (name = "risk", description = "Risk limits and validation"),
        (name = "strategy", description = "Strategy presets"),
        (name = "portfolio", description = "Paper portfolio snapshots"),
        (name = "backtest", description = "Synthetic SMA backtest"),
        (name = "monitor", description = "Live monitor control when attached"),
    ),
    info(
        title = "Rust Trading Bot API",
        version = "0.1.0",
        description = "HTTP surface for backend modules. Monitor routes require a monitor handle in the same process."
    )
)]
pub struct ApiDoc;
