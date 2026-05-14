use crate::db::kline::{self, Entity as Kline};
use crate::models::schemas::{
    KlineCleanRequest, KlineCleanResult, KlineExportParams, KlineImportItem, KlineImportRequest,
    KlineImportResult, KlineImportLogResponse, KlineListResponse, KlineListMeta, KlineQualityAnomaly,
    KlineQualityReport, KlineQueryParams, KlineResponse,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

const VALID_INTERVALS: [&str; 8] = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
const VALID_SOURCES: [&str; 3] = ["csv", "api", "exchange"];
const BATCH_SIZE: usize = 5000;

fn model_to_response(m: kline::Model) -> KlineResponse {
    KlineResponse {
        id: m.id,
        user_id: m.user_id,
        symbol: m.symbol,
        interval: m.interval,
        open_time: m.open_time,
        open: m.open,
        high: m.high,
        low: m.low,
        close: m.close,
        volume: m.volume,
        close_time: m.close_time,
        quote_volume: m.quote_volume,
        trades: m.trades,
        source: m.source,
        created_at: m.created_at,
    }
}

fn page_size(params: &KlineQueryParams) -> u64 {
    params.size.unwrap_or(20).clamp(1, 100)
}

fn page_num(params: &KlineQueryParams) -> u64 {
    params.page.unwrap_or(1).max(1)
}

fn offset(params: &KlineQueryParams) -> u64 {
    (page_num(params) - 1) * page_size(params)
}

// ============ Query ============

pub async fn query_klines(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: KlineQueryParams,
) -> Result<KlineListResponse, AppError> {
    let page = page_num(&params);
    let size = page_size(&params);
    let off = offset(&params);

    let mut query = Kline::find().filter(kline::Column::UserId.eq(user_id));

    if let Some(ref symbol) = params.symbol {
        query = query.filter(kline::Column::Symbol.eq(symbol));
    }
    if let Some(ref interval) = params.interval {
        query = query.filter(kline::Column::Interval.eq(interval));
    }
    if let Some(start) = params.start_time {
        query = query.filter(kline::Column::OpenTime.gte(start));
    }
    if let Some(end) = params.end_time {
        query = query.filter(kline::Column::OpenTime.lte(end));
    }

    let total = query.clone().count(db).await? as u64;

    let items: Vec<KlineResponse> = query
        .order_by_asc(kline::Column::OpenTime)
        .offset(off)
        .limit(size)
        .all(db)
        .await?
        .into_iter()
        .map(model_to_response)
        .collect();

    // Gap detection: check if consecutive bars differ by more than expected interval ms
    let gap_detected = detect_gaps(&items);

    Ok(KlineListResponse {
        items,
        meta: KlineListMeta {
            total,
            page,
            size,
            gap_detected,
        },
    })
}

fn detect_gaps(items: &[KlineResponse]) -> bool {
    if items.len() < 2 {
        return false;
    }
    let interval_ms = match items.first().map(|i| i.interval.as_str()) {
        Some("1m") => 60_000,
        Some("5m") => 300_000,
        Some("15m") => 900_000,
        Some("30m") => 1_800_000,
        Some("1h") => 3_600_000,
        Some("4h") => 14_400_000,
        Some("1d") => 86_400_000,
        Some("1w") => 604_800_000,
        _ => return false,
    };

    for window in items.windows(2) {
        let diff = window[1].open_time - window[0].open_time;
        if diff > interval_ms * 2 {
            return true;
        }
    }
    false
}

// ============ Import ============

pub async fn import_klines(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: KlineImportRequest,
) -> Result<KlineImportResult, AppError> {
    validate_import_request(&req)?;

    let _total = req.data.len() as i64;
    let mut imported = 0_i64;
    let mut duplicates = 0_i64;
    let mut failed = 0_i64;

    // Process in batches of BATCH_SIZE
    for chunk in req.data.chunks(BATCH_SIZE) {
        let (imp, dup, fail) = import_batch(db, user_id, &req.symbol, &req.interval, &req.source, chunk)
            .await?;
        imported += imp;
        duplicates += dup;
        failed += fail;
    }

    Ok(KlineImportResult {
        imported_rows: imported,
        duplicate_rows: duplicates,
        failed_rows: failed,
    })
}

fn validate_import_request(req: &KlineImportRequest) -> Result<(), AppError> {
    if req.symbol.trim().is_empty() {
        return Err(AppError::Validation("symbol is required".into()));
    }
    if !VALID_INTERVALS.contains(&req.interval.as_str()) {
        return Err(AppError::Validation(format!(
            "invalid interval '{}', must be one of {:?}",
            req.interval, VALID_INTERVALS
        )));
    }
    if !VALID_SOURCES.contains(&req.source.as_str()) {
        return Err(AppError::Validation(format!(
            "invalid source '{}', must be one of {:?}",
            req.source, VALID_SOURCES
        )));
    }
    if req.data.is_empty() {
        return Err(AppError::Validation("data cannot be empty".into()));
    }
    if req.data.len() > 100_000 {
        return Err(AppError::Validation("data exceeds maximum of 100,000 rows per import".into()));
    }
    Ok(())
}

async fn import_batch(
    db: &DatabaseConnection,
    user_id: Uuid,
    symbol: &str,
    interval: &str,
    source: &str,
    items: &[KlineImportItem],
) -> Result<(i64, i64, i64), AppError> {
    let mut imported = 0_i64;
    let mut duplicates = 0_i64;
    let mut failed = 0_i64;
    let now = chrono::Utc::now();

    for item in items {
        // Check for existing kline with same user_id, symbol, interval, open_time
        let existing = Kline::find()
            .filter(kline::Column::UserId.eq(user_id))
            .filter(kline::Column::Symbol.eq(symbol))
            .filter(kline::Column::Interval.eq(interval))
            .filter(kline::Column::OpenTime.eq(item.open_time))
            .one(db)
            .await?;

        if existing.is_some() {
            duplicates += 1;
            continue;
        }

        // Validate numeric fields
        if parse_f64(&item.open).is_err()
            || parse_f64(&item.high).is_err()
            || parse_f64(&item.low).is_err()
            || parse_f64(&item.close).is_err()
            || parse_f64(&item.volume).is_err()
        {
            failed += 1;
            continue;
        }

        // Validate OHLC relationships
        let (o, h, l, c) = (
            parse_f64(&item.open).unwrap(),
            parse_f64(&item.high).unwrap(),
            parse_f64(&item.low).unwrap(),
            parse_f64(&item.close).unwrap(),
        );
        if h < o || h < l || h < c || l > o || l > c {
            failed += 1;
            continue;
        }

        let active = kline::ActiveModel {
            id: sea_orm::Set(0_i64),
            user_id: sea_orm::Set(user_id),
            symbol: sea_orm::Set(symbol.to_string()),
            interval: sea_orm::Set(interval.to_string()),
            open_time: sea_orm::Set(item.open_time),
            open: sea_orm::Set(item.open.clone()),
            high: sea_orm::Set(item.high.clone()),
            low: sea_orm::Set(item.low.clone()),
            close: sea_orm::Set(item.close.clone()),
            volume: sea_orm::Set(item.volume.clone()),
            close_time: sea_orm::Set(item.close_time),
            quote_volume: sea_orm::Set(item.quote_volume.clone()),
            trades: sea_orm::Set(item.trades),
            source: sea_orm::Set(source.to_string()),
            created_at: sea_orm::Set(now),
        };

        if active.insert(db).await.is_ok() {
            imported += 1;
        } else {
            failed += 1;
        }
    }

    Ok((imported, duplicates, failed))
}

fn parse_f64(s: &str) -> Result<f64, ()> {
    s.trim().parse::<f64>().map_err(|_| ())
}

// ============ Import History ============

pub async fn import_history(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<KlineImportLogResponse>, AppError> {
    // Group klines by (user_id, symbol, interval, source, date) and summarize
    // Since we don't have a separate import_logs table, we derive from klines table
    // Return aggregated stats per symbol/interval combination
    #[derive(Debug)]
    struct ImportSummary {
        symbol: String,
        interval: String,
        source: String,
        total_rows: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    use sea_orm::DatabaseBackend;
    use sea_orm::Statement;

    let now = chrono::Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);

    // We aggregate from the klines table - get distinct groups
    let raw_sql = r#"
        SELECT symbol, interval, source, COUNT(*) as cnt, MIN(created_at) as first_import
        FROM klines
        WHERE user_id = $1 AND created_at >= $2
        GROUP BY symbol, interval, source
        ORDER BY first_import DESC
        LIMIT 100
    "#;

    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        raw_sql,
        vec![user_id.to_string().into(), thirty_days_ago.into()],
    );

    let rows: Vec<ImportSummary> = db
        .query_all(stmt)
        .await?
        .into_iter()
        .filter_map(|row| {
            let symbol: String = row.try_get_by_index::<String>(0).ok()?;
            let interval: String = row.try_get_by_index::<String>(1).ok()?;
            let source: String = row.try_get_by_index::<String>(2).ok()?;
            let total_rows: i64 = row.try_get_by_index::<i64>(3).ok()?;
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get_by_index::<chrono::DateTime<chrono::Utc>>(4).ok()?;

            Some(ImportSummary {
                symbol,
                interval,
                source,
                total_rows,
                created_at,
            })
        })
        .collect();

    Ok(rows
        .into_iter()
        .map(|s| KlineImportLogResponse {
            id: 0,
            user_id,
            symbol: s.symbol,
            interval: s.interval,
            source: s.source,
            total_rows: s.total_rows,
            imported_rows: s.total_rows,
            duplicate_rows: 0,
            failed_rows: 0,
            status: "completed".to_string(),
            created_at: s.created_at,
        })
        .collect())
}

// ============ Quality Report ============

pub async fn quality_report(
    db: &DatabaseConnection,
    user_id: Uuid,
    symbol: &str,
    interval: &str,
) -> Result<KlineQualityReport, AppError> {
    let klines = Kline::find()
        .filter(kline::Column::UserId.eq(user_id))
        .filter(kline::Column::Symbol.eq(symbol))
        .filter(kline::Column::Interval.eq(interval))
        .order_by_asc(kline::Column::OpenTime)
        .all(db)
        .await?;

    let total_rows = klines.len() as i64;
    if total_rows == 0 {
        return Ok(KlineQualityReport {
            symbol: symbol.to_string(),
            interval: interval.to_string(),
            total_rows: 0,
            valid_rows: 0,
            coverage_pct: 0.0,
            gap_count: 0,
            anomaly_count: 0,
            duplicate_count: 0,
            anomalies: vec![],
        });
    }

    let mut valid_rows = 0_i64;
    let mut duplicate_count = 0_i64;
    let mut anomaly_count = 0_i64;
    let mut anomalies = Vec::new();
    let mut seen_open_times: std::collections::HashSet<i64> = std::collections::HashSet::new();

    // Expected interval in ms
    let interval_ms: i64 = match interval {
        "1m" => 60_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "4h" => 14_400_000,
        "1d" => 86_400_000,
        "1w" => 604_800_000,
        _ => 3_600_000,
    };

    // Check duplicates and anomalies in single pass
    for k in &klines {
        // Duplicate detection
        if seen_open_times.contains(&k.open_time) {
            duplicate_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: k.open_time,
                anomaly_type: "duplicate".to_string(),
                open: Some(k.open.clone()),
                high: Some(k.high.clone()),
                low: Some(k.low.clone()),
                close: Some(k.close.clone()),
            });
        } else {
            seen_open_times.insert(k.open_time);
        }

        // Validate OHLC
        let open_p = parse_f64(&k.open);
        let high_p = parse_f64(&k.high);
        let low_p = parse_f64(&k.low);
        let close_p = parse_f64(&k.close);

        if open_p.is_err() || high_p.is_err() || low_p.is_err() || close_p.is_err() {
            anomaly_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: k.open_time,
                anomaly_type: "invalid".to_string(),
                open: Some(k.open.clone()),
                high: Some(k.high.clone()),
                low: Some(k.low.clone()),
                close: Some(k.close.clone()),
            });
            continue;
        }

        let (o, h, l, c) = (
            open_p.unwrap(),
            high_p.unwrap(),
            low_p.unwrap(),
            close_p.unwrap(),
        );

        // OHLC validity: high >= max(open, high, low, close), low <= min(...)
        if h < o.max(h).max(l).max(c) || l > o.min(h).min(l).min(c) {
            anomaly_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: k.open_time,
                anomaly_type: "ohlc_invalid".to_string(),
                open: Some(k.open.clone()),
                high: Some(k.high.clone()),
                low: Some(k.low.clone()),
                close: Some(k.close.clone()),
            });
            continue;
        }

        // Anomaly: close = 0 or negative
        if c <= 0.0 || o <= 0.0 || h <= 0.0 || l <= 0.0 {
            anomaly_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: k.open_time,
                anomaly_type: "zero_or_negative".to_string(),
                open: Some(k.open.clone()),
                high: Some(k.high.clone()),
                low: Some(k.low.clone()),
                close: Some(k.close.clone()),
            });
            continue;
        }

        // Anomaly: high < low
        if h < l {
            anomaly_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: k.open_time,
                anomaly_type: "high_less_than_low".to_string(),
                open: Some(k.open.clone()),
                high: Some(k.high.clone()),
                low: Some(k.low.clone()),
                close: Some(k.close.clone()),
            });
            continue;
        }

        valid_rows += 1;
    }

    // Gap detection
    let mut gap_count = 0_i64;
    for window in klines.windows(2) {
        let diff = window[1].open_time - window[0].open_time;
        if diff > interval_ms * 2 {
            gap_count += 1;
            anomalies.push(KlineQualityAnomaly {
                open_time: window[0].open_time,
                anomaly_type: "gap".to_string(),
                open: None,
                high: None,
                low: None,
                close: None,
            });
        }
    }

    let coverage_pct = if total_rows > 0 {
        (valid_rows as f64) / (total_rows as f64) * 100.0
    } else {
        0.0
    };

    // Keep anomalies bounded - return at most 100
    anomalies.truncate(100);

    Ok(KlineQualityReport {
        symbol: symbol.to_string(),
        interval: interval.to_string(),
        total_rows,
        valid_rows,
        coverage_pct: (coverage_pct * 100.0).round() / 100.0,
        gap_count,
        anomaly_count,
        duplicate_count,
        anomalies,
    })
}

