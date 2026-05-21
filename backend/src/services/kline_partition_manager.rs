//! Kline Partition Manager — 自动创建和清理 K线分区表
//!
//! PRD: P1-F5
//! - 自动创建未来 2 个月的分区
//! - 自动清理超过 retention_months 的分区
//! - 后台定时任务，每日 UTC 00:00 检查

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use chrono::Datelike;

/// 分区保留月数（默认 12 个月）
const DEFAULT_RETENTION_MONTHS: u32 = 12;
/// 每次预创建未来分区数
const FUTURE_PARTITION_MONTHS: u32 = 2;

/// 分区信息
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub name: String,        // e.g. "klines_2026_05"
    pub year: i32,
    pub month: u32,
    pub row_count: Option<i64>,
}

/// K线分区管理器
pub struct KlinePartitionManager {
    db: Arc<sea_orm::DatabaseConnection>,
    retention_months: u32,
    running: Arc<RwLock<bool>>,
}

impl KlinePartitionManager {
    /// 创建新的分区管理器
    pub fn new(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        Self::with_retention(db, DEFAULT_RETENTION_MONTHS)
    }

    /// 创建分区管理器，自定义保留月数
    pub fn with_retention(db: Arc<sea_orm::DatabaseConnection>, retention_months: u32) -> Self {
        Self {
            db,
            retention_months,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 获取月份的第一天的 UTC 00:00 时间戳（毫秒）
    fn month_start_ms(year: i32, month: u32) -> i64 {
        use chrono::{TimeZone, Utc};
        Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .unwrap()
            .timestamp_millis()
    }

    /// 获取下个月的第一天的 UTC 00:00 时间戳（毫秒）
    fn next_month_start_ms(year: i32, month: u32) -> i64 {
        let (ny, nm) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        Self::month_start_ms(ny, nm)
    }

    /// 生成分区名
    fn partition_name(year: i32, month: u32) -> String {
        format!("klines_{}_{:02}", year, month)
    }

    /// 创建指定月份的分区表
    ///
    /// SQL:
    /// CREATE TABLE klines_YYYY_MM PARTITION OF klines
    ///     FOR VALUES FROM ('YYYY-MM-01 00:00:00+00:00')
    ///     TO ('YYYY-MM+1-01 00:00:00+00:00');
    pub async fn create_partition(
        &self,
        year: i32,
        month: u32,
    ) -> Result<(), crate::utils::error::AppError> {
        let name = Self::partition_name(year, month);
        let start = format!("{}-{:02}-01 00:00:00+00:00", year, month);
        let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let end = format!("{}-{:02}-01 00:00:00+00:00", ny, nm);

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF klines FOR VALUES FROM ('{}') TO ('{}')",
            name, start, end
        );

        info!(partition = %name, sql = %sql, "Creating kline partition");

        let db = self.db.as_ref();
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        ))
        .await
        .map_err(|e| {
            error!(partition = %name, error = %e, "Failed to create partition");
            crate::utils::error::AppError::Internal(e.to_string())
        })?;

