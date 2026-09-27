use std::sync::Arc;

use crate::core::config::ProductOwnerBootstrapConfig;
use crate::core::database::{AppDatabases, GraphProjectionSync};
use crate::core::error::BotError;
use crate::core::persistence::Database;
use crate::core::providers::JevAdvisor;
use crate::modules::agents::adapters::ensure_product_owner_bootstrapped;
use crate::modules::agents::{AgentRegistry, VerifiedProductOwner};
use crate::modules::bots::{BotCatalogBackend, BotCatalogStore, BotRuntimePort};
use crate::modules::bots::{BotPromotionRecord, BotRuntimeStatus, PromoteBotRequest};
use crate::modules::config_api::Config;
use crate::modules::http_bridge::agents::{
    AdvisoryRequest, AdvisoryResponse, AgentListResponse, AgentResponse, AuditLogResponse,
    LifecycleResponse, RegisterAgentRequest,
};
use crate::modules::http_bridge::bots::{BotCatalogPersistResponse, BotCatalogResponse};
use crate::modules::http_bridge::bots_runtime;
use crate::modules::http_bridge::config::ConfigSnapshotResponse;
use crate::modules::http_bridge::monitor::{
    MonitorCommandHttpError, MonitorCommandRequest, MonitorSnapshotResponse,
};
use crate::modules::http_bridge::orders::{SubmitOrderHttpRequest, SubmitOrderResponse};
use crate::modules::http_bridge::providers::ProvidersStatusResponse;
use crate::modules::monitor::MonitorHandle;
use crate::modules::orders::{
    take_last_spot_submit_ack, InMemoryOrderIdempotencyStore, InMemoryOrderReconciliationLedger,
    OrderReconciliationLedger, PgOrderIdempotencyStore, PgOrderReconciliationStore,
    ReconciliationPollSummary, ReconciliationState,
};
use crate::presentation::http::admin_auth::HttpAdminAuth;
use crate::presentation::http::error::ApiError;
use crate::presentation::http::order_execution::HttpOrderExecutor;

/// HTTP-only seams loaded at bootstrap (admin bearer + order execution + bot runtime).
#[derive(Clone)]
pub struct HttpApiSeams {
    pub admin_auth: HttpAdminAuth,
    pub order_executor: HttpOrderExecutor,
    pub bot_runtime: Arc<dyn BotRuntimePort>,
}

impl HttpApiSeams {
    pub fn disabled_fail_closed() -> Self {
        use crate::modules::bots::FailClosedBotRuntime;
        Self {
            admin_auth: HttpAdminAuth::disabled(),
            order_executor: HttpOrderExecutor::fail_closed(),
            bot_runtime: Arc::new(FailClosedBotRuntime),
        }
    }

    pub fn from_env() -> Self {
        use crate::modules::bots::shared_bot_runtime;
        Self {
            admin_auth: HttpAdminAuth::from_env(),
            order_executor: HttpOrderExecutor::from_env(),
            bot_runtime: shared_bot_runtime(),
        }
    }

    pub fn with_admin(auth: HttpAdminAuth) -> Self {
        Self {
            admin_auth: auth,
            order_executor: HttpOrderExecutor::fail_closed(),
            bot_runtime: Arc::new(crate::modules::bots::FailClosedBotRuntime),
        }
    }

    #[cfg(test)]
    pub fn with_order_executor(auth: HttpAdminAuth, order_executor: HttpOrderExecutor) -> Self {
        Self {
            admin_auth: auth,
            order_executor,
            bot_runtime: Arc::new(crate::modules::bots::FailClosedBotRuntime),
        }
    }

    #[cfg(test)]
    pub fn with_bot_runtime(auth: HttpAdminAuth, bot_runtime: Arc<dyn BotRuntimePort>) -> Self {
        Self {
            admin_auth: auth,
            order_executor: HttpOrderExecutor::fail_closed(),
            bot_runtime,
        }
    }

    /// Same bot runtime pointer as `serve` (`ApiState::for_http_server` → `HttpApiSeams::from_env`).
    #[cfg(test)]
    pub fn serve_aligned() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod http_api_seams_tests {
    use super::HttpApiSeams;
    use crate::modules::bots::shared_bot_runtime;
    use std::sync::Arc;

    #[test]
    fn from_env_shares_process_wide_bot_runtime_with_serve() {
        let seams = HttpApiSeams::from_env();
        let shared = shared_bot_runtime();
        assert!(Arc::ptr_eq(&seams.bot_runtime, &shared));
    }

    #[test]
    fn serve_aligned_matches_from_env_bot_runtime() {
        let from_env = HttpApiSeams::from_env().bot_runtime;
        let aligned = HttpApiSeams::serve_aligned().bot_runtime;
        assert!(Arc::ptr_eq(&from_env, &aligned));
    }
}

#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

pub struct ApiStateInner {
    pub monitor: Option<MonitorHandle>,
    pub databases: AppDatabases,
    pub agents: Arc<std::sync::Mutex<AgentRegistry>>,
    pub bot_catalog: Arc<tokio::sync::Mutex<BotCatalogBackend>>,
    pub jev: Option<JevAdvisor>,
    pub app_config: Config,
    pub http_admin_auth: HttpAdminAuth,
    pub order_executor: HttpOrderExecutor,
    pub order_idempotency: Arc<InMemoryOrderIdempotencyStore>,
    pub order_idempotency_pg: Option<PgOrderIdempotencyStore>,
    pub order_reconciliation: Arc<InMemoryOrderReconciliationLedger>,
    pub order_reconciliation_pg: Option<PgOrderReconciliationStore>,
    pub bot_runtime: Arc<dyn BotRuntimePort>,
    pub verified_product_owner: Option<VerifiedProductOwner>,
}

impl ApiState {
    pub fn new(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::disabled_fail_closed(),
            None,
        )
    }

