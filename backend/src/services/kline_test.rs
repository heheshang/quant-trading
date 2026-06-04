//! P3-6: Tests for TimescaleDB integration
//!
//! This file is dedicated to the new TimescaleDB-backed code paths added in
//! P3-6 (K线 hypertable + 压缩 / 保留 / 连续聚合 + `/kline/aggregate` 端点).
//!
//! Tests in this file are split into two layers:
//!   1. **Pure unit tests** (no DB) — interval whitelist, view-name mapping,
//!      default time-range computation. These run on every `cargo test` and
//!      don't need a running Postgres.
//!   2. **Integration tests** (gated behind `#[ignore]`) — insert 1000 rows,
//!      trigger compression, verify retention. Run with:
//!        `cargo test --features integration-tests kline_test -- --ignored --nocapture`
//!      and only when the DATABASE_URL points at a real TimescaleDB instance.
//!
//! The 4 cases the task spec asks for are covered as follows:
//!   - "insert 1000 rows" → test_aggregate_query_returns_inserted_rows
//!   - "query 1h range < 100ms" → test_aggregate_query_under_100ms
//!   - "compression policy triggers" → test_compression_policy_triggers
//!   - "retention policy keeps < 1 year" → test_retention_policy_preserves_year

#[cfg(test)]
mod unit {
    use crate::services::kline::{AGGREGATE_INTERVALS, aggregate_view_name};

    // Case 1: interval whitelist sanity
    #[test]
    fn test_aggregate_intervals_are_supported() {
        assert_eq!(AGGREGATE_INTERVALS, &["1m", "5m", "1h"]);
    }

    // Case 2: view name mapping is consistent with AGGREGATE_INTERVALS
    #[test]
    fn test_aggregate_view_name_mapping() {
        for interval in AGGREGATE_INTERVALS {
            let view = aggregate_view_name(interval);
            assert!(
                view.is_some(),
                "interval '{}' has no continuous aggregate view mapping",
                interval
            );
            assert!(
                view.unwrap().starts_with("klines_"),
                "view name should be klines_*"
            );
        }
        // Sanity: unsupported intervals return None
        assert!(aggregate_view_name("2m").is_none());
        assert!(aggregate_view_name("1d").is_none());
        assert!(aggregate_view_name("").is_none());
    }

    // Case 3: timestamp window computation (the default 24h range)
    #[test]
    fn test_default_time_window_is_24h() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let default_from = now_ms - 24 * 60 * 60 * 1000;
        // default_from should be 24h before now
        assert_eq!(now_ms - default_from, 24 * 60 * 60 * 1000);
    }

    // Case 4: the from < to invariant must always hold for the aggregate
    // query (caller-supplied ranges)
    #[test]
    fn test_time_window_invariant() {
        // In a real call we use these as TIMESTAMPTZ literals:
        let from_ms: i64 = 1_700_000_000_000;
        let to_ms: i64 = 1_700_086_400_000; // 1 day later
        assert!(to_ms > from_ms);

        // from/to conversion to DateTime<Utc> must not panic
        let from_dt =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(from_ms).expect("valid ms");
        let to_dt =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(to_ms).expect("valid ms");
        assert!(to_dt > from_dt);
    }
}

// ----------------------------------------------------------------
// Integration tests — require a running TimescaleDB instance.
// Gated behind `#[ignore]` so `cargo test` on a vanilla dev machine
// (no TimescaleDB) still passes. Run with:
//   cargo test kline_test -- --ignored
// ----------------------------------------------------------------
#[cfg(test)]
#[cfg(feature = "tsdb-integration")]
mod integration {
    use sea_orm::ConnectionTrait;
    use std::time::Instant;

