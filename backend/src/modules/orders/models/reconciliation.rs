use super::error::OrdersError;
use super::request::OrderSide;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingReconciliationItem {
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationState {
    /// Submit aceito localmente; confirmação da exchange ainda não aplicada.
    Pending,
    /// `exchange_order_id` observado e associado ao `client_order_id`.
    Reconciled { exchange_order_id: String },
    /// Estado local e remoto não coincidem (detalhe operacional).
    Divergent { reason: String },
}

/// Ledger durável ou in-process para pós-submit Gate 2 (não substitui idempotência HTTP).
pub trait OrderReconciliationLedger: Send + Sync {
    fn mark_pending(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
    ) -> Result<(), OrdersError>;

    fn confirm_exchange_order(
        &self,
        client_order_id: &str,
        exchange_order_id: &str,
    ) -> Result<ReconciliationState, OrdersError>;

    fn mark_divergent(&self, client_order_id: &str, reason: &str) -> Result<(), OrdersError>;

    fn state(&self, client_order_id: &str) -> Option<ReconciliationState>;

    fn pending_count(&self) -> usize;

    fn list_pending(&self) -> Vec<PendingReconciliationItem>;
}