    pub fn with_agent_registry(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        http_admin_auth: HttpAdminAuth,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::with_admin(http_admin_auth),
            None,
        )
    }

    pub fn with_agent_registry_and_verified_owner(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        http_admin_auth: HttpAdminAuth,
        verified_product_owner: VerifiedProductOwner,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::with_admin(http_admin_auth),
            Some(verified_product_owner),
        )
    }
    /// Bootstrap `ApiState` for the HTTP server (admin + order execution from environment).
    pub fn for_http_server(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        verified_product_owner: Option<VerifiedProductOwner>,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::from_env(),
            verified_product_owner,
        )
    }

    /// Composition root used by `server::run`: hydrate agents from PG, wire env seams, hydrate reconciliation, persist catalog.
    pub async fn build_api_state_for_http_serve(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
    ) -> Self {
        use crate::modules::http_bridge::agents::{
            apply_agent_identity_snapshot, load_agent_identity_snapshot,
        };

        if let Some(db) = databases.postgres_handle() {
            match load_agent_identity_snapshot(db.as_postgres()).await {
                Ok((loaded_agents, audit)) if !loaded_agents.is_empty() => {
                    let mut guard = agents.lock().expect("agent registry lock poisoned");
                    if let Err(error) =
                        apply_agent_identity_snapshot(&mut guard, loaded_agents, audit)
                    {
                        tracing::warn!(
                            target: "api",
                            %error,
                            "agent registry restore from PostgreSQL failed"
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(target: "api", %error, "agent registry load from PostgreSQL failed");
                }
            }
        }
        let bootstrap_config = ProductOwnerBootstrapConfig::from_env();
        let mut verified_product_owner = None;
        if let Some(db) = databases.postgres_handle() {
            let pool = db.as_postgres().pool();
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            match ensure_product_owner_bootstrapped(pool, &bootstrap_config, now_ms).await {
                Ok(owner_opt) => {
                    verified_product_owner = owner_opt.map(VerifiedProductOwner::new);
                }
                Err(error) => {
                    tracing::error!(target: "api", %error, "product owner bootstrap failed");
                    if let Ok(Some(owner)) =
                        crate::modules::agents::adapters::load_bootstrapped_owner_id(pool).await
                    {
                        verified_product_owner = Some(VerifiedProductOwner::new(owner));
                    }
                }
            }
        }
        let jev = JevAdvisor::from_env(app_config.jev.clone()).ok().flatten();
        let state = Self::for_http_server(
            monitor,
            databases,
            jev,
            app_config,
            agents,
            verified_product_owner,
        );
        if state.database().is_some() {
            if let Err(error) = state.hydrate_order_reconciliation_from_pg().await {
                tracing::warn!(
                    target: "api",
                    %error,
                    "order reconciliation hydrate from PostgreSQL failed"
                );
            }
            if let Err(error) = state.persist_bot_catalog().await {
                tracing::warn!(target: "api", %error, "bot catalog boot persist failed");
            }
        }
        state
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_stores(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        bot_catalog: Arc<tokio::sync::Mutex<BotCatalogBackend>>,
        http_seams: HttpApiSeams,
        verified_product_owner: Option<VerifiedProductOwner>,
    ) -> Self {
        let order_idempotency_pg = databases
            .postgres_handle()
            .map(|db| PgOrderIdempotencyStore::new(db.as_postgres()));
        let order_reconciliation_pg = databases
            .postgres_handle()
            .map(|db| PgOrderReconciliationStore::new(db.as_postgres()));
        if let Some(pg) = order_reconciliation_pg.clone() {
            crate::modules::orders::register_live_reconciliation_pg_mirror(std::sync::Arc::new(pg));
        } else {
            crate::modules::orders::clear_live_reconciliation_pg_mirror();
        }
        Self {
            inner: Arc::new(ApiStateInner {
                monitor,
                databases,
                agents,
                bot_catalog,
                jev,
                app_config,
                http_admin_auth: http_seams.admin_auth,
                order_executor: http_seams.order_executor,
                order_idempotency: Arc::new(InMemoryOrderIdempotencyStore::new()),
                order_idempotency_pg,
                order_reconciliation:
                    crate::modules::orders::shared_live_order_reconciliation_ledger(),
                order_reconciliation_pg,
                bot_runtime: http_seams.bot_runtime,
                verified_product_owner,
            }),
        }
    }

    pub fn bot_runtime_status(&self) -> BotRuntimeStatus {
        bots_runtime::bot_runtime_status(self.inner.bot_runtime.as_ref())
    }

    #[cfg(test)]
    pub fn bot_runtime_arc_for_test(&self) -> Arc<dyn BotRuntimePort> {
        self.inner.bot_runtime.clone()
    }

    pub async fn promote_bot_http(
        &self,
        request: PromoteBotRequest,
    ) -> Result<BotPromotionRecord, ApiError> {
        request.validate().map_err(ApiError::from_bots_error)?;
        if let Some(agency_raw) = self.inner.http_admin_auth.bound_agency_id_opt() {
            let agency = crate::modules::agents::AgencyId::new(agency_raw)
                .map_err(ApiError::from_agents_error)?;
            let agent_owner = self
                .with_agents(|registry| {
                    crate::modules::agents::assert_runtime_promotion_authorized(
                        registry,
                        &agency,
                        &request.promoted_by,
                        &request.bot_id,
                    )?;
                    let agent_id = crate::modules::agents::AgentId::new(&request.promoted_by)?;
                    let agent = registry.get(&agency, &agent_id)?;
                    Ok(agent.owner.as_str().to_string())
                })
                .await
                .map_err(ApiError::from_agents_error)?;
            crate::presentation::http::register_owner::verify_promoted_by_product_owner(
                self.inner.verified_product_owner.as_ref(),
                &request.promoted_by,
                Some(agent_owner.as_str()),
            )?;
        } else {
            crate::presentation::http::register_owner::verify_promoted_by_product_owner(
                self.inner.verified_product_owner.as_ref(),
                &request.promoted_by,
                None,
            )?;
        }
        let config = self.app_config().clone();
        let catalog = self.inner.bot_catalog.lock().await;
        crate::modules::http_bridge::bots::assert_bot_promotion_allowed(
            &*catalog,
            &config,
            &request.bot_id,
        )
        .await
        .map_err(ApiError::from_bot_error)?;
        let agency_id = self.inner.http_admin_auth.bound_agency_id_opt();
        let record = bots_runtime::promote_bot(self.inner.bot_runtime.as_ref(), request)
            .map_err(ApiError::from_bots_error)?;
        if let Some(db) = self.database() {
            let store = crate::modules::bots::adapters::PgBotCatalogStore::new(db.as_postgres());
            store
                .enqueue_bot_promotion_graph_projection(&record, agency_id)
                .await
                .map_err(|error| ApiError::from_bot_error(BotError::Configuration(error)))?;
            crate::core::database::graph_projection_drain_best_effort(
                self.graph_projection_sync(),
                1,
            )
            .await;
        } else {
            crate::modules::bots::adapters::graph_projection::best_effort_project_bot_promotion(
                self.graph_projection_sync(),
                &record,
                agency_id,
            )
            .await;
        }
        Ok(record)
    }

    pub async fn demote_bot_http(&self) -> Result<(), crate::modules::bots::BotsError> {
        let previous = self.bot_runtime_status().active;
        bots_runtime::demote_bot(self.inner.bot_runtime.as_ref())?;
        if let Some(active) = previous {
            if let Some(db) = self.database() {
                let store =
                    crate::modules::bots::adapters::PgBotCatalogStore::new(db.as_postgres());
                store
                    .enqueue_bot_demotion_graph_projection(&active.bot_id)
                    .await
                    .map_err(crate::modules::bots::BotsError::CatalogStore)?;
                crate::core::database::graph_projection_drain_best_effort(
                    self.graph_projection_sync(),
                    1,
                )
                .await;
            } else {
                crate::modules::bots::adapters::graph_projection::best_effort_retract_bot_promotion(
                    self.graph_projection_sync(),
                    &active.bot_id,
                )
                .await;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn with_bot_runtime(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        http_admin_auth: HttpAdminAuth,
        bot_runtime: Arc<dyn BotRuntimePort>,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::with_bot_runtime(http_admin_auth, bot_runtime),
            None,
        )
    }

    #[cfg(test)]
    pub fn with_order_executor(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        http_admin_auth: HttpAdminAuth,
        order_executor: HttpOrderExecutor,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
            HttpApiSeams::with_order_executor(http_admin_auth, order_executor),
            None,
        )
    }

    pub fn paper_wallet_snapshot(
        &self,
        query: crate::modules::http_bridge::portfolio::PaperSnapshotQuery,
    ) -> Result<crate::modules::http_bridge::portfolio::PaperSnapshotResponse, BotError> {
        crate::modules::http_bridge::portfolio::paper_wallet_snapshot(query)
    }

    pub async fn submit_order_http(
        &self,
        body: SubmitOrderHttpRequest,
    ) -> Result<SubmitOrderResponse, crate::modules::orders::OrdersError> {
        use crate::modules::http_bridge::orders::validate_client_order_id;
        use crate::modules::orders::OrderIdempotencyStore;

        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::orders::OrderSide;

        validate_client_order_id(body.client_order_id.as_deref())?;
        let symbol = body.symbol.clone();
        let order_side = match body.side {
            OrderSideBody::Buy => OrderSide::Buy,
            OrderSideBody::Sell => OrderSide::Sell,
        };
        let idem_key = body
            .client_order_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let mut pg_claimed_key: Option<String> = None;
        if let Some(ref key) = idem_key {
            if OrderIdempotencyStore::is_completed(&*self.inner.order_idempotency, key) {
                return Ok(SubmitOrderResponse { accepted: true });
            }
            if let Some(pg) = &self.inner.order_idempotency_pg {
                if pg.is_completed(key).await? {
                    OrderIdempotencyStore::record_completed(&*self.inner.order_idempotency, key);
                    return Ok(SubmitOrderResponse { accepted: true });
                }
                if !pg.try_claim(key).await? {
                    OrderIdempotencyStore::record_completed(&*self.inner.order_idempotency, key);
                    return Ok(SubmitOrderResponse { accepted: true });
                }
                pg_claimed_key = Some(key.clone());
            }
        }

        let track_live_reconciliation = self.inner.order_executor.live_exchange_wired();
        let reconciliation = if track_live_reconciliation {
            Some(&*self.inner.order_reconciliation as &dyn OrderReconciliationLedger)
        } else {
            None
        };

        let response = match crate::modules::http_bridge::orders::submit_order_http(
            body,
            &self.inner.order_executor,
            &*self.inner.order_idempotency,
            reconciliation,
        ) {
            Ok(response) => response,
            Err(error) => {
                if let (Some(pg), Some(key)) = (&self.inner.order_idempotency_pg, &pg_claimed_key) {
                    pg.release_claim(key).await?;
                }
                return Err(error);
            }
        };

        if track_live_reconciliation {
            if let Some(key) = &idem_key {
                if let Some(ack) = take_last_spot_submit_ack() {
                    crate::modules::orders::recording_bind_client_exchange(
                        key,
                        &ack.exchange_order_id,
                    );
                    self.inner
                        .order_reconciliation
                        .confirm_exchange_order(key, &ack.exchange_order_id)?;
                }
            }
        }

        if track_live_reconciliation {
            if let (Some(pg), Some(key)) = (&self.inner.order_reconciliation_pg, &idem_key) {
                if let Some(rec_state) = self.inner.order_reconciliation.state(key) {
                    pg.upsert_state(key, &symbol, order_side, &rec_state)
                        .await?;
                }
            }
        }

        if let Some(key) = idem_key.as_deref() {
            let snapshot = crate::modules::orders::adapters::RedactedOrderSubmitSnapshot {
                client_order_id: key.to_string(),
                symbol: symbol.clone(),
                side: order_side,
                execution_mode: self.order_execution_mode().as_api_label().to_string(),
                submitting_bot_id: None,
            };
            if pg_claimed_key.is_some() {
                if let Some(pg) = &self.inner.order_idempotency_pg {
                    let messages =
                        crate::modules::orders::adapters::order_graph_projection_outbox_messages(
                            &snapshot,
                        );
                    pg.persist_idempotency_and_enqueue_graph_projection(key, &messages)
                        .await?;
                    crate::core::database::graph_projection_drain_best_effort(
                        self.graph_projection_sync(),
                        messages.len(),
                    )
                    .await;
                }
            } else {
                crate::modules::orders::adapters::best_effort_project_order_intent(
                    self.graph_projection_sync(),
                    &snapshot,
                )
                .await;
            }
        }

        Ok(response)
    }

    pub fn monitor(&self) -> Option<&MonitorHandle> {
        self.inner.monitor.as_ref()
    }

    pub fn databases(&self) -> &AppDatabases {
        &self.inner.databases
    }

    pub fn database(&self) -> Option<&Database> {
        self.inner.databases.postgres_handle()
    }

    fn graph_projection_sync(&self) -> GraphProjectionSync<'_> {
        GraphProjectionSync {
            postgres: self.database().map(|db| db.as_postgres()),
            neo4j: self.inner.databases.neo4j(),
        }
    }

    pub fn jev(&self) -> Option<&JevAdvisor> {
        self.inner.jev.as_ref()
    }

    pub fn app_config(&self) -> &Config {
        &self.inner.app_config
    }

    pub fn http_admin_auth_enabled(&self) -> bool {
        self.inner.http_admin_auth.enabled()
    }

    pub fn http_owner_binding_active(&self) -> bool {
        self.inner.http_admin_auth.owner_binding_active()
    }

    pub fn product_owner_bootstrap_active(&self) -> bool {
        self.inner.verified_product_owner.is_some()
    }

    pub fn http_agency_binding_active(&self) -> bool {
        self.inner.http_admin_auth.agency_binding_active()
    }

    pub fn order_execution_mode(
        &self,
    ) -> crate::presentation::http::order_execution::HttpOrderExecutionMode {
        self.inner.order_executor.mode()
    }

    pub fn live_exchange_wired(&self) -> bool {
        self.inner.order_executor.live_exchange_wired()
    }

    pub fn order_reconciliation_pending_count(&self) -> usize {
        self.inner.order_reconciliation.pending_count()
    }

    /// In-process count; when PostgreSQL is wired, uses the max of memory and durable pending rows.
    pub async fn order_reconciliation_pending_count_observed(&self) -> usize {
        let in_process = self.inner.order_reconciliation.pending_count();
        if let Some(pg) = &self.inner.order_reconciliation_pg {
            match pg.pending_count().await {
                Ok(pg_count) => in_process.max(pg_count),
                Err(_) => in_process,
            }
        } else {
            in_process
        }
    }

    pub fn order_reconciliation_state(&self, client_order_id: &str) -> Option<ReconciliationState> {
        self.inner.order_reconciliation.state(client_order_id)
    }

    /// Best-effort: load durable reconciliation rows into the in-process ledger after HTTP boot.
    pub async fn hydrate_order_reconciliation_from_pg(
        &self,
    ) -> Result<(), crate::modules::orders::OrdersError> {
        if let Some(pg) = &self.inner.order_reconciliation_pg {
            let rows = pg.list_for_memory_hydrate().await?;
            for (client_order_id, symbol, side, state) in rows {
                self.inner.order_reconciliation.seed_hydrated_row(
                    &client_order_id,
                    &symbol,
                    side,
                    state,
                );
            }
        }
        Ok(())
    }

    pub async fn reconcile_pending_orders_once(
        &self,
    ) -> Result<
        crate::modules::orders::ReconciliationPollSummary,
        crate::modules::orders::OrdersError,
    > {
        use crate::modules::orders::{
            run_reconciliation_poll_once, LiveExchangeSpotOrderReconciliationQuery,
        };

        if !self.live_exchange_wired() {
            return Ok(crate::modules::orders::ReconciliationPollSummary::default());
        }
        let pending_before = self.inner.order_reconciliation.list_pending();
        let summary = run_reconciliation_poll_once(
            &*self.inner.order_reconciliation,
            &LiveExchangeSpotOrderReconciliationQuery,
        )?;
        if let Some(pg) = &self.inner.order_reconciliation_pg {
            for item in pending_before {
                if let Some(rec_state) =
                    self.inner.order_reconciliation.state(&item.client_order_id)
                {
                    if !matches!(rec_state, ReconciliationState::Pending) {
                        pg.upsert_state(&item.client_order_id, &item.symbol, item.side, &rec_state)
                            .await?;
                    }
                }
            }
        }
        Ok(summary)
    }

    pub async fn order_reconciliation_lookup(
        &self,
        client_order_id: &str,
    ) -> Result<Option<ReconciliationState>, crate::modules::orders::OrdersError> {
        if let Some(state) = self.inner.order_reconciliation.state(client_order_id) {
            return Ok(Some(state));
        }
        if let Some(pg) = &self.inner.order_reconciliation_pg {
            return pg.state(client_order_id).await;
        }
        Ok(None)
    }

    /// Alias for [`reconcile_pending_orders_once`] (background poll + manual HTTP).
    pub async fn run_order_reconciliation_poll_once(
        &self,
    ) -> Result<ReconciliationPollSummary, crate::modules::orders::OrdersError> {
        self.reconcile_pending_orders_once().await
    }

    pub fn bot_catalog(&self) -> &Arc<tokio::sync::Mutex<BotCatalogBackend>> {
        &self.inner.bot_catalog
    }

    pub async fn with_bot_catalog<F, Fut, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(&'a mut BotCatalogBackend) -> Fut,
        Fut: std::future::Future<Output = R> + Send,
        R: Send,
    {
        let mut guard = self.inner.bot_catalog.lock().await;
        f(&mut guard).await
    }

    pub async fn with_agents<R>(&self, f: impl FnOnce(&mut AgentRegistry) -> R) -> R {
        let mut guard = self
            .inner
            .agents
            .lock()
            .expect("agent registry lock poisoned");
        f(&mut guard)
    }

    pub fn require_http_admin(&self, headers: &axum::http::HeaderMap) -> Result<(), ApiError> {
        self.inner.http_admin_auth.verify_headers(headers)
    }

    pub fn require_register_owner_id(&self, owner_id: &str) -> Result<(), ApiError> {
        crate::presentation::http::register_owner::verify_register_owner_id(
            self.inner.verified_product_owner.as_ref(),
            &self.inner.http_admin_auth,
            owner_id,
        )
    }

    pub async fn persist_bot_catalog(&self) -> Result<BotCatalogPersistResponse, BotError> {
        let config = self.app_config().clone();
        let postgres_wired = self.database().is_some();
        let mut guard = self.inner.bot_catalog.lock().await;
        let response =
            crate::modules::http_bridge::bots::persist_catalog_for_config(&config, &mut *guard)
                .await?;
        let entries = guard
            .load_catalog()
            .await
            .map_err(BotError::Configuration)?;
        if postgres_wired {
            crate::core::database::graph_projection_drain_best_effort(
                self.graph_projection_sync(),
                entries.len(),
            )
            .await;
        } else {
            crate::modules::bots::adapters::graph_projection::best_effort_project_bot_catalog(
                self.graph_projection_sync(),
                &entries,
            )
            .await;
        }
        Ok(response)
    }

    pub fn monitor_snapshot(&self) -> Result<MonitorSnapshotResponse, ApiError> {
        let handle = self.monitor().ok_or_else(ApiError::monitor_unavailable)?;
        let response = crate::modules::http_bridge::monitor::snapshot_response(handle);
        Ok(
            crate::modules::http_bridge::monitor::attach_bot_runtime_status(
                response,
                self.bot_runtime_status(),
            ),
        )
    }

    pub fn accept_monitor_command(&self, body: MonitorCommandRequest) -> Result<(), ApiError> {
        let handle = self.monitor().ok_or_else(ApiError::monitor_unavailable)?;
        crate::modules::http_bridge::monitor::send_monitor_command(handle, body).map_err(|error| {
            match error {
                MonitorCommandHttpError::ChannelFull => ApiError::new(
                    axum::http::StatusCode::CONFLICT,
                    "monitor command channel full",
                ),
                MonitorCommandHttpError::ChannelClosed => ApiError::new(
                    axum::http::StatusCode::CONFLICT,
                    "monitor command channel closed",
                ),
            }
        })
    }

    pub async fn bot_catalog_snapshot(&self) -> Result<BotCatalogResponse, BotError> {
        let guard = self.inner.bot_catalog.lock().await;
        crate::modules::http_bridge::bots::catalog_from_store(&*guard, self.app_config()).await
    }

    pub fn require_bound_agency(&self, agency_id: &str) -> Result<(), ApiError> {
        self.inner.http_admin_auth.verify_agency_id(agency_id)
    }

    pub async fn list_agents_in_agency(&self, agency: &str) -> Result<AgentListResponse, ApiError> {
        self.with_agents(|registry| {
            crate::modules::http_bridge::agents::list_agents(registry, agency)
        })
        .await
        .map_err(ApiError::from_agents_error)
    }

    pub async fn get_agent_in_agency(
        &self,
        agency: &str,
        agent_id: &str,
    ) -> Result<AgentResponse, ApiError> {
        self.with_agents(|registry| {
            crate::modules::http_bridge::agents::get_agent(registry, agency, agent_id)
        })
        .await
        .map_err(ApiError::from_agents_error)
    }

    pub async fn agents_audit_log(&self, agency: &str) -> Result<AuditLogResponse, ApiError> {
        self.with_agents(|registry| {
            crate::modules::http_bridge::agents::audit_log(registry, agency)
        })
        .await
        .map_err(ApiError::from_agents_error)
    }

    pub fn active_config_snapshot(&self) -> ConfigSnapshotResponse {
        crate::modules::http_bridge::config::map_config(self.app_config())
    }

    pub fn config_snapshot_from_path(
        &self,
        query: crate::modules::http_bridge::config::ConfigSnapshotQuery,
    ) -> Result<ConfigSnapshotResponse, BotError> {
        crate::modules::http_bridge::config::load_config_snapshot(query)
    }

    pub fn providers_status_snapshot(&self) -> ProvidersStatusResponse {
        crate::modules::http_bridge::providers::status_from_config(self.app_config())
    }

    pub fn bot_ranking_from_metrics(
        &self,
        metrics: Vec<crate::modules::bots::BotMetrics>,
    ) -> Result<crate::modules::http_bridge::bots::BotRankingResponse, BotError> {
        crate::modules::http_bridge::bots::ranking_from_metrics(metrics)
    }

    pub fn bot_catalog_for_config(&self) -> Result<BotCatalogResponse, BotError> {
        crate::modules::http_bridge::bots::catalog_for_config(self.app_config())
    }

    pub async fn register_agent_and_persist(
        &self,
        body: RegisterAgentRequest,
    ) -> Result<AgentResponse, ApiError> {
        let agency = body.agency.clone();
        let agent_id = body.agent_id.clone();
        let response = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::register_agent(registry, body)
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        self.persist_agent_after_mutation(&agency, &agent_id)
            .await?;
        Ok(response)
    }

    pub async fn pause_agent_and_persist(
        &self,
        agency: &str,
        agent_id: &str,
    ) -> Result<LifecycleResponse, ApiError> {
        let response = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::pause(registry, agency, agent_id)
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        self.persist_agent_after_mutation(agency, agent_id).await?;
        Ok(response)
    }

    pub async fn resume_agent_and_persist(
        &self,
        agency: &str,
        agent_id: &str,
    ) -> Result<LifecycleResponse, ApiError> {
        let response = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::resume(registry, agency, agent_id)
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        self.persist_agent_after_mutation(agency, agent_id).await?;
        Ok(response)
    }

    pub async fn retire_agent_and_persist(
        &self,
        agency: &str,
        agent_id: &str,
    ) -> Result<LifecycleResponse, ApiError> {
        let response = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::retire(registry, agency, agent_id)
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        self.persist_agent_after_mutation(agency, agent_id).await?;
        Ok(response)
    }

    pub async fn run_agent_advisory(
        &self,
        agent_id: &str,
        body: AdvisoryRequest,
    ) -> Result<AdvisoryResponse, ApiError> {
        let advisor = self.jev().cloned().ok_or_else(ApiError::jev_unavailable)?;
        let step = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::advisory_prepare(registry, agent_id, body)
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        crate::modules::http_bridge::agents::advisory_finish(&advisor, step)
            .await
            .map_err(ApiError::from_bot_error)
    }

    pub async fn persist_agent_after_mutation(
        &self,
        agency: &str,
        agent_id: &str,
    ) -> Result<(), ApiError> {
        let postgres = self.database().map(|db| db.as_postgres());
        let snapshot = self
            .with_agents(|registry| {
                crate::modules::http_bridge::agents::snapshot_for_persist(
                    registry, agency, agent_id,
                )
            })
            .await
            .map_err(ApiError::from_agents_error)?;
        if let Some(pg) = postgres {
            crate::modules::http_bridge::agents::persist_identity_rows(
                pg,
                &snapshot.0,
                &snapshot.1,
            )
            .await
            .map_err(ApiError::from_agents_error)?;
            crate::core::database::graph_projection_drain_best_effort(
                self.graph_projection_sync(),
                1,
            )
            .await;
        } else {
            crate::modules::agents::adapters::graph_projection::best_effort_project_agent_definition(
                self.graph_projection_sync(),
                &snapshot.0,
            )
            .await;
        }
        Ok(())
    }
}

#[cfg(test)]
impl ApiState {
    pub fn test_seed_order_reconciliation(
        &self,
        client_order_id: &str,
        state: ReconciliationState,
    ) {
        self.inner
            .order_reconciliation
            .seed_entry(client_order_id, state);
    }

    pub fn test_mark_reconciliation_pending(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: crate::modules::orders::OrderSide,
    ) {
        self.inner
            .order_reconciliation
            .mark_pending(client_order_id, symbol, side)
            .expect("test pending");
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
        )
    }
}

#[cfg(test)]
mod state_tests {
    #![allow(clippy::await_holding_lock)]

    use crate::modules::orders::lock_shared_live_order_reconciliation_ledger_for_test;

    use super::*;
    use crate::modules::agents::AgentRegistry;
    use std::sync::Arc;

    #[tokio::test]
    async fn persist_bot_catalog_then_snapshot_round_trip_in_memory() {
        let state = ApiState::default();
        let persisted = state.persist_bot_catalog().await.expect("persist");
        assert!(persisted.persisted);
        assert!(!persisted.bots.is_empty());
        let snapshot = state.bot_catalog_snapshot().await.expect("snapshot");
        assert_eq!(snapshot.bots.len(), persisted.bots.len());
    }

    #[tokio::test]
    async fn pg_bot_catalog_snapshot_round_trip_via_api_state() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let config = crate::modules::config_api::Config::default();
        let databases = AppDatabases {
            postgres: Some(db.clone()),
            neo4j: None,
        };
        let state = ApiState::new(None, databases, None, config.clone());
        let persisted = state.persist_bot_catalog().await.expect("persist");
        assert!(persisted.persisted);
        assert!(!persisted.bots.is_empty());

        let cold = ApiState::new(
            None,
            AppDatabases {
                postgres: Some(db),
                neo4j: None,
            },
            None,
            config,
        );
        let snapshot = cold.bot_catalog_snapshot().await.expect("snapshot");
        assert_eq!(snapshot.bots.len(), persisted.bots.len());
        let persisted_ids: std::collections::HashSet<_> =
            persisted.bots.iter().map(|b| b.bot_id.as_str()).collect();
        for entry in &snapshot.bots {
            assert!(
                persisted_ids.contains(entry.bot_id.as_str()),
                "unexpected bot_id in PG snapshot: {}",
                entry.bot_id
            );
        }
    }

    #[tokio::test]
    async fn persist_bot_catalog_includes_monitor_registry_v2_entries() {
        use crate::modules::http_bridge::config::MonitorStrategyConfigEntry;
        let mut config = crate::modules::config_api::Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let state = ApiState::new(None, AppDatabases::empty(), None, config);
        let persisted = state.persist_bot_catalog().await.expect("persist");
        assert!(persisted.persisted);
        let v2 = persisted
            .bots
            .iter()
            .find(|entry| entry.strategy_version == 2)
            .expect("v2 catalog row after persist_monitor_catalog_snapshot");
        assert_eq!(v2.monitor_fast_period, 3);
        let snapshot = state.bot_catalog_snapshot().await.expect("snapshot");
        assert!(snapshot
            .bots
            .iter()
            .any(|entry| entry.strategy_version == 2));
    }

    #[tokio::test]
    async fn persist_catalog_then_promote_monitor_registry_v2_bot() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::bots::{BotPromotionState, InMemoryBotRuntime, PromoteBotRequest};
        use crate::modules::http_bridge::config::MonitorStrategyConfigEntry;
        use std::sync::Arc;

        let mut config = crate::modules::config_api::Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let active_tf = config.market.timeframe.clone();
        let runtime = Arc::new(InMemoryBotRuntime::new());
        let state = ApiState::with_bot_runtime(
            None,
            AppDatabases::empty(),
            None,
            config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            runtime.clone(),
        );
        let persisted = state.persist_bot_catalog().await.expect("persist");
        let bot_id = persisted
            .bots
            .iter()
            .find(|entry| entry.strategy_version == 2 && entry.timeframe == active_tf)
            .map(|entry| entry.bot_id.clone())
            .expect("v2 bot for active market timeframe");
        let record = state
            .promote_bot_http(PromoteBotRequest {
                bot_id: bot_id.clone(),
                promoted_by: "owner-1".into(),
            })
            .await
            .expect("promote v2 after persist");
        assert_eq!(record.state, BotPromotionState::Active);
        assert_eq!(
            state
                .bot_runtime_status()
                .active
                .as_ref()
                .map(|r| r.bot_id.as_str()),
            Some(bot_id.as_str())
        );
        let binding = crate::modules::bots::strategy_evaluation_binding_with_runtime(
            state.app_config(),
            runtime.as_ref(),
        );
        assert_eq!(binding.sma_fast, 3);
        assert_eq!(binding.sma_slow, 15);
        assert_eq!(binding.promoted_bot_id.as_deref(), Some(bot_id.as_str()));
    }

    #[tokio::test]
    async fn persist_catalog_then_promote_bot_http_with_in_memory_runtime() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::bots::{BotPromotionState, InMemoryBotRuntime, PromoteBotRequest};
        use std::sync::Arc;

        let config = crate::modules::config_api::Config::default();
        let active_tf = config.market.timeframe.clone();
        let state = ApiState::with_bot_runtime(
            None,
            AppDatabases::empty(),
            None,
            config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            Arc::new(InMemoryBotRuntime::new()),
        );
        let persisted = state.persist_bot_catalog().await.expect("persist");
        let bot_id = persisted
            .bots
            .iter()
            .find(|entry| entry.timeframe == active_tf)
            .map(|entry| entry.bot_id.clone())
            .expect("catalog entry for active timeframe");
        let record = state
            .promote_bot_http(PromoteBotRequest {
                bot_id,
                promoted_by: "owner-1".into(),
            })
            .await
            .expect("promote after catalog persist");
        assert_eq!(record.state, BotPromotionState::Active);
        assert!(state.bot_runtime_status().active.is_some());
    }

    #[tokio::test]
    async fn promote_bot_http_enforces_agent_capability_when_agency_bound() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::bots::{InMemoryBotRuntime, PromoteBotRequest};
        use crate::modules::http_bridge::agents::{
            AgentRoleBody, RegisterAgentRequest, SupervisorRefBody,
        };
        use std::sync::Arc;

        let config = crate::modules::config_api::Config::default();
        let active_tf = config.market.timeframe.clone();
        let agents = Arc::new(std::sync::Mutex::new(AgentRegistry::new()));
        let state = ApiState::with_bot_runtime(
            None,
            AppDatabases::empty(),
            None,
            config,
            agents,
            HttpAdminAuth::for_test_bound_agency("bound-agency"),
            Arc::new(InMemoryBotRuntime::new()),
        );
        state
            .register_agent_and_persist(RegisterAgentRequest {
                agency: "bound-agency".into(),
                owner_id: "owner-1".into(),
                agent_id: "ceo".into(),
                display_name: "CEO".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-1".into(),
                },
                consult_jev: false,
                promote_runtime_bot: false,
            })
            .await
            .expect("register ceo");
        let persisted = state.persist_bot_catalog().await.expect("catalog");
        let bot_id = persisted
            .bots
            .iter()
            .find(|entry| entry.timeframe == active_tf)
            .map(|entry| entry.bot_id.clone())
            .expect("bot for timeframe");
        let denied = state
            .promote_bot_http(PromoteBotRequest {
                bot_id: bot_id.clone(),
                promoted_by: "ceo".into(),
            })
            .await
            .expect_err("ceo lacks promote_runtime_bot");
        assert_eq!(denied.status_code(), axum::http::StatusCode::FORBIDDEN);
        state
            .register_agent_and_persist(RegisterAgentRequest {
                agency: "bound-agency".into(),
                owner_id: "owner-1".into(),
                agent_id: "promoter".into(),
                display_name: "Promoter".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-1".into(),
                },
                consult_jev: false,
                promote_runtime_bot: true,
            })
            .await
            .expect("register promoter");
        state
            .promote_bot_http(PromoteBotRequest {
                bot_id,
                promoted_by: "promoter".into(),
            })
            .await
            .expect("promoter authorized");
    }

    #[tokio::test]
    async fn promote_bot_http_rejects_owner_mismatch_when_product_owner_verified() {
        use crate::modules::agents::{AgentRegistry, VerifiedProductOwner};
        use crate::modules::bots::{InMemoryBotRuntime, PromoteBotRequest};
        use std::sync::Arc;

        let config = crate::modules::config_api::Config::default();
        let active_tf = config.market.timeframe.clone();
        let state = ApiState::with_stores(
            None,
            AppDatabases::empty(),
            None,
            config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            Arc::new(tokio::sync::Mutex::new(
                crate::modules::bots::BotCatalogBackend::from_databases(&AppDatabases::empty()),
            )),
            HttpApiSeams::with_bot_runtime(
                HttpAdminAuth::disabled(),
                Arc::new(InMemoryBotRuntime::new()),
            ),
            Some(VerifiedProductOwner::for_test("owner-verified")),
        );
        let persisted = state.persist_bot_catalog().await.expect("catalog");
        let bot_id = persisted
            .bots
            .iter()
            .find(|entry| entry.timeframe == active_tf)
            .map(|entry| entry.bot_id.clone())
            .expect("bot for timeframe");
        let err = state
            .promote_bot_http(PromoteBotRequest {
                bot_id,
                promoted_by: "not-the-owner".into(),
            })
            .await
            .expect_err("promoted_by must match bootstrapped owner");
        assert_eq!(err.status_code(), axum::http::StatusCode::FORBIDDEN);
        assert_eq!(err.error_code(), Some("owner_mismatch"));
    }

    #[tokio::test]
    async fn demote_bot_http_clears_in_memory_runtime_after_promote() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::bots::{InMemoryBotRuntime, PromoteBotRequest};
        use std::sync::Arc;

        let config = crate::modules::config_api::Config::default();
        let active_tf = config.market.timeframe.clone();
        let state = ApiState::with_bot_runtime(
            None,
            AppDatabases::empty(),
            None,
            config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            Arc::new(InMemoryBotRuntime::new()),
        );
        let persisted = state.persist_bot_catalog().await.expect("catalog");
        let bot_id = persisted
            .bots
            .iter()
            .find(|entry| entry.timeframe == active_tf)
            .map(|entry| entry.bot_id.clone())
            .expect("bot for timeframe");
        state
            .promote_bot_http(PromoteBotRequest {
                bot_id,
                promoted_by: "owner-1".into(),
            })
            .await
            .expect("promote");
        assert!(state.bot_runtime_status().active.is_some());
        state.demote_bot_http().await.expect("demote");
        assert!(state.bot_runtime_status().active.is_none());
    }

    #[test]
    fn bot_catalog_for_config_materializes_registry_strategies_with_periods() {
        use crate::modules::bots::{enumerate_timeframes_for_mode, MonitorStrategyRegistry};
        let state = ApiState::default();
        let config = state.app_config();
        let catalog = state.bot_catalog_for_config().expect("catalog");
        let expected_len = enumerate_timeframes_for_mode(config.operation).len()
            * MonitorStrategyRegistry::from_config(config)
                .expect("registry")
                .definitions()
                .len();
        assert_eq!(catalog.bots.len(), expected_len);
        let active_tf = &config.market.timeframe;
        let active = catalog
            .bots
            .iter()
            .find(|b| b.timeframe == *active_tf)
            .expect("entry for active timeframe");
        assert_eq!(active.monitor_fast_period, config.strategy.sma_fast as u32);
        assert_eq!(active.monitor_slow_period, config.strategy.sma_slow as u32);
    }

    #[test]
    fn http_seams_accessors_reflect_defaults() {
        use crate::presentation::http::order_execution::HttpOrderExecutionMode;
        let state = ApiState::default();
        assert!(!state.http_admin_auth_enabled());
        assert!(!state.http_owner_binding_active());
        assert!(!state.http_agency_binding_active());
        assert!(!state.live_exchange_wired());
        assert_eq!(
            state.order_execution_mode(),
            HttpOrderExecutionMode::Disabled
        );
    }

    fn with_orders_env_lock<F: FnOnce()>(f: F) {
        crate::core::test_env_lock::with_env_test_lock(f);
    }

    #[test]
    fn for_http_server_loads_order_executor_from_env() {
        use crate::modules::agents::AgentRegistry;
        use crate::presentation::http::order_execution::HttpOrderExecutionMode;
        with_orders_env_lock(|| {
            std::env::set_var("BOT_ORDERS_EXECUTION", "paper");
            let state = ApiState::for_http_server(
                None,
                AppDatabases::empty(),
                None,
                Config::default(),
                Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
                None,
            );
            assert_eq!(state.order_execution_mode(), HttpOrderExecutionMode::Paper);
            std::env::remove_var("BOT_ORDERS_EXECUTION");
        });
    }

    #[test]
    fn for_http_server_wires_process_wide_bot_runtime_like_serve() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::bots::shared_bot_runtime;
        with_orders_env_lock(|| {
            let state = ApiState::for_http_server(
                None,
                AppDatabases::empty(),
                None,
                Config::default(),
                Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
                None,
            );
            let shared = shared_bot_runtime();
            assert!(std::sync::Arc::ptr_eq(
                &state.bot_runtime_arc_for_test(),
                &shared
            ));
        });
    }

    #[test]
    fn active_config_snapshot_exposes_operation_from_loaded_config() {
        let state = ApiState::default();
        let snap = state.active_config_snapshot();
        assert!(!snap.operation.is_empty());
        assert!(!snap.symbol.is_empty());
    }

    #[test]
    fn active_config_snapshot_includes_monitor_registry_extensions() {
        use crate::modules::http_bridge::config::MonitorStrategyConfigEntry;
        let mut config = crate::modules::config_api::Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let state = ApiState::new(None, AppDatabases::empty(), None, config);
        let snap = state.active_config_snapshot();
        assert_eq!(snap.monitor_registry.len(), 1);
        assert_eq!(snap.monitor_registry[0].version, 2);
    }

    #[test]
    fn providers_status_snapshot_reflects_config() {
        let state = ApiState::default();
        let status = state.providers_status_snapshot();
        assert_eq!(status.jev_enabled, state.app_config().jev.enabled);
    }

    #[tokio::test]
    async fn submit_order_fail_closed_via_state_returns_execution_disabled() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::modules::orders::OrdersError;
        let state = ApiState::default();
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: None,
            paper_fill_unit_price: None,
        };
        let err = state.submit_order_http(body).await.unwrap_err();
        assert!(matches!(err, OrdersError::ExecutionDisabled));
    }

    #[tokio::test]
    async fn submit_order_dev_accept_via_state_returns_accepted() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::presentation::http::order_execution::HttpOrderExecutor;
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::dev_accept(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: None,
            paper_fill_unit_price: None,
        };
        let response = state.submit_order_http(body).await.expect("dev accept");
        assert!(response.accepted);
    }

    #[tokio::test]
    async fn submit_order_invalid_client_order_id_via_state_returns_invalid_request() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::modules::orders::OrdersError;
        use crate::presentation::http::order_execution::HttpOrderExecutor;
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::dev_accept(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: Some("   ".into()),
            paper_fill_unit_price: None,
        };
        let err = state.submit_order_http(body).await.unwrap_err();
        assert!(matches!(err, OrdersError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn pg_submit_order_idempotency_reads_pg_when_memory_empty() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        let key = format!(
            "idem-lookup-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        store.record_completed(&key).await.expect("seed PG");

        let state = ApiState::with_order_executor(
            None,
            AppDatabases {
                postgres: Some(db),
                neo4j: None,
            },
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::dev_accept(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: Some(key),
            paper_fill_unit_price: None,
        };
        let replay = state
            .submit_order_http(body)
            .await
            .expect("PG idempotent accept");
        assert!(replay.accepted);
    }

    #[tokio::test]
    async fn pg_submit_order_idempotency_releases_claim_when_submit_fails() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::modules::orders::{OrdersError, PgOrderIdempotencyStore};
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let key = format!(
            "idem-risk-fail-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let state = ApiState::with_order_executor(
            None,
            AppDatabases {
                postgres: Some(db.clone()),
                neo4j: None,
            },
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::dev_accept(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 100.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: Some(key.clone()),
            paper_fill_unit_price: None,
        };
        let err = state.submit_order_http(body).await.unwrap_err();
        assert!(matches!(err, OrdersError::RiskRejected(_)));

        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        assert!(
            !store
                .is_completed(&key)
                .await
                .expect("lookup after failed submit"),
            "claim must be released when execution does not complete"
        );
    }

    #[test]
    fn bot_ranking_from_metrics_via_api_state_returns_entry() {
        use crate::modules::bots::{
            BotId, BotMetrics, EvaluationWindow, RunId, StrategyId, StrategyVersion,
        };
        let state = ApiState::default();
        let strategy_id = StrategyId::new("sma-cross").expect("strategy");
        let bot_id =
            BotId::new(&strategy_id, StrategyVersion(1), "5m", "BTC/USDT").expect("bot_id");
        let metrics = vec![BotMetrics {
            bot_id,
            timeframe: "5m".into(),
            symbol: "BTC/USDT".into(),
            strategy_id,
            strategy_version: StrategyVersion(1),
            run_id: RunId("r1".into()),
            dataset_hash: "dataset-v1".into(),
            window: EvaluationWindow {
                start_ms: 100,
                end_ms: 200,
            },
            quote_currency: "USDT".into(),
            initial_capital_quote: 1000.0,
            net_pnl_quote: 20.0,
            net_return_pct: 2.0,
            max_drawdown_pct: 5.0,
            trades: 10,
            accuracy_pct: Some(92.0),
        }];
        let response = state.bot_ranking_from_metrics(metrics).expect("ranking");
        assert_eq!(response.report.entries.len(), 1);
        assert_eq!(response.report.entries[0].rank, 1);
    }

    #[test]
    fn active_config_snapshot_aligns_with_default_toml_path() {
        use crate::modules::http_bridge::config::ConfigSnapshotQuery;
        let state = ApiState::default();
        let active = state.active_config_snapshot();
        let from_path = state
            .config_snapshot_from_path(ConfigSnapshotQuery {
                config: "src/core/config/bot.toml".into(),
            })
            .expect("load default path");
        assert_eq!(active.symbol, from_path.symbol);
        assert_eq!(active.timeframe, from_path.timeframe);
        assert_eq!(active.sma_fast, from_path.sma_fast);
        assert_eq!(active.sma_slow, from_path.sma_slow);
    }

    #[test]
    fn config_snapshot_from_path_loads_bundled_default_toml() {
        use crate::modules::http_bridge::config::ConfigSnapshotQuery;
        let state = ApiState::default();
        let snapshot = state
            .config_snapshot_from_path(ConfigSnapshotQuery {
                config: "src/core/config/bot.toml".into(),
            })
            .expect("default bot.toml");
        assert!(!snapshot.symbol.is_empty());
        assert!(!snapshot.timeframe.is_empty());
    }

    #[tokio::test]
    async fn paper_wallet_snapshot_via_api_state_reflects_paper_submit() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::portfolio::PaperSnapshotQuery;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::modules::orders::{PaperLedgerExecutor, PaperLedgerTestGuard};
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        let _paper_ledger = PaperLedgerTestGuard::acquire();
        PaperLedgerExecutor::clear_ledger();
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::paper(),
        );
        state
            .submit_order_http(SubmitOrderHttpRequest {
                symbol: "BTC/USDT".into(),
                side: OrderSideBody::Buy,
                quote_amount: 100.0,
                estimated_daily_loss: 0.0,
                open_positions: 0,
                limits: RiskLimitsBody {
                    max_order_quote: 200.0,
                    max_daily_loss_quote: 20.0,
                    max_open_positions: 1,
                },
                client_order_id: None,
                paper_fill_unit_price: Some(50_000.0),
            })
            .await
            .expect("paper submit");
        let snapshot = state
            .paper_wallet_snapshot(PaperSnapshotQuery {
                quote: "usdt".into(),
            })
            .expect("snapshot");
        assert_eq!(snapshot.available, "900");
        assert_eq!(snapshot.positions.len(), 1);
        PaperLedgerExecutor::clear_ledger();
    }

    #[tokio::test]
    async fn submit_order_client_order_id_replay_via_state_returns_accepted() {
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::presentation::http::order_execution::HttpOrderExecutor;
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::dev_accept(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: Some("state-idem-1".into()),
            paper_fill_unit_price: None,
        };
        let first = state.submit_order_http(body.clone()).await.expect("first");
        assert!(first.accepted);
        let replay = state.submit_order_http(body).await.expect("replay");
        assert!(replay.accepted);
    }

    #[tokio::test]
    async fn submit_order_recording_live_exchange_auto_reconciles_client_order_id() {
        use crate::core::test_env_lock::EnvTestGuard;
        let _env = EnvTestGuard::acquire();
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        use crate::modules::http_bridge::orders::OrderSideBody;
        use crate::modules::http_bridge::risk::RiskLimitsBody;
        use crate::modules::orders::ReconciliationState;
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        crate::modules::orders::RecordingSpotOrderSubmitPort::clear();
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::live_exchange(),
        );
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: Some("state-recording-cid".into()),
            paper_fill_unit_price: None,
        };
        state.submit_order_http(body).await.expect("live recording");
        assert_eq!(state.order_reconciliation_pending_count(), 0);
        match state.order_reconciliation_state("state-recording-cid") {
            Some(ReconciliationState::Reconciled { exchange_order_id }) => {
                assert!(
                    exchange_order_id.starts_with("recording-"),
                    "unexpected exchange_order_id: {exchange_order_id}"
                );
            }
            other => panic!("expected reconciled state, got {other:?}"),
        }
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    }

    #[tokio::test]
    async fn order_reconciliation_pending_count_observed_without_pg_matches_memory() {
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        let state = ApiState::default();
        assert_eq!(state.order_reconciliation_pending_count(), 0);
        assert_eq!(state.order_reconciliation_pending_count_observed().await, 0);
    }

    #[tokio::test]
    async fn hydrate_order_reconciliation_from_pg_noop_without_database() {
        let state = ApiState::default();
        state
            .hydrate_order_reconciliation_from_pg()
            .await
            .expect("noop when PostgreSQL is not wired");
    }

    #[tokio::test]
    async fn pg_hydrate_order_reconciliation_from_pg_after_durable_write() {
        use crate::modules::orders::OrderSide;

        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgOrderReconciliationStore::new(db.as_postgres());
        let key = format!(
            "recon-hydrate-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        store
            .mark_pending(&key, "BTC/USDT", OrderSide::Buy)
            .await
            .expect("pending");
        store
            .confirm_exchange_order(&key, "ex-hydrate-1")
            .await
            .expect("confirm");

        let state = ApiState::new(
            None,
            AppDatabases {
                postgres: Some(db),
                neo4j: None,
            },
            None,
            crate::modules::config_api::Config::default(),
        );
        assert!(
            state.order_reconciliation_state(&key).is_none(),
            "ledger should not have PG-only row before hydrate"
        );
        state
            .hydrate_order_reconciliation_from_pg()
            .await
            .expect("hydrate");
        match state.order_reconciliation_state(&key) {
            Some(ReconciliationState::Reconciled { exchange_order_id }) => {
                assert_eq!(exchange_order_id, "ex-hydrate-1");
            }
            other => panic!("expected reconciled after hydrate, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn reconcile_pending_orders_once_confirms_recording_binding() {
        use crate::core::test_env_lock::EnvTestGuard;
        let _env = EnvTestGuard::acquire();
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        use crate::modules::orders::adapters::clear_recording_client_bindings;
        use crate::modules::orders::{
            recording_bind_client_exchange, OrderSide, ReconciliationState,
        };
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        clear_recording_client_bindings();
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::live_exchange(),
        );
        state.test_mark_reconciliation_pending("poll-http-cid", "BTC/USDT", OrderSide::Buy);
        recording_bind_client_exchange("poll-http-cid", "recording-99");
        let summary = state.reconcile_pending_orders_once().await.expect("poll");
        assert_eq!(summary.confirmed, 1);
        assert_eq!(
            state.order_reconciliation_state("poll-http-cid"),
            Some(ReconciliationState::Reconciled {
                exchange_order_id: "recording-99".into(),
            })
        );
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
        clear_recording_client_bindings();
    }

    #[tokio::test]
    async fn pg_order_reconciliation_lookup_reads_pg_when_memory_empty() {
        use crate::modules::orders::OrderSide;

        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgOrderReconciliationStore::new(db.as_postgres());
        let key = format!(
            "recon-lookup-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        store
            .mark_pending(&key, "ETH/USDT", OrderSide::Sell)
            .await
            .expect("pending");
        store
            .confirm_exchange_order(&key, "ex-lookup-9")
            .await
            .expect("confirm");

        let state = ApiState::new(
            None,
            AppDatabases {
                postgres: Some(db),
                neo4j: None,
            },
            None,
            crate::modules::config_api::Config::default(),
        );
        assert!(state.order_reconciliation_state(&key).is_none());
        match state
            .order_reconciliation_lookup(&key)
            .await
            .expect("lookup")
        {
            Some(ReconciliationState::Reconciled { exchange_order_id }) => {
                assert_eq!(exchange_order_id, "ex-lookup-9");
            }
            other => panic!("expected PG fallback reconciled, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn order_reconciliation_lookup_after_memory_seed_matches_hydrate_contract() {
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        let state = ApiState::default();
        state.test_seed_order_reconciliation("boot-hydrate-cid", ReconciliationState::Pending);
        let found = state
            .order_reconciliation_lookup("boot-hydrate-cid")
            .await
            .expect("lookup");
        assert_eq!(found, Some(ReconciliationState::Pending));
        assert_eq!(state.order_reconciliation_pending_count_observed().await, 1);
    }

    #[tokio::test]
    async fn run_order_reconciliation_poll_once_confirms_stuck_pending_on_live_wired() {
        use crate::core::test_env_lock::EnvTestGuard;
        let _env = EnvTestGuard::acquire();
        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        use crate::modules::agents::AgentRegistry;
        use crate::modules::orders::adapters::clear_recording_client_bindings;
        use crate::modules::orders::{recording_bind_client_exchange, ReconciliationState};
        use crate::presentation::http::order_execution::HttpOrderExecutor;

        clear_recording_client_bindings();
        std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
        let state = ApiState::with_order_executor(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            HttpAdminAuth::disabled(),
            HttpOrderExecutor::live_exchange(),
        );
        state.test_seed_order_reconciliation("poll-api-cid", ReconciliationState::Pending);
        recording_bind_client_exchange("poll-api-cid", "recording-7");
        let summary = state
            .run_order_reconciliation_poll_once()
            .await
            .expect("poll");
        assert_eq!(summary.confirmed, 1);
        assert_eq!(state.order_reconciliation_pending_count(), 0);
        match state.order_reconciliation_state("poll-api-cid") {
            Some(ReconciliationState::Reconciled { exchange_order_id }) => {
                assert_eq!(exchange_order_id, "recording-7");
            }
            other => panic!("expected reconciled, got {other:?}"),
        }
        clear_recording_client_bindings();
        std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
    }

    #[tokio::test]
    async fn build_api_state_for_http_serve_without_database_wires_executor() {
        let agents = Arc::new(std::sync::Mutex::new(AgentRegistry::new()));
        let state = ApiState::build_api_state_for_http_serve(
            None,
            AppDatabases::empty(),
            crate::modules::config_api::Config::default(),
            agents,
        )
        .await;
        let catalog = state.bot_catalog_for_config().expect("catalog");
        assert!(!catalog.bots.is_empty());
    }

    #[tokio::test]
    async fn pg_http_boot_sequence_mirrors_serve_wiring() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::http_bridge::agents::{
            AgentRoleBody, RegisterAgentRequest, SupervisorRefBody,
        };
        use crate::modules::orders::OrderSide;
        use std::sync::{Arc, Mutex};

        use crate::core::config::ProductOwnerBootstrapConfig;
        use crate::modules::agents::adapters::ensure_product_owner_bootstrapped;

        let _ledger_guard = lock_shared_live_order_reconciliation_ledger_for_test();
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        ensure_product_owner_bootstrapped(
            db.pool(),
            &ProductOwnerBootstrapConfig {
                bootstrap_owner_id: Some("owner-boot".into()),
                bootstrap_ack: true,
            },
            1,
        )
        .await
        .expect("product owner bootstrap for serve wiring test");
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let agency = format!("agency-boot-{suffix}");
        let agent_id = format!("ceo-boot-{suffix}");
        let recon_key = format!("recon-boot-{suffix}");

        let warm_agents = Arc::new(Mutex::new(AgentRegistry::new()));
        let config = crate::modules::config_api::Config::default();
        let warm = ApiState::with_agent_registry(
            None,
            AppDatabases {
                postgres: Some(db.clone()),
                neo4j: None,
            },
            None,
            config.clone(),
            warm_agents,
            HttpAdminAuth::disabled(),
        );
        warm.register_agent_and_persist(RegisterAgentRequest {
            agency: agency.clone(),
            owner_id: "owner-boot".into(),
            agent_id: agent_id.clone(),
            display_name: "CEO".into(),
            role: AgentRoleBody::Ceo,
            supervisor: SupervisorRefBody::Owner {
                owner_id: "owner-boot".into(),
            },
            consult_jev: false,
            promote_runtime_bot: true,
        })
        .await
        .expect("register warm");
        let persisted = warm.persist_bot_catalog().await.expect("persist catalog");
        assert!(persisted.persisted);

        let store = PgOrderReconciliationStore::new(db.as_postgres());
        store
            .mark_pending(&recon_key, "BTC/USDT", OrderSide::Buy)
            .await
            .expect("pending");
        store
            .confirm_exchange_order(&recon_key, "ex-boot-1")
            .await
            .expect("confirm");

        let cold_agents = Arc::new(Mutex::new(AgentRegistry::new()));
        let cold = ApiState::build_api_state_for_http_serve(
            None,
            AppDatabases {
                postgres: Some(db),
                neo4j: None,
            },
            config,
            cold_agents,
        )
        .await;
        assert!(
            cold.product_owner_bootstrap_active(),
            "serve-like boot must load bootstrapped owner from PG"
        );

        let listed = cold
            .list_agents_in_agency(&agency)
            .await
            .expect("list agents");
        assert_eq!(listed.agents.len(), 1);
        assert_eq!(listed.agents[0].agent_id, agent_id);
        assert!(listed.agents[0].promote_runtime_bot);

        let catalog = cold.bot_catalog_snapshot().await.expect("catalog snapshot");
        assert!(!catalog.bots.is_empty());

        match cold.order_reconciliation_state(&recon_key) {
            Some(ReconciliationState::Reconciled { exchange_order_id }) => {
                assert_eq!(exchange_order_id, "ex-boot-1");
            }
            other => panic!("expected reconciled after serve-like boot, got {other:?}"),
        }

        use crate::presentation::http::server::build_router;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = build_router(cold);
        let agents_uri = format!("/api/v1/agents?agency={agency}");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&agents_uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("agents route");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(json["agents"].as_array().map(|a| a.len()), Some(1));

        let catalog_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/bots/catalog")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("bots catalog route");
        assert_eq!(catalog_resp.status(), StatusCode::OK);
        let catalog_bytes = axum::body::to_bytes(catalog_resp.into_body(), usize::MAX)
            .await
            .expect("catalog body");
        let catalog_json: serde_json::Value =
            serde_json::from_slice(&catalog_bytes).expect("catalog json");
        assert!(catalog_json["bots"]
            .as_array()
            .is_some_and(|bots| !bots.is_empty()));

        let config_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/config/active")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("config active route");
        assert_eq!(config_resp.status(), StatusCode::OK);

        let recon_uri = format!("/api/v1/orders/reconciliation/{recon_key}");
        let recon_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&recon_uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("reconciliation route");
        assert_eq!(recon_resp.status(), StatusCode::OK);
        let recon_bytes = axum::body::to_bytes(recon_resp.into_body(), usize::MAX)
            .await
            .expect("recon body");
        let recon_json: serde_json::Value =
            serde_json::from_slice(&recon_bytes).expect("recon json");
        assert_eq!(recon_json["state"], "reconciled");
        assert_eq!(recon_json["exchange_order_id"].as_str(), Some("ex-boot-1"));
    }

    #[tokio::test]
    async fn pg_register_agent_and_persist_cold_start_via_snapshot() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::http_bridge::agents::{
            apply_agent_identity_snapshot, list_agents, load_agent_identity_snapshot,
            AgentRoleBody, RegisterAgentRequest, SupervisorRefBody,
        };
        use std::sync::{Arc, Mutex};

        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let postgres = db.as_postgres();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let agency = format!("agency-api-{suffix}");
        let agent_id = format!("ceo-api-{suffix}");

        let agents = Arc::new(Mutex::new(AgentRegistry::new()));
        let state = ApiState::with_agent_registry(
            None,
            AppDatabases {
                postgres: Some(db.clone()),
                neo4j: None,
            },
            None,
            crate::modules::config_api::Config::default(),
            agents,
            HttpAdminAuth::disabled(),
        );
        state
            .register_agent_and_persist(RegisterAgentRequest {
                agency: agency.clone(),
                owner_id: "owner-pg".into(),
                agent_id: agent_id.clone(),
                display_name: "CEO".into(),
                role: AgentRoleBody::Ceo,
                supervisor: SupervisorRefBody::Owner {
                    owner_id: "owner-pg".into(),
                },
                consult_jev: true,
                promote_runtime_bot: true,
            })
            .await
            .expect("register");

        let (loaded, audit) = load_agent_identity_snapshot(postgres)
            .await
            .expect("load snapshot");
        let mut cold_registry = AgentRegistry::new();
        apply_agent_identity_snapshot(&mut cold_registry, loaded, audit).expect("cold start");
        let listed = list_agents(&cold_registry, &agency).expect("list");
        let agent = listed
            .agents
            .iter()
            .find(|entry| entry.agent_id == agent_id)
            .expect("agent in cold registry");
        assert!(agent.consult_jev);
        assert!(agent.promote_runtime_bot);
    }

    #[tokio::test]
    async fn register_agent_and_persist_in_memory_registry() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::http_bridge::agents::{
            AgentRoleBody, RegisterAgentRequest, SupervisorRefBody,
        };
        use std::sync::{Arc, Mutex};
        let agents = Arc::new(Mutex::new(AgentRegistry::new()));
        let state = ApiState::with_agent_registry(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            agents,
            HttpAdminAuth::disabled(),
        );
        let body = RegisterAgentRequest {
            agency: "integration-agency".into(),
            owner_id: "owner-1".into(),
            agent_id: "ceo".into(),
            display_name: "CEO".into(),
            role: AgentRoleBody::Ceo,
            supervisor: SupervisorRefBody::Owner {
                owner_id: "owner-1".into(),
            },
            consult_jev: false,
            promote_runtime_bot: false,
        };
        let response = state
            .register_agent_and_persist(body)
            .await
            .expect("register");
        assert_eq!(response.agent_id, "ceo");
        let listed = state
            .list_agents_in_agency("integration-agency")
            .await
            .expect("list");
        assert_eq!(listed.agents.len(), 1);
    }

    #[tokio::test]
    async fn register_agent_promote_runtime_bot_visible_on_get() {
        use crate::modules::agents::AgentRegistry;
        use crate::modules::http_bridge::agents::{
            AgentRoleBody, RegisterAgentRequest, SupervisorRefBody,
        };
        use std::sync::{Arc, Mutex};
        let agents = Arc::new(Mutex::new(AgentRegistry::new()));
        let state = ApiState::with_agent_registry(
            None,
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
            agents,
            HttpAdminAuth::disabled(),
        );
        let body = RegisterAgentRequest {
            agency: "cap-agency".into(),
            owner_id: "owner-1".into(),
            agent_id: "promoter".into(),
            display_name: "Promoter".into(),
            role: AgentRoleBody::Ceo,
            supervisor: SupervisorRefBody::Owner {
                owner_id: "owner-1".into(),
            },
            consult_jev: true,
            promote_runtime_bot: true,
        };
        let registered = state
            .register_agent_and_persist(body)
            .await
            .expect("register");
        assert!(registered.promote_runtime_bot);
        assert!(registered.consult_jev);
        let fetched = state
            .get_agent_in_agency("cap-agency", "promoter")
            .await
            .expect("get");
        assert!(fetched.promote_runtime_bot);
        assert!(fetched.consult_jev);
    }

    #[test]
    fn monitor_snapshot_without_handle_is_unavailable() {
        let state = ApiState::default();
        let err = state.monitor_snapshot().unwrap_err();
        assert_eq!(
            err.status_code(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(err.error_code(), Some("monitor_unavailable"));
    }

    #[test]
    fn accept_monitor_command_without_handle_is_unavailable() {
        use crate::modules::http_bridge::monitor::{MonitorCommandName, MonitorCommandRequest};
        let state = ApiState::default();
        let err = state
            .accept_monitor_command(MonitorCommandRequest {
                command: MonitorCommandName::Pause,
            })
            .unwrap_err();
        assert_eq!(
            err.status_code(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(err.error_code(), Some("monitor_unavailable"));
    }
}
