# PRD: P1-F5 分表存储（K线按月分区）

> 版本: v1.0  
> 状态: Draft  
> Author: PM  
> Date: 2026-05-21

---

## 1. 背景

当前 `klines` 表为单一表，随着时间累积数据量持续增长，查询性能下降。单表过亿行时索引维护成本高、DDL 操作锁表时间长。

PostgreSQL 分区表可以将数据按月拆分，子表独立存储，查询时自动裁剪只扫描相关分区。

## 2. 目标

| 目标 | 指标 |
|------|------|
| 按月分区 | 每月自动创建 `klines_YYYY_MM` 子表 |
| 查询透明 | 应用层代码零改动，分区对查询自动裁剪 |
| 自动清理 | 超过保留月数的分区自动归档/删除 |

## 3. 功能范围

### F1: 分区表架构
- Parent table: `klines`（只定义结构，不存储数据）
- Child tables: `klines_YYYY_MM`（按 `open_time` 月份 RANGE 分区）
- Index on each child: `(symbol, interval, open_time)`

### F2: 分区自动创建
- 定时任务每日检查：提前创建未来 2 个月分区
- 分区命名：`klines_2026_06`、`klines_2026_07`
- 分区范围：每月初 00:00 UTC 到月末 23:59:59 UTC

### F3: 分区自动清理
- 保留期限：默认 12 个月（可配置 `retention_months`）
- 每月 1 日 UTC 00:00 执行清理
- 清理前执行 `ALTER TABLE klines_YYYY_MM DETACH CONCURRENTLY`
- 清理后 `DROP TABLE klines_YYYY_MM`

### F4: 应用层代码零改动
- 现有 `kline_writer.rs` 写入路径不变
- Sea-orm 查询不变
- 写入时 PostgreSQL 自动路由到正确分区

## 4. 验收标准

| AC | 标准 |
|----|------|
| AC1 | 2026-06-01 00:00 UTC 自动创建 `klines_2026_06` 分区 |
| AC2 | `klines_2026_06` 分区范围：2026-06-01 00:00:00 至 2026-06-30 23:59:59 |
| AC3 | 2027-07-01 自动删除超过 12 个月的 `klines_2025_06` |
| AC4 | 现有写入/查询代码零改动，分区对应用透明 |
| AC5 | `\d klines` 显示分区结构（parent + children） |

## 5. 技术方案

### 5.1 数据库层

```sql
-- Parent table（不存储数据）
CREATE TABLE klines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
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
    deleted_at TIMESTAMPTZ
) PARTITION BY RANGE (open_time);

-- Example child partition
CREATE TABLE klines_2026_06 PARTITION OF klines
    FOR VALUES FROM ('2026-06-01 00:00:00+00:00') TO ('2026-07-01 00:00:00+00:00');
```

### 5.2 分区管理器服务

```rust
// src/services/kline_partition_manager.rs
pub struct KlinePartitionManager {
    db: Arc<DatabaseConnection>,
    retention_months: u32,
}

impl KlinePartitionManager {
    /// 创建指定月份的分区表
    pub async fn create_partition(&self, year: i32, month: u32) -> Result<(), AppError>;

    /// 删除指定月份的分区表
    pub async fn drop_partition(&self, year: i32, month: u32) -> Result<(), AppError>;

    /// 列出所有现有分区
    pub async fn list_partitions(&self) -> Result<Vec<String>, AppError>;

    /// 启动后台定时任务
    pub fn start_background_tasks(self: Arc<Self>) { ... }
}
```

## 6. 非功能需求

| 需求 | 指标 |
|------|------|
| 兼容性 | Sea-orm 0.12 + PostgreSQL 14+ |
| 数据完整性 | 分区切换不影响数据写入 |
| 可观测性 | 分区创建/删除记录日志 |
| 配置化 | 保留月数可运行时配置 |

## 7. 依赖

- Phase 1 数据库架构
- Phase 4 kline_writer.rs（已有写入路径）

## 8. 不在此范围

- 回测引擎的 in-memory K线数据
- 其他表的分区（orders/trades 等）
