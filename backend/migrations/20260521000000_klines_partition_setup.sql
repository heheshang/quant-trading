-- Migration: 20260521000000_klines_partition_setup.sql
-- Description: Convert klines table to partitioned table (PARTITION BY RANGE)
-- 
-- IMPORTANT: This migration requires PostgreSQL 14+
-- 
-- Since we cannot ALTER TABLE to become partitioned if data exists,
-- we use a two-step approach:
-- Step 1: Rename existing klines to klines historical
-- Step 2: Create partitioned parent klines
-- Step 3: Create partitions for current month and next 2 months
-- Step 4: Copy historical data to current partition (optional)

-- Step 1: Rename existing table
ALTER TABLE IF EXISTS klines RENAME TO klines_old;

-- Step 2: Create partitioned parent table
CREATE TABLE IF NOT EXISTS klines (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    interval VARCHAR(10) NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    open DECIMAL(20, 8) NOT NULL,
    high DECIMAL(20, 8) NOT NULL,
    low DECIMAL(20, 8) NOT NULL,
    close DECIMAL(20, 8) NOT NULL,
    volume DECIMAL(20, 8) NOT NULL,
    close_time TIMESTAMPTZ,
    quote_volume DECIMAL(20, 8),
    trades BIGINT,
    source VARCHAR(20) NOT NULL DEFAULT 'binance',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    PRIMARY KEY (id, open_time)
) PARTITION BY RANGE (open_time);

-- Add indexes
CREATE INDEX IF NOT EXISTS idx_klines_symbol_interval_time 
    ON klines (symbol, interval, open_time DESC);

CREATE INDEX IF NOT EXISTS idx_klines_user_id 
    ON klines (user_id);

-- Step 3: Create partitions for current month and next 2 months
-- Current: 2026-05
CREATE TABLE IF NOT EXISTS klines_2026_05 PARTITION OF klines
    FOR VALUES FROM ('2026-05-01 00:00:00+00:00') TO ('2026-06-01 00:00:00+00:00');

-- Next month: 2026-06
CREATE TABLE IF NOT EXISTS klines_2026_06 PARTITION OF klines
    FOR VALUES FROM ('2026-06-01 00:00:00+00:00') TO ('2026-07-01 00:00:00+00:00');

-- Month after: 2026-07
CREATE TABLE IF NOT EXISTS klines_2026_07 PARTITION OF klines
    FOR VALUES FROM ('2026-07-01 00:00:00+00:00') TO ('2026-08-01 00:00:00+00:00');

-- Step 4: (Optional - run manually if historical data needs to be preserved)
-- INSERT INTO klines SELECT * FROM klines_old WHERE open_time >= '2026-05-01 00:00:00+00:00';

-- Note: Drop old table after data migration is complete
-- DROP TABLE IF EXISTS klines_old;
