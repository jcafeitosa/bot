-- Manual retention for Gate 2 orders tables (run against database trading_bot).
-- Policy: docs/reference/cli-and-config.md#pg-orders-retention-gate-2
-- Review row counts before COMMIT; take backup in production.
--
-- Dry-run (run these alone first; no writes):
-- SELECT COUNT(*) AS idempotency_rows_to_delete
-- FROM order_idempotency_keys
-- WHERE recorded_at < NOW() - INTERVAL '90 days';
-- SELECT COUNT(*) AS reconciliation_terminal_rows_to_delete
-- FROM order_reconciliation
-- WHERE state IN ('reconciled', 'divergent')
--   AND updated_at < NOW() - INTERVAL '180 days';
-- SELECT client_order_id, symbol, side, updated_at AS pending_stale
-- FROM order_reconciliation
-- WHERE state = 'pending'
--   AND updated_at < NOW() - INTERVAL '7 days'
-- ORDER BY updated_at ASC;

BEGIN;

DELETE FROM order_idempotency_keys
WHERE recorded_at < NOW() - INTERVAL '90 days';

DELETE FROM order_reconciliation
WHERE state IN ('reconciled', 'divergent')
  AND updated_at < NOW() - INTERVAL '180 days';

COMMIT;