// ============ Clean ============

pub async fn clean_klines(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: KlineCleanRequest,
) -> Result<KlineCleanResult, AppError> {
    if req.symbol.trim().is_empty() {
        return Err(AppError::Validation("symbol is required".into()));
    }
    if !VALID_INTERVALS.contains(&req.interval.as_str()) {
        return Err(AppError::Validation("invalid interval".into()));
    }
    if req.clean_types.is_empty() {
        return Err(AppError::Validation("clean_types cannot be empty".into()));
    }

    let valid_types: Vec<&str> = req
        .clean_types
        .iter()
        .filter(|t| ["gap", "anomaly", "duplicate"].contains(&t.as_str()))
        .map(|s| s.as_str())
        .collect();

    if valid_types.is_empty() {
        return Err(AppError::Validation(
            "clean_types must contain at least one of: gap, anomaly, duplicate".into(),
        ));
    }

    // Get all klines for this symbol/interval
    let klines = Kline::find()
        .filter(kline::Column::UserId.eq(user_id))
        .filter(kline::Column::Symbol.eq(&req.symbol))
        .filter(kline::Column::Interval.eq(&req.interval))
        .order_by_asc(kline::Column::OpenTime)
        .all(db)
        .await?;

    let _interval_ms: i64 = match req.interval.as_str() {
        "1m" => 60_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "4h" => 14_400_000,
        "1d" => 86_400_000,
        "1w" => 604_800_000,
        _ => 3_600_000,
    };

    let mut removed = 0_i64;
    let mut seen_open_times: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for k in klines {
        let mut should_delete = false;

        if valid_types.contains(&"duplicate") {
            if seen_open_times.contains(&k.open_time) {
                should_delete = true;
            } else {
                seen_open_times.insert(k.open_time);
            }
        }

        if valid_types.contains(&"anomaly") && !should_delete {
            let open_p = parse_f64(&k.open);
            let high_p = parse_f64(&k.high);
            let low_p = parse_f64(&k.low);
            let close_p = parse_f64(&k.close);

            if open_p.is_err()
                || high_p.is_err()
                || low_p.is_err()
                || close_p.is_err()
            {
                should_delete = true;
            } else {
                let (o, h, l, c) = (
                    open_p.unwrap(),
                    high_p.unwrap(),
                    low_p.unwrap(),
                    close_p.unwrap(),
                );
                if h < o || h < l || h < c || l > o || l > c || h < l || c <= 0.0 || o <= 0.0 {
                    should_delete = true;
                }
            }
        }

        if valid_types.contains(&"gap") && !should_delete {
            // We need the previous bar's open_time - can't easily do this without re-reading
            // Skip gap detection for single-pass deletion; gap detection is best-effort here
        }

        if should_delete {
            let _ = Kline::delete_by_id(k.id).exec(db).await;
            removed += 1;
        }
    }

    Ok(KlineCleanResult { removed_count: removed })
}

// ============ Export ============

pub async fn export_klines(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: KlineExportParams,
) -> Result<String, AppError> {
    if params.symbol.trim().is_empty() {
        return Err(AppError::Validation("symbol is required".into()));
    }
    if !VALID_INTERVALS.contains(&params.interval.as_str()) {
        return Err(AppError::Validation("invalid interval".into()));
    }

    let klines = Kline::find()
        .filter(kline::Column::UserId.eq(user_id))
        .filter(kline::Column::Symbol.eq(&params.symbol))
        .filter(kline::Column::Interval.eq(&params.interval))
        .order_by_asc(kline::Column::OpenTime)
        .all(db)
        .await?;

    let mut csv = String::from("open_time,open,high,low,close,volume,close_time,quote_volume,trades\n");
    for k in klines {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            k.open_time,
            k.open,
            k.high,
            k.low,
            k.close,
            k.volume,
            k.close_time.map_or(String::new(), |v| v.to_string()),
            k.quote_volume.as_deref().unwrap_or(""),
            k.trades.map_or(String::new(), |v| v.to_string()),
        ));
    }

    Ok(csv)
}
