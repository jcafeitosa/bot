-- F2.1: durable outbox for Neo4j graph projection (PG SoT; MERGE via drain stub).
CREATE TABLE IF NOT EXISTS graph_projection_outbox (
    id BIGSERIAL PRIMARY KEY,
    graph_domain TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'done', 'retry')),
    attempt_count INT NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    UNIQUE (graph_domain, idempotency_key)
);

CREATE INDEX IF NOT EXISTS graph_projection_outbox_status_id_idx
    ON graph_projection_outbox (status, id)
    WHERE status IN ('pending', 'retry');

COMMENT ON TABLE graph_projection_outbox IS
    'Durable queue for idempotent Neo4j MERGE; enqueue post-PG-commit until same-tx seam exists.';
