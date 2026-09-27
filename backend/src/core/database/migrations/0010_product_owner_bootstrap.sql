-- Gate 1: durable product owner bootstrap (explicit env + ACK at runtime; not human IdP).

CREATE TABLE IF NOT EXISTS product_owner_bootstrap (
    singleton_id SMALLINT PRIMARY KEY CHECK (singleton_id = 1),
    owner_id TEXT NOT NULL CHECK (char_length(owner_id) BETWEEN 1 AND 64),
    bootstrapped_at_ms BIGINT NOT NULL CHECK (bootstrapped_at_ms >= 0),
    source TEXT NOT NULL CHECK (source IN ('env_explicit'))
);

CREATE TABLE IF NOT EXISTS product_owner_bootstrap_events (
    event_id BIGSERIAL PRIMARY KEY,
    owner_id TEXT NOT NULL CHECK (char_length(owner_id) BETWEEN 1 AND 64),
    kind TEXT NOT NULL CHECK (kind IN ('bootstrapped')),
    at_ms BIGINT NOT NULL CHECK (at_ms >= 0)
);

CREATE INDEX IF NOT EXISTS product_owner_bootstrap_events_at_idx
    ON product_owner_bootstrap_events (at_ms DESC);