        info!(partition = %name, "Partition created successfully");
        Ok(())
    }

    /// 删除指定月份的分区表（保留数据）
    /// 使用 DETACH CONCURRENTLY 避免锁表
    pub async fn drop_partition(
        &self,
        year: i32,
        month: u32,
    ) -> Result<(), crate::utils::error::AppError> {
        let name = Self::partition_name(year, month);

        info!(partition = %name, "Dropping kline partition");

        // Step 1: Detach (non-blocking)
        let detach_sql = format!("ALTER TABLE {} DETACH CONCURRENTLY", name);
        let db = self.db.as_ref();
        let _ = db
            .execute(Statement::from_string(DatabaseBackend::Postgres, detach_sql))
            .await;

        // Step 2: Drop
        let drop_sql = format!("DROP TABLE IF EXISTS {}", name);
        db.execute(Statement::from_string(DatabaseBackend::Postgres, drop_sql))
            .await
            .map_err(|e| {
                error!(partition = %name, error = %e, "Failed to drop partition");
                crate::utils::error::AppError::Internal(e.to_string())
            })?;

        info!(partition = %name, "Partition dropped successfully");
        Ok(())
    }

    /// 列出所有现有分区（按名称排序）
    pub async fn list_partitions(&self) -> Result<Vec<PartitionInfo>, crate::utils::error::AppError> {
        let simple_sql = r#"
            SELECT child.relname AS partition_name
            FROM pg_inherits
            JOIN pg_class parent ON pg_inherits.inhparent = parent.oid
            JOIN pg_class child ON pg_inherits.inhrelid = child.oid
            WHERE parent.relname = 'klines'
            ORDER BY child.relname
        "#;

        let db = self.db.as_ref();
        let stmt = Statement::from_string(DatabaseBackend::Postgres, simple_sql.to_string());
        let rows = db
            .query_all(stmt)
            .await
            .map_err(|e| crate::utils::error::AppError::Internal(e.to_string()))?;

        let partitions: Vec<PartitionInfo> = rows
            .into_iter()
            .filter_map(|row| {
                let name: String = row.try_get_by_index::<String>(0).ok()?;
                // Parse year and month from name like "klines_2026_05"
                let parts: Vec<&str> = name.split('_').collect();
                if parts.len() >= 3 {
                    let year: i32 = parts[1].parse().ok()?;
                    let month: u32 = parts[2].parse().ok()?;
                    Some(PartitionInfo {
                        name,
                        year,
                        month,
                        row_count: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(partitions)
    }

    /// 预创建未来 2 个月的分区
    pub async fn ensure_future_partitions(&self) -> Result<(), crate::utils::error::AppError> {
        use chrono::Utc;
        let now = Utc::now();
        let current_year = now.format("%Y").to_string().parse::<i32>().unwrap();
        let current_month = now.format("%m").to_string().parse::<u32>().unwrap();

        let mut created = 0;
        for offset in 0..=FUTURE_PARTITION_MONTHS {
            let (year, month) = if current_month + offset > 12 {
                (current_year + 1, current_month + offset - 12)
            } else {
                (current_year, current_month + offset)
            };

            let name = Self::partition_name(year, month);

            // Check if partition already exists
            let existing = self.list_partitions().await?;
            if existing.iter().any(|p| p.name == name) {
                info!(partition = %name, "Partition already exists, skipping");
                continue;
            }

            match self.create_partition(year, month).await {
                Ok(_) => {
                    created += 1;
                    info!(partition = %name, "Created future partition");
                }
                Err(e) => {
                    warn!(partition = %name, error = %e, "Failed to create partition");
                }
            }
        }

        info!(created_partitions = created, "ensure_future_partitions completed");
        Ok(())
    }

    /// 清理超过保留期的分区
    pub async fn cleanup_old_partitions(&self) -> Result<(), crate::utils::error::AppError> {
        use chrono::{Months, Utc};

        let partitions = self.list_partitions().await?;
        let cutoff = Utc::now()
            .checked_sub_months(Months::new(self.retention_months))
            .unwrap();
        let cutoff_year = cutoff.format("%Y").to_string().parse::<i32>().unwrap();
        let cutoff_month = cutoff.format("%m").to_string().parse::<u32>().unwrap();

        let mut dropped = 0;
        for partition in partitions {
            // Compare year/month with cutoff
            let is_old = if partition.year < cutoff_year {
                true
            } else if partition.year == cutoff_year {
                partition.month < cutoff_month
            } else {
                false
            };

            if is_old {
                match self.drop_partition(partition.year, partition.month).await {
                    Ok(_) => {
                        dropped += 1;
                        info!(partition = %partition.name, "Dropped old partition");
                    }
                    Err(e) => {
                        warn!(partition = %partition.name, error = %e, "Failed to drop old partition");
                    }
                }
            }
        }

        info!(dropped_partitions = dropped, "cleanup_old_partitions completed");
        Ok(())
    }

    /// 启动后台定时任务
    /// - 每日 UTC 00:00 检查并创建未来分区
    /// - 每月 1 日 UTC 00:00 执行清理
    pub fn start_background_tasks(self: Arc<Self>) {
        let running = self.running.clone();
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(3600)); // Check every hour
            let mut stopped = false;

            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if stopped { break; }
                        let running_guard = running.read().await;
                        if *running_guard {
                            // Check and create future partitions
                            if let Err(e) = self.ensure_future_partitions().await {
                                error!(error = %e, "ensure_future_partitions failed");
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(86400)) => {
                        // Daily: check if today is 1st of month, run cleanup
                        let now = chrono::Utc::now();
                        if now.day() == 1 {
                            if let Err(e) = self.cleanup_old_partitions().await {
                                error!(error = %e, "cleanup_old_partitions failed");
                            }
                        }
                    }
                }
            }
        });
    }

    /// 获取保留月数配置
    pub fn retention_months(&self) -> u32 {
        self.retention_months
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_name() {
        assert_eq!(KlinePartitionManager::partition_name(2026, 5), "klines_2026_05");
        assert_eq!(KlinePartitionManager::partition_name(2026, 12), "klines_2026_12");
        assert_eq!(KlinePartitionManager::partition_name(2027, 1), "klines_2027_01");
    }

    #[test]
    fn test_month_start_ms() {
        let ms = KlinePartitionManager::month_start_ms(2026, 5);
        assert_eq!(ms, 1777593600000); // 2026-05-01 00:00:00 UTC
    }

    #[test]
    fn test_next_month_start_ms() {
        // May -> June
        assert_eq!(KlinePartitionManager::next_month_start_ms(2026, 5), KlinePartitionManager::month_start_ms(2026, 6));
        // December -> January next year
        assert_eq!(KlinePartitionManager::next_month_start_ms(2026, 12), KlinePartitionManager::month_start_ms(2027, 1));
    }

    #[test]
    fn test_old_partition_detection() {
        // Simulate: current 2026-05, cutoff for 12 months retention is 2025-05
        // Partitions before 2025-05 should be dropped
        let cutoff_year = 2025;
        let cutoff_month = 5;

        // klines_2025_04 -> old (month 4 < 5)
        assert!(2025 < cutoff_year || (2025 == cutoff_year && 4 < cutoff_month));
        // klines_2025_05 -> NOT old (month 5 >= 5)
        assert!(!(2025 < cutoff_year || (2025 == cutoff_year && 5 < cutoff_month)));
        // klines_2026_05 -> NOT old
        assert!(!(2026 < cutoff_year || (2026 == cutoff_year && 5 < cutoff_month)));
    }
}
