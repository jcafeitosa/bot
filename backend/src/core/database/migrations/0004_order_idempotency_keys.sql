-- Gate 2: durable dedupe keys for HTTP POST /orders/submit (no order payload stored).
CREATE TABLE IF NOT EXISTS order_idempotency_keys (
    client_order_id TEXT PRIMARY KEY,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE order_idempotency_keys IS 'Completed client_order_id values; replays return accepted without re-executing the order port.';
