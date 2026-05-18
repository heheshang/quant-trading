-- Migration: Create klines_phase4 partitioned table for Historical Kline Persistence
-- Phase 4 T4: Historical Kline Persistence
-- Date: 2026-05-20
-- Note: Using separate table name to avoid conflict with production klines table

CREATE TABLE IF NOT EXISTS klines_phase4 (
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
    deleted_at      TIMESTAMPTZ,
    -- Note: No composite PRIMARY KEY (id, open_time) since SeaORM migrations
    -- don't handle partitioned table PKs well. Using unique index instead.
    UNIQUE (symbol, interval, open_time) DEFERRABLE INITIALLY DEFERRED
) PARTITION BY RANGE (open_time);

-- Initial partitions for 2026-05, 2026-06, 2026-07, 2026-08, 2026-09, 2026-10
CREATE TABLE IF NOT EXISTS klines_phase4_202605 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS klines_phase4_202606 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE IF NOT EXISTS klines_phase4_202607 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE IF NOT EXISTS klines_phase4_202608 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS klines_phase4_202609 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE IF NOT EXISTS klines_phase4_202610 PARTITION OF klines_phase4
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');

-- Unique index for upsert operations (symbol + interval + open_time) - deferrable
CREATE UNIQUE INDEX IF NOT EXISTS uq_klines_phase4 ON klines_phase4 (symbol, interval, open_time) DEFERRABLE;
-- Lookup index for time-range queries
CREATE INDEX IF NOT EXISTS idx_klines_phase4_lookup ON klines_phase4 (symbol, interval, open_time DESC);
