-- Create ticker_snapshots table (partitioned by timestamp)
CREATE TABLE IF NOT EXISTS ticker_snapshots (
    id          UUID        NOT NULL DEFAULT gen_random_uuid(),
    symbol      VARCHAR(20) NOT NULL,
    price       DECIMAL(20,8) NOT NULL,
    change      DECIMAL(20,8) NOT NULL DEFAULT 0,
    change_percent DECIMAL(10,4) NOT NULL DEFAULT 0,
    volume      DECIMAL(20,8) NOT NULL DEFAULT 0,
    high        DECIMAL(20,8) NOT NULL DEFAULT 0,
    low         DECIMAL(20,8) NOT NULL DEFAULT 0,
    bid         DECIMAL(20,8) NOT NULL DEFAULT 0,
    ask         DECIMAL(20,8) NOT NULL DEFAULT 0,
    timestamp   TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Partitions for 2026-05, 2026-06, 2026-07, 2026-08, 2026-09, 2026-10
CREATE TABLE ticker_snapshots_202605 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE ticker_snapshots_202606 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE ticker_snapshots_202607 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE ticker_snapshots_202608 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE ticker_snapshots_202609 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE ticker_snapshots_202610 PARTITION OF ticker_snapshots
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');

CREATE INDEX idx_ticker_snapshots_lookup ON ticker_snapshots (symbol, timestamp DESC);
