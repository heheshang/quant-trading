-- 20260603140001_add_timescaledb_policies.sql
--
-- P3-6: 时序表压缩 + 保留策略
-- 报告依据: PRD §3.18
--
-- 设计目标:
--   - 7 天前的 chunk 自动压缩 (segment by symbol,interval / order by open_time DESC)
--   - K线 1 年保留,trades 1 年保留,orders 1 年保留
--
-- 关键决策:
--   1. compress_segmentby = 'symbol,interval' 是为了「同 symbol 同时段」的多行
--      能压缩到同一 segment,显著提升压缩比 (TimescaleDB 文档推荐)。
--   2. compress_orderby = 'open_time DESC' 是为了「最新数据查询」能快速命中
--      columnar 索引 (TimescaleDB 在 orderby 倒序上做了特殊优化)。
--   3. 7 天后才开始压缩,确保热数据(过去一周)保持行存 + 高频写入的写性能。
--   4. add_compression_policy 的 if_not_exists 让幂等重跑成为可能。
--   5. klines / klines_phase4 都需要压缩;trades / orders 也加 (后台 admin 查询
--      几乎只看最近 1 月,老数据存档到 cold chunk 即可)。

-- ============ 1. klines 压缩配置 ============
ALTER TABLE klines SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'symbol,interval',
    timescaledb.compress_orderby = 'open_time DESC'
);

-- 7 天前的 chunk 自动压缩
SELECT add_compression_policy('klines',
    compress_after => INTERVAL '7 days',
    if_not_exists => TRUE);

-- ============ 2. klines_phase4 压缩配置 (主写入路径) ============
ALTER TABLE klines_phase4 SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'symbol,interval',
    timescaledb.compress_orderby = 'open_time DESC'
);

SELECT add_compression_policy('klines_phase4',
    compress_after => INTERVAL '7 days',
    if_not_exists => TRUE);

-- ============ 3. trades 压缩配置 ============
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = 'trades') THEN
        ALTER TABLE trades SET (
            timescaledb.compress,
            timescaledb.compress_segmentby = 'symbol,side',
            timescaledb.compress_orderby = 'created_at DESC'
        );

        PERFORM add_compression_policy('trades',
            compress_after => INTERVAL '7 days',
            if_not_exists => TRUE);

        RAISE NOTICE 'trades compression policy installed';
    END IF;
END $$;

-- ============ 4. orders 压缩配置 ============
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = 'orders') THEN
        ALTER TABLE orders SET (
            timescaledb.compress,
            timescaledb.compress_segmentby = 'symbol,status',
            timescaledb.compress_orderby = 'created_at DESC'
        );

        PERFORM add_compression_policy('orders',
            compress_after => INTERVAL '7 days',
            if_not_exists => TRUE);

        RAISE NOTICE 'orders compression policy installed';
    END IF;
END $$;

-- ============ 5. retention policy (1 年) ============
-- K线保留 1 年 — 满足监管 / 回测需求
SELECT add_retention_policy('klines',
    drop_after => INTERVAL '1 year',
    if_not_exists => TRUE);

SELECT add_retention_policy('klines_phase4',
    drop_after => INTERVAL '1 year',
    if_not_exists => TRUE);

-- trades 保留 1 年 (税务/合规)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = 'trades') THEN
        PERFORM add_retention_policy('trades',
            drop_after => INTERVAL '1 year',
            if_not_exists => TRUE);
    END IF;
END $$;

-- orders 保留 1 年
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = 'orders') THEN
        PERFORM add_retention_policy('orders',
            drop_after => INTERVAL '1 year',
            if_not_exists => TRUE);
    END IF;
END $$;
