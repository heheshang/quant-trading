-- Migration: Create klines partitioned table for Historical Kline Persistence
-- Phase 4 T4: Historical Kline Persistence
-- Date: 2026-05-17

CREATE TABLE IF NOT EXISTS klines (
    id              UUID        NOT NULL DEFAULT gen_random_uuid(),
    symbol          VARCHAR(20) NOT NULL,
    interval        VARCHAR(10) NOT NULL,
    open_time       TIMESTAMPTZ NOT NULL,
    close_time      TIMESTAMPTZ NOT NULL,
    open            DECIMAL(20,8) NOT NULL,
    high            DECIMAL(20,8) NOT NULL,
    low             DECIMAL(20,8) NOT NULL,
    close           DECIMAL(20,8) NOT NULL,
    volume          DECIMAL(20,8) NOT NULL,
    quote_volume    DECIMAL(20,8) NOT NULL,
    trades          INT         NOT NULL DEFAULT 0,
    source          VARCHAR(20) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, open_time)
) PARTITION BY RANGE (open_time);

-- Initial partitions for 2026-05, 2026-06, 2026-07
CREATE TABLE klines_202605 PARTITION OF klines
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE klines_202606 PARTITION OF klines
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE klines_202607 PARTITION OF klines
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');

-- Unique constraint for upsert operations (symbol + interval + open_time)
CREATE UNIQUE INDEX uq_klines ON klines (symbol, interval, open_time);
-- Lookup index for time-range queries
CREATE INDEX idx_klines_lookup ON klines (symbol, interval, open_time DESC);