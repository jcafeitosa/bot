-- Gate 2: durable reconciliation ledger for HTTP POST /orders/submit (client_order_id → exchange ack).
CREATE TABLE IF NOT EXISTS order_reconciliation (
    client_order_id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    state TEXT NOT NULL CHECK (state IN ('pending', 'reconciled', 'divergent')),
    exchange_order_id TEXT,
    divergent_reason TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS order_reconciliation_pending_idx
    ON order_reconciliation (state)
    WHERE state = 'pending';

COMMENT ON TABLE order_reconciliation IS 'Post-submit reconciliation state; mirrors in-process OrderReconciliationLedger when PostgreSQL is wired.';