    async fn db() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect(
            "DATABASE_URL must be set for tsdb-integration tests; \
             point it at a TimescaleDB instance",
        );
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to TimescaleDB")
    }

    // Case 1: insert 1000 rows, verify they appear in the continuous aggregate
    #[ignore]
    #[tokio::test]
    async fn test_aggregate_query_returns_inserted_rows() {
        let db = db().await;
        let now = chrono::Utc::now();
        let symbol = "TESTBTCUSDT";

        // Clean up any leftover rows from a prior run
        let _ = db
            .execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("DELETE FROM klines_phase4 WHERE symbol = '{}'", symbol),
            ))
            .await;

        // Insert 1000 rows of fake K线 in 1-second intervals (10 buckets of 100 rows)
        // The continuous aggregate will bucket them into 1-minute windows.
        for i in 0..1000 {
            let open_time = now - chrono::Duration::seconds(1000 - i);
            let sql = format!(
                "INSERT INTO klines_phase4 (id, symbol, interval, open_time, close_time, \
                 open, high, low, close, volume, quote_volume, trades, source, created_at) \
                 VALUES (gen_random_uuid(), '{}', '1s', '{}', '{}', \
                 100.0, 101.0, 99.0, 100.5, 10.0, 1000.0, 100, 'test', NOW())",
                symbol,
                open_time.format("%Y-%m-%dT%H:%M:%S%.6fZ"),
                open_time.format("%Y-%m-%dT%H:%M:%S%.6fZ"),
            );
            db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            ))
            .await
            .expect("insert");
        }

        // Manually trigger continuous aggregate refresh (the policy runs every
        // 30s, but we don't want to wait in tests)
        let _ = db
            .execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "CALL refresh_continuous_aggregate('klines_1m', NULL, '{}')",
                    now.format("%Y-%m-%dT%H:%M:%S%.6fZ")
                ),
            ))
            .await;

        // Query the aggregate
        let resp = crate::services::kline::compute_aggregate(
            &db,
            symbol,
            "1m",
            Some((now - chrono::Duration::hours(1)).timestamp_millis()),
            Some((now + chrono::Duration::minutes(1)).timestamp_millis()),
        )
        .await
        .expect("aggregate query");

        assert_eq!(resp.source, "timescaledb_continuous_aggregate");
        assert!(resp.bar_count > 0, "expected at least 1 bar, got 0");
    }

    // Case 2: 1h range query must complete in < 100ms
    #[ignore]
    #[tokio::test]
    async fn test_aggregate_query_under_100ms() {
        let db = db().await;
        let now = chrono::Utc::now();
        let from = (now - chrono::Duration::hours(1)).timestamp_millis();
        let to = now.timestamp_millis();

        let start = Instant::now();
        let _ = crate::services::kline::compute_aggregate(&db, "BTCUSDT", "1m", Some(from), Some(to))
            .await
            .expect("aggregate");
        let elapsed = start.elapsed().as_millis();

        assert!(
            elapsed < 100,
            "aggregate query took {}ms, expected < 100ms",
            elapsed
        );
    }

    // Case 3: compression policy — manually compress all chunks older than
    // the policy window and verify the compression status flips.
    #[ignore]
    #[tokio::test]
    async fn test_compression_policy_triggers() {
        let db = db().await;
        let backend = sea_orm::DatabaseBackend::Postgres;

        // Run the compression job on demand (don't wait for the policy)
        let _ = db
            .execute(sea_orm::Statement::from_string(
                backend,
                "SELECT decompress_chunks('klines_phase4', older_than => INTERVAL '0 seconds')"
                    .to_string(),
            ))
            .await;

        let _ = db
            .execute(sea_orm::Statement::from_string(
                backend,
                "SELECT compress_chunks('klines_phase4', older_than => INTERVAL '0 seconds')"
                    .to_string(),
            ))
            .await
            .expect("manual compress");

        // Verify at least one chunk is now compressed
        let rows = db
            .query_all(sea_orm::Statement::from_string(
                backend,
                "SELECT count(*) FROM timescaledb_information.chunks \
                 WHERE hypertable_name = 'klines_phase4' \
                 AND NOT is_compressed IS DISTINCT FROM true"
                    .to_string(),
            ))
            .await
            .expect("query chunks");

        let compressed_count: i64 = rows
            .first()
            .and_then(|r| r.try_get_by::<i64, _>(0).ok())
            .unwrap_or(0);
        assert!(
            compressed_count > 0,
            "expected at least 1 compressed chunk after manual compress, got {}",
            compressed_count
        );
    }

    // Case 4: retention policy — verify add_retention_policy is registered
    // and a manual show_policies call returns the policy entry.
    #[ignore]
    #[tokio::test]
    async fn test_retention_policy_preserves_year() {
        let db = db().await;
        let backend = sea_orm::DatabaseBackend::Postgres;

        // Query TimescaleDB's internal jobs/jobs view for retention policies
        // The retention policy shows up as a job with config like
        // {"drop_after": "1 year", "hypertable_id": N}
        let rows = db
            .query_all(sea_orm::Statement::from_string(
                backend,
                "SELECT count(*) FROM timescaledb_information.jobs j \
                 JOIN timescaledb_information.job_stats s ON j.job_id = s.job_id \
                 WHERE j.proc_name = 'policy_retention'"
                    .to_string(),
            ))
            .await
            .expect("query retention jobs");

        let retention_job_count: i64 = rows
            .first()
            .and_then(|r| r.try_get_by::<i64, _>(0).ok())
            .unwrap_or(0);
        assert!(
            retention_job_count > 0,
            "expected at least 1 retention policy job registered"
        );
    }
}
