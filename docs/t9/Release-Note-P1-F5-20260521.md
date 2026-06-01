# Release Note v0.9.1 — P1-F5 分表存储

## 功能
- **KlinePartitionManager**: PostgreSQL 分区表管理器
  - `create_partition(year, month)`: 创建月度分区
  - `drop_partition(year, month)`: 删除分区（DETACH CONCURRENTLY）
  - `list_partitions()`: 列出所有现有分区
  - `ensure_future_partitions()`: 自动创建未来2个月分区
  - `cleanup_old_partitions()`: 清理超过保留期的分区
  - `start_background_task()`: 后台任务，24h ticker，UTC每月1日执行

## 技术
- PostgreSQL RANGE partitioning by month
- sea_orm Statement + ConnectionTrait
- Migration: `20260521000000_klines_partition_setup.sql`
- ADR-020: PostgreSQL Range Partitioning Architecture

## 测试
- 267 tests passed（+5 新增）
- test_partition_name, test_next_month_start_ms, test_month_start_ms
- test_partition_name_edge_cases, test_partition_name_various_months

## Breaking Changes
- 需要 PostgreSQL 14+
- 部署前需执行 Migration SQL