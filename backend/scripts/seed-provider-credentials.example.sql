-- Example dev seed for provider_credentials (migration 0007+).
-- Copy to a local-only file, replace placeholders, run against trading_bot.
-- Never commit real API keys or run this file verbatim in CI.

-- Prerequisites:
--   DATABASE_URL=postgresql://USER:PASSWORD@127.0.0.1:5432/trading_bot
--   Migrations applied (bot migrate or HTTP boot with DATABASE_URL).

INSERT INTO provider_credentials (provider_id, key_name, secret)
VALUES
    ('typesafe', 'api_key', '<TYPESAFE_API_KEY>'),
    ('openai', 'api_key', '<OPENAI_API_KEY>'),
    ('nvidia', 'api_key', '<NVIDIA_API_KEY>'),
    ('ngc', 'api_key', '<NGC_API_KEY>')
ON CONFLICT (provider_id, key_name) DO UPDATE
SET secret = EXCLUDED.secret,
    updated_at = now();

-- After INSERT, restart the API process or trigger reload_from_pool on connect
-- (HTTP boot reloads on PostgreSQL connect).
