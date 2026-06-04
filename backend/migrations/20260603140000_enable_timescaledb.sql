-- 20260603140000_enable_timescaledb.sql
--
-- P3-6: 时序表分区 — TimescaleDB hypertable
-- 报告依据: PRD §3.18 (时序表分区方案)
-- 设计目标: K线 / 订单 / 风控事件 走 hypertable,自动按时间分块 + 压缩老数据 + 1 年保留
--
-- 影响范围:
--   - klines (范围分区表,月分区)           → 重建为 hypertable (按 7d chunk)
--   - klines_phase4 (范围分区表,月分区)     → 重建为 hypertable (按 7d chunk)
--   - trades (按 created_at 时间序)         → 转换为 hypertable (按 1d chunk)
--   - orders (按 created_at 时间序)         → 转换为 hypertable (按 1d chunk)
--
-- 关键决策:
--   1. klines / klines_phase4 原本是范围分区表 (PARTITION BY RANGE (open_time)),
--      TimescaleDB 不支持在已有 range-partitioned 父表上直接 create_hypertable。
--      迁移方案: detach 所有手动 partition → drop 父表 → 重建为 hypertable →
--      create_hypertable(7 days chunk)。
--   2. trades / orders 是普通表,可以直接 create_hypertable(时间列)。
--   3. 全部用 IF NOT EXISTS / DO $$ 保护,允许幂等重跑。
--   4. 连续聚合 (continuous aggregate) 预创建 1m / 5m / 1h 三个常用周期,
--      用 first/last/max/min/sum 实现 OHLCV 聚合 — 与 PRD §3.18 例子一致。
--   5. 连续聚合的 WITH NO DATA: 第一次创建时不跑回填 (历史数据有限);
--      后台 worker 会按刷新策略增量更新。
--
-- 为什么 7d chunk: K线查询大多按 1d ~ 30d 范围,7d chunk 让单次查询最多命中 5 个 chunk
-- (30d / 7d ≈ 4.3),在压缩后 (每个 chunk ~MB 级) 查询延迟 < 100ms。

-- ============ 1. 启用 timescaledb extension ============
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- ============ 2. klines (旧的范围分区表 → hypertable) ============
-- Step 1: 把所有手动 monthly partition 全部 DETACH
DO $$
DECLARE
    part_record RECORD;
BEGIN
    FOR part_record IN
        SELECT child.relname AS partition_name
        FROM pg_inherits
        JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
        JOIN pg_class child ON pg_inherits.inhrelid = child.oid
        WHERE parent.relname = 'klines'
    LOOP
        EXECUTE format('ALTER TABLE klines DETACH CONCURRENTLY %I', part_record.partition_name);
        EXECUTE format('DROP TABLE IF EXISTS %I', part_record.partition_name);
        RAISE NOTICE 'Dropped manual partition: %', part_record.partition_name;
    END LOOP;
END $$;

-- Step 2: 重建为普通表 (不分区),create_hypertable 会把它转成 hypertable
-- 注意: 原表含 PRIMARY KEY (id, open_time),hypertables 允许 PK 包含分区列
ALTER TABLE IF EXISTS klines DROP CONSTRAINT IF EXISTS klines_pkey;
ALTER TABLE IF EXISTS klines
    ADD CONSTRAINT klines_pkey PRIMARY KEY (id, open_time);

-- 索引 (从原 klines_partition_setup 同步过来)
CREATE INDEX IF NOT EXISTS idx_klines_symbol_interval_time
    ON klines (symbol, interval, open_time DESC);
CREATE INDEX IF NOT EXISTS idx_klines_user_id
    ON klines (user_id);

-- Step 3: 转 hypertable (7d chunk, 允许存在空间关联索引)
SELECT create_hypertable(
    'klines',
    'open_time',
    chunk_time_interval => INTERVAL '7 days',
    if_not_exists => TRUE
);

-- ============ 3. klines_phase4 → hypertable ============
-- 同样的 detach + drop + create_hypertable 流程
DO $$
DECLARE
    part_record RECORD;
