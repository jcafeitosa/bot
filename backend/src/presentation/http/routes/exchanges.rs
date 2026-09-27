use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::modules::config_api::Environment;
use crate::modules::exchanges::{
    capabilities::{self, ExchangeCapability},
    registry::default_dev_accounts,
    router::{rest_use_for_need, route_market_need, MarketNeed},
    ExchangeAccountId, ExchangeId, MarketType,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct ExchangeCatalogResponse {
    pub exchanges: Vec<ExchangeCapability>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutingRow {
    pub need: String,
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_use: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExchangeRoutingResponse {
    pub account_key: String,
    pub symbol: String,
    pub routes: Vec<RoutingRow>,
}

fn spot_dev_account() -> ExchangeAccountId {
    ExchangeAccountId::new(
        ExchangeId::Binance,
        MarketType::Spot,
        "paper-main",
        Environment::Dev,
    )
    .expect("valid dev spot account id")
}

#[utoipa::path(
    get,
    path = "/api/v1/exchanges/catalog",
    tag = "exchanges",
    responses((status = 200, description = "Exchange capability catalog", body = ExchangeCatalogResponse))
)]
pub async fn catalog() -> Json<ExchangeCatalogResponse> {
    Json(ExchangeCatalogResponse {
        exchanges: capabilities::catalog().into_values().collect(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/exchanges/routing",
    tag = "exchanges",
    responses((status = 200, description = "Market need routing matrix", body = ExchangeRoutingResponse))
)]
pub async fn routing_matrix() -> Json<ExchangeRoutingResponse> {
    let account = spot_dev_account();
    let symbol = default_dev_accounts()
        .into_iter()
        .find(|entry| entry.id.market == MarketType::Spot)
        .and_then(|entry| entry.symbols.first().cloned())
        .unwrap_or_else(|| "BTC/USDT".to_owned());
    let needs = [
        MarketNeed::HistoricalBackfill,
        MarketNeed::LiveIncremental,
        MarketNeed::OrderLifecycle,
        MarketNeed::BalanceSnapshot,
        MarketNeed::UserData,
    ];
    let routes = needs
        .into_iter()
        .map(|need| {
            let transport = route_market_need(&account, need)
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|error| format!("error:{error}"));
            RoutingRow {
                need: format!("{need:?}"),
                transport,
                rest_use: rest_use_for_need(need).map(|value| format!("{value:?}")),
            }
        })
        .collect();
    Json(ExchangeRoutingResponse {
        account_key: account.key(),
        symbol,
        routes,
    })
}
