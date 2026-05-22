# ADR-020: K线分区表架构

> 版本: v1.0  
> 状态: Accepted  
> Date: 2026-05-21

## 背景

`klines` 表数据持续增长，单表查询性能下降。PostgreSQL 分区表按月拆分数据，查询时自动分区裁剪，应用代码零改动。

## 决策

### 核心策略
- **Parent table**: `klines`（仅结构定义，不存储数据）PARTITION BY RANGE (open_time)
- **Child tables**: `klines_YYYY_MM`（每月一个）
- **分区键**: `open_time`（K线时间戳）
- **命名规范**: `klines_YYYY_MM`

### 分区创建
- 后台任务：每日 UTC 00:00 检查，提前创建未来 2 个月分区
- 分区范围：[month_start, month_next)

### 分区清理
- 默认保留 12 个月（可配置 retention_months）
- 每月 1 日 UTC 00:00 执行清理

### 兼容性
- Sea-orm DeriveActiveModel 绑定 `klines`，写入时 PostgreSQL 自动路由
- 现有 `kline_writer.rs` 零改动

## 实现

### 新增文件
- `src/services/kline_partition_manager.rs`

### SQL 迁移
- `migrations/007_klines_partition_setup.sql`

### 关键函数
- `create_partition(year, month)` — 创建月度分区
- `drop_partition(year, month)` — 删除过期分区
- `list_partitions()` — 列出所有分区
- `ensure_future_partitions()` — 预创建未来 2 个月
- `cleanup_old_partitions()` — 清理超期分区

### 不实现
- 按交易对子分区（过度工程）
- 历史数据迁移（现有数据不动）