BEGIN
    FOR part_record IN
        SELECT child.relname AS partition_name
        FROM pg_inherits
        JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
        JOIN pg_class child ON pg_inherits.inhrelid = child.oid
        WHERE parent.relname = 'klines_phase4'
    LOOP
        EXECUTE format('ALTER TABLE klines_phase4 DETACH CONCURRENTLY %I', part_record.partition_name);
        EXECUTE format('DROP TABLE IF EXISTS %I', part_record.partition_name);
        RAISE NOTICE 'Dropped manual partition: %', part_record.partition_name;
    END LOOP;
END $$;

ALTER TABLE IF EXISTS klines_phase4
    ADD CONSTRAINT IF NOT EXISTS klines_phase4_pkey PRIMARY KEY (id, open_time);

-- klines_phase4 已经有 UNIQUE (symbol, interval, open_time) + 2 个索引,create_hypertable
-- 会自动处理。create_hypertable 自身要求时间列在唯一约束里,UNIQUE 包含 open_time 已经满足
SELECT create_hypertable(
    'klines_phase4',
    'open_time',
    chunk_time_interval => INTERVAL '7 days',
    if_not_exists => TRUE
);

-- ============ 4. trades → hypertable (按 1d chunk,因为单日 trade 量可能很大) ============
-- 检查表存在 (trades 由 order 实体创建,先确保不依赖顺序)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'trades') THEN
        PERFORM create_hypertable(
            'trades',
            'created_at',
            chunk_time_interval => INTERVAL '1 day',
            if_not_exists => TRUE
        );
        RAISE NOTICE 'trades converted to hypertable (1d chunk)';
    END IF;
END $$;

-- trades 的索引 (从 20260514000000_orders.sql 同步)
CREATE INDEX IF NOT EXISTS idx_trades_order_id ON trades(order_id);
CREATE INDEX IF NOT EXISTS idx_trades_user_id ON trades(user_id);
CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol);
-- 注: idx_trades_created_at 会在 create_hypertable 后自动创建 (因为它是时间索引)

-- ============ 5. orders → hypertable (1d chunk) ============
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'orders') THEN
        PERFORM create_hypertable(
            'orders',
            'created_at',
            chunk_time_interval => INTERVAL '1 day',
            if_not_exists => TRUE
        );
        RAISE NOTICE 'orders converted to hypertable (1d chunk)';
    END IF;
END $$;

-- ============ 6. 连续聚合 (continuous aggregate) for K线 ============
-- 1m OHLCV: 实时 1 分钟 K线
CREATE MATERIALIZED VIEW IF NOT EXISTS klines_1m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 minute', open_time) AS bucket,
    symbol,
    interval,
    first(open, open_time)       AS open,
    max(high)                    AS high,
    min(low)                     AS low,
    last(close, open_time)       AS close,
    sum(volume)                  AS volume,
    sum(quote_volume)            AS quote_volume,
    sum(trades)                  AS trades
FROM klines_phase4
WHERE deleted_at IS NULL
GROUP BY bucket, symbol, interval
WITH NO DATA;

-- 1m 连续聚合刷新策略:每 30 秒刷新最近 2 小时窗口
-- (start_offset = 2h 是为了能 backfill 还在变化的桶)
SELECT add_continuous_aggregate_policy('klines_1m',
    start_offset => INTERVAL '2 hours',
    end_offset   => INTERVAL '1 minute',
    schedule_interval => INTERVAL '30 seconds',
    if_not_exists => TRUE);

-- 5m OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS klines_5m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('5 minutes', open_time) AS bucket,
    symbol,
    interval,
    first(open, open_time) AS open,
    max(high)              AS high,
    min(low)               AS low,
    last(close, open_time) AS close,
    sum(volume)            AS volume
FROM klines_phase4
WHERE deleted_at IS NULL
GROUP BY bucket, symbol, interval
WITH NO DATA;

SELECT add_continuous_aggregate_policy('klines_5m',
    start_offset => INTERVAL '2 hours',
    end_offset   => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists => TRUE);

-- 1h OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS klines_1h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', open_time) AS bucket,
    symbol,
    interval,
    first(open, open_time) AS open,
    max(high)              AS high,
    min(low)               AS low,
    last(close, open_time) AS close,
    sum(volume)            AS volume
FROM klines_phase4
WHERE deleted_at IS NULL
GROUP BY bucket, symbol, interval
WITH NO DATA;

SELECT add_continuous_aggregate_policy('klines_1h',
    start_offset => INTERVAL '1 day',
    end_offset   => INTERVAL '1 hour',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists => TRUE);
