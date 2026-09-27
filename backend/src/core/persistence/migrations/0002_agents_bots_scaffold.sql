-- Gate 1 scaffold: administrative agent identities and versioned bot catalog rows.
-- No Rust repository wiring in this migration; runtime remains in-memory until Gate 1.

CREATE TABLE IF NOT EXISTS agent_identities (
    agent_id TEXT PRIMARY KEY CHECK (char_length(agent_id) BETWEEN 1 AND 64),
    agency_id TEXT NOT NULL CHECK (char_length(agency_id) BETWEEN 1 AND 64),
    owner_id TEXT NOT NULL CHECK (char_length(owner_id) BETWEEN 1 AND 64),
    display_name TEXT NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 128),
    role TEXT NOT NULL CHECK (role IN ('ceo', 'level_b', 'level_a', 'specialist', 'worker')),
    supervisor_kind TEXT NOT NULL CHECK (supervisor_kind IN ('owner', 'agent')),
    supervisor_owner_id TEXT CHECK (
        supervisor_owner_id IS NULL OR char_length(supervisor_owner_id) BETWEEN 1 AND 64
    ),
    supervisor_agent_id TEXT CHECK (
        supervisor_agent_id IS NULL OR char_length(supervisor_agent_id) BETWEEN 1 AND 64
    ),
    lifecycle_state TEXT NOT NULL CHECK (lifecycle_state IN ('active', 'paused', 'retired')),
    consult_jev BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_ms BIGINT NOT NULL CHECK (created_at_ms >= 0),
    updated_at_ms BIGINT NOT NULL CHECK (updated_at_ms >= created_at_ms),
    CHECK (
        (supervisor_kind = 'owner' AND supervisor_owner_id IS NOT NULL AND supervisor_agent_id IS NULL)
        OR (supervisor_kind = 'agent' AND supervisor_agent_id IS NOT NULL AND supervisor_owner_id IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS agent_identities_agency_idx ON agent_identities (agency_id);

CREATE TABLE IF NOT EXISTS agent_identity_events (
    event_id BIGSERIAL PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agent_identities (agent_id) ON DELETE CASCADE,
    agency_id TEXT NOT NULL CHECK (char_length(agency_id) BETWEEN 1 AND 64),
    kind TEXT NOT NULL CHECK (kind IN ('registered', 'paused', 'resumed', 'retired')),
    at_ms BIGINT NOT NULL CHECK (at_ms >= 0)
);

CREATE INDEX IF NOT EXISTS agent_identity_events_agent_at_idx
    ON agent_identity_events (agent_id, at_ms DESC);

CREATE TABLE IF NOT EXISTS bot_catalog_entries (
    bot_id TEXT PRIMARY KEY CHECK (char_length(bot_id) BETWEEN 1 AND 160),
    strategy_id TEXT NOT NULL CHECK (char_length(strategy_id) BETWEEN 1 AND 80),
    strategy_version INTEGER NOT NULL CHECK (strategy_version >= 0),
    timeframe TEXT NOT NULL CHECK (char_length(timeframe) BETWEEN 1 AND 8),
    symbol TEXT NOT NULL CHECK (char_length(symbol) BETWEEN 3 AND 32),
    operation_mode TEXT NOT NULL CHECK (
        operation_mode IN ('hft', 'scalper', 'day_trader', 'swing_trader')
    ),
    persisted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS bot_catalog_entries_strategy_idx
    ON bot_catalog_entries (strategy_id, strategy_version);
