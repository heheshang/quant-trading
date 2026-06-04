# TimescaleDB 时序表分区

> P3-6: 报告依据 PRD §3.18 — K线 / 订单 / 风控事件 走 TimescaleDB hypertable

## 1. 架构概览

我们用 [TimescaleDB](https://www.timescale.com/) (基于 PostgreSQL 16 的时序扩展) 替换
了原本的 vanilla Postgres,主要为了:

- **自动按时间分块 (chunk)** — K线 / 订单 / 风控事件 都是按时间有序写入的,
  手动维护 monthly partition 又丑又慢,hypertables 让 DB 自己做。
- **自动压缩老数据** — 7 天前的 chunk 自动转成列存,通常 10x+ 压缩比。
- **1 年 retention policy** — 后台 worker 自动 drop 过期 chunk,免维护。
- **连续聚合 (continuous aggregate)** — 1m / 5m / 1h K线由后台 worker 增量维护,
  端点 `/api/v1/kline/aggregate` 直接读视图,查询 < 50ms。

### 1.1 改造前后对比

| 项 | 改前 (vanilla Postgres) | 改后 (TimescaleDB) |
|---|---|---|
| 镜像 | `postgres:16-alpine` | `timescale/timescaledb:latest-pg16` |
| K线分块 | 月分区 (klines_2026_05 等) | 7d chunk (自动) |
| 压缩 | 无 | 7d 后自动列存 |
| 保留 | 手动 DROP PARTITION | 1 年 retention policy |
| 1m 聚合 | 实时 GROUP BY (200ms+) | 连续聚合 (< 50ms) |

### 1.2 哪些表走了 hypertable

| 表 | chunk 大小 | 用途 |
|---|---|---|
| `klines` | 7d | Phase 3 旧版 K线 (admin 导入/导出还在用) |
| `klines_phase4` | 7d | **主写入路径** — KlineWriter WS 推来的 K线 |
| `trades` | 1d | 订单成交 (volume 大) |
| `orders` | 1d | 委托 (volume 大) |

## 2. 启动 timescale 容器

```bash
cd /home/ssk/workspace/quant-trading

# 1. 启动 (取代旧的 postgres:16-alpine)
docker compose up -d postgres

# 2. 验证 extension 已加载
docker exec -it quant-postgres psql -U quant -d quant_trading -c "\dx"
# 预期输出: timescaledb | X.Y.Z | public | timescaledb

# 3. 启动 backend (自动跑迁移)
docker compose up -d backend
# backend log 里会看到:
#   "TimescaleDB extension detected — installing hypertables and policies"
#   "TimescaleDB hypertables, policies, and continuous aggregates installed"

# 4. 验证 hypertable 已创建
docker exec -it quant-postgres psql -U quant -d quant_trading -c "
  SELECT hypertable_name, num_chunks
  FROM timescaledb_information.hypertables h
  JOIN LATERAL (
    SELECT count(*) AS num_chunks FROM timescaledb_information.chunks c
    WHERE c.hypertable_name = h.hypertable_name
  ) c ON true;
"
```

> ⚠️ **DATABASE_URL 不变** — TimescaleDB 镜像与 postgres:16 完全兼容,无 break change。
> 已经上线的部署只要改 docker-compose image 即可,不需要迁移数据。

## 3. 怎么查 hypertable

### 3.1 用 SQL 直接查 (推荐运维用)

```sql
-- 所有 hypertable
SELECT hypertable_name, num_dimensions
FROM timescaledb_information.hypertables;

-- 单个 hypertable 的 chunk 列表 + 压缩状态
SELECT chunk_name, range_start, range_end,
       (compression_status = 1) AS is_compressed,
       (compression_status = 3) AS is_partial_compressed
FROM chunk_compression_stats('klines_phase4');

-- 连续聚合 (1m/5m/1h K线) 的最新数据
SELECT bucket, symbol, open, high, low, close, volume
FROM klines_1m
WHERE symbol = 'BTCUSDT' AND bucket > now() - interval '1 hour'
ORDER BY bucket DESC
LIMIT 60;
```

### 3.2 用我们的后端 API (推荐前端用)

```bash
# 1m 聚合 K线 (TimescaleDB continuous aggregate)
curl -H "Authorization: Bearer $JWT" \
  "http://localhost:8080/api/v1/kline/aggregate?symbol=BTCUSDT&interval=1m&from=$(date -d '-1 hour' +%s%3N)&to=$(date +%s%3N)"

# 响应
{
  "code": 0,
  "data": {
    "symbol": "BTCUSDT",
    "interval": "1m",
    "source": "timescaledb_continuous_aggregate",  // 或 "live_fallback" (vanilla PG)
    "bar_count": 60,
    "bars": [
      { "bucket_ms": 1747500000000, "bucket_iso": "2025-05-17T10:00:00Z",
        "symbol": "BTCUSDT", "interval": "1m",
        "open": 97000.0, "high": 97100.0, "low": 96900.0,
        "close": 97050.0, "volume": 100.5 },
      ...
    ]
  }
}
```

### 3.3 用 SQLAlchemy / Python (数据科学/分析)

```python
import pandas as pd
from sqlalchemy import create_engine

engine = create_engine(os.environ["DATABASE_URL"])

# 1h 连续聚合 (TimescaleDB 自动维护,query 极快)
df = pd.read_sql("""
    SELECT bucket, symbol, open, high, low, close, volume
    FROM klines_1h
    WHERE symbol = 'BTCUSDT'
      AND bucket >= now() - interval '7 days'
    ORDER BY bucket ASC
""", engine)

# 直接查 hypertable (拿原始 1m bar)
df_raw = pd.read_sql("""
    SELECT open_time, open, high, low, close, volume
    FROM klines_phase4
    WHERE symbol = 'BTCUSDT'
      AND open_time >= now() - interval '1 day'
    ORDER BY open_time ASC
""", engine)
```

## 4. 怎么加新 hypertable (e.g. 风控事件)

假设我们要给 `risk_events` 加 hypertable:

```sql
-- 1. 启用 extension (幂等)
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- 2. 转 hypertable (假设已有 risk_events 表,带 created_at 列)
SELECT create_hypertable(
    'risk_events',
    'created_at',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- 3. 压缩策略 (7 天后转列存)
ALTER TABLE risk_events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'event_type,severity',
    timescaledb.compress_orderby = 'created_at DESC'
);
SELECT add_compression_policy('risk_events',
    compress_after => INTERVAL '7 days',
    if_not_exists => TRUE);

-- 4. 保留策略 (1 年)
SELECT add_retention_policy('risk_events',
    drop_after => INTERVAL '1 year',
    if_not_exists => TRUE);
```

对应的运行时入口 (在 `src/db/mod.rs::install_timescaledb_extensions`) 也要加上
相应的 DDL,这样 backend 重启也会幂等重跑。

> 📌 **注意**: 写完 SQL 之后,记得在 `install_timescaledb_extensions` 里也加一遍,
> 走 `cargo run` 启 backend 时会自动应用。

## 5. 压缩 / 保留 / 连续聚合后台任务

TimescaleDB 装好后会启动若干后台 worker (`bgw`):

| Worker | 任务 | 调度 |
|---|---|---|
| `Compression BGW` | 跑 `add_compression_policy` 注册的压缩任务 | 默认每 4h 一次 |
| `Retention BGW` | 跑 `add_retention_policy` 注册的清理任务 | 默认每 4h 一次 |
| `Refresh CAGG BGW` | 跑 `add_continuous_aggregate_policy` 注册的刷新 | 见迁移里每个聚合的 `schedule_interval` |

可以用 SQL 看现在活跃的 job:

```sql
SELECT j.job_id, j.application_name, j.schedule_interval,
       j.config, s.last_start, s.last_finish, s.last_success
FROM timescaledb_information.jobs j
LEFT JOIN timescaledb_information.job_stats s ON j.job_id = s.job_id
ORDER BY j.job_id;
```

## 6. 备份策略

TimescaleDB 是基于 PostgreSQL 的,所以标准 PG 备份工具 (`pg_dump`, `pg_basebackup`,
`wal-g`) 都能用。**注意以下几点**:

- **压缩 chunk 必须先解压再 dump** — `pg_dump` 不会自动解压;
  推荐用 `pg_dump -Fc` (custom format) 让 `pg_restore` 自动还原压缩 chunk。
- **连续聚合视图不要 dump** — `klines_1m` 等是视图,被定义为从 hypertable 拉数据;
  restore 后会自动从底层数据重建。把 `klines_1m` 加进 `pg_dump --exclude-table` 黑名单:
  ```bash
  pg_dump -Fc --exclude-table='klines_1m' --exclude-table='klines_5m' --exclude-table='klines_1h' \
    -U quant quant_trading > backup.dump
  ```
- **retention policy 跨 restore** — 备份前 1 年的数据,r 后 restore 后
  老 chunk 会被立刻 drop 掉 (policy 还在跑),所以建议先在目标机器上
  `SELECT alter_job(...)` 关掉 retention,再 dump/restore。

我们用 docker volume 挂载数据目录,正常 `pg_dump` cron 跑就行:

```bash
# docker exec 里跑 pg_dump
docker exec quant-postgres pg_dump -U quant -d quant_trading \
  -Fc --exclude-table=klines_1m --exclude-table=klines_5m --exclude-table=klines_1h \
  > /backups/quant-$(date +%Y%m%d).dump
```

## 7. 常见问题

### 7.1 启动时报 `extension timescaledb is not available`

```text
ERROR: extension "timescaledb" is not available
DETAIL: Could not open extension control file "/usr/share/postgresql/16/extension/timescaledb.control": No such file or directory.
```

→ 镜像错了。检查 `docker-compose.yml` 里 `image: timescale/timescaledb:latest-pg16`,
不能用 `postgres:16-alpine`。

### 7.2 `klines` 已经是 range-partitioned 怎么办

我们的迁移 (`20260603140000_enable_timescaledb.sql`) 在 `create_hypertable` 之前
会先 `DETACH CONCURRENTLY` + `DROP` 掉所有手动 partition。手动操作步骤:

```sql
DO $$
DECLARE part RECORD;
BEGIN
  FOR part IN
    SELECT child.relname AS partition_name
    FROM pg_inherits
    JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
    JOIN pg_class child ON pg_inherits.inhrelid = child.oid
    WHERE parent.relname = 'klines'
  LOOP
    EXECUTE format('ALTER TABLE klines DETACH CONCURRENTLY %I', part.partition_name);
    EXECUTE format('DROP TABLE IF EXISTS %I', part.partition_name);
  END LOOP;
END $$;
```

### 7.3 `/api/v1/kline/aggregate` 返回 `source: "live_fallback"`

→ TimescaleDB 连续聚合视图没建,或者 DATABASE_URL 指向了 vanilla Postgres。
检查 `\dx` 看 timescaledb extension 在不在,以及连续聚合视图:

```sql
\dm  -- 列出所有 materialized view
-- 应该看到: klines_1m, klines_5m, klines_1h
```

如果视图不存在,跑 `backend/migrations/20260603140000_enable_timescaledb.sql`。

### 7.4 迁移后历史数据丢了

我们 detach 手动 monthly partition 的同时 `DROP TABLE` 了它们。
**生产环境部署前先备份**:
```bash
docker exec quant-postgres pg_dump -t klines -t klines_phase4 -U quant quant_trading > klines_pre_tsdb.dump
```

## 8. 参考

- [TimescaleDB 官方文档 — Hypertables & Continuous Aggregates](https://docs.timescale.com/self-hosted/latest/hypertables/)
- [TimescaleDB 压缩指引](https://docs.timescale.com/self-hosted/latest/compression/)
- [TimescaleDB 保留策略](https://docs.timescale.com/self-hosted/latest/data-retention/)
- 内部: `docs/openapi.json` — `/kline/aggregate` 端点的 OpenAPI schema
- 内部: `backend/migrations/20260603140000_enable_timescaledb.sql` — 完整迁移 SQL
- 内部: `backend/migrations/20260603140001_add_timescaledb_policies.sql` — 压缩 + 保留策略
