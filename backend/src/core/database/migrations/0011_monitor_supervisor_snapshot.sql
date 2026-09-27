-- C17 fatia 1: advisory supervisor metadata (singleton; not orders/agents SoT).

CREATE TABLE IF NOT EXISTS monitor_supervisor_snapshot (
    singleton_id SMALLINT PRIMARY KEY CHECK (singleton_id = 1),
    last_tick_ms BIGINT NOT NULL CHECK (last_tick_ms >= 0),
    promoted_bot_id TEXT CHECK (
        promoted_bot_id IS NULL OR char_length(promoted_bot_id) BETWEEN 1 AND 128
    ),
    updated_at_ms BIGINT NOT NULL CHECK (updated_at_ms >= 0)
);
