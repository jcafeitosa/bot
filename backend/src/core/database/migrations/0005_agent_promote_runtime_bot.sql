-- Persist agent capability for HTTP bot runtime promotion authorization after restart.

ALTER TABLE agent_identities
    ADD COLUMN IF NOT EXISTS promote_runtime_bot BOOLEAN NOT NULL DEFAULT FALSE;
