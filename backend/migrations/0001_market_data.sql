CREATE TABLE IF NOT EXISTS market_datasets (
    dataset_id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    base_timeframe TEXT NOT NULL CHECK (base_timeframe = '1m'),
    start_ms BIGINT NOT NULL CHECK (start_ms >= 0),
    end_ms BIGINT NOT NULL CHECK (end_ms > start_ms),
    candle_count BIGINT NOT NULL CHECK (candle_count > 0),
    gap_count BIGINT NOT NULL CHECK (gap_count >= 0),
    source TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS candles_1m (
    symbol TEXT NOT NULL,
    time_ms BIGINT NOT NULL CHECK (time_ms >= 0),
    open DOUBLE PRECISION NOT NULL CHECK (open > 0),
    high DOUBLE PRECISION NOT NULL CHECK (high > 0),
    low DOUBLE PRECISION NOT NULL CHECK (low > 0),
    close DOUBLE PRECISION NOT NULL CHECK (close > 0),
    volume DOUBLE PRECISION NOT NULL CHECK (volume >= 0),
    dataset_id TEXT NOT NULL REFERENCES market_datasets(dataset_id),
    PRIMARY KEY (symbol, time_ms),
    CHECK (high >= GREATEST(open, close)),
    CHECK (low <= LEAST(open, close)),
    CHECK (high >= low)
);

CREATE INDEX IF NOT EXISTS candles_1m_symbol_time_idx ON candles_1m (symbol, time_ms DESC);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        PERFORM create_hypertable('candles_1m', 'time_ms', 'symbol', 4, if_not_exists => TRUE, migrate_data => TRUE);
    END IF;
END
$$;
