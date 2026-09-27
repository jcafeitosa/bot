-- Provider API keys (application stores secrets; encryption-at-rest is an optional follow-up ADR).

CREATE TABLE IF NOT EXISTS provider_credentials (
    provider_id TEXT NOT NULL,
    key_name TEXT NOT NULL,
    secret TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider_id, key_name)
);

COMMENT ON TABLE provider_credentials IS 'LLM/provider API keys; load via core::providers::credentials (never log secret column).';
