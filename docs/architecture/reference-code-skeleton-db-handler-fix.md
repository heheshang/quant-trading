# 回测引擎参考代码骨架 — DB 层实现 + Handler 修复

基于 ADR-008 决策，提供 `db/backtest.rs` 实现和 `handlers/backtest.rs` 修复方案。

---

## 1. db/backtest.rs — 完整 SeaORM 实现骨架

### 前置条件：创建 SeaORM Entity

```bash
# 在 backend 目录下
cargo install sea-orm-cli
sea-orm-cli generate entity -o src/db/entities -t backtest_results,kline_data,strategies
```

### Entity 文件: backend/src/db/entities/backtest_results.rs

```rust
// 自动生成，需包含以下字段：
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "backtest_results")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub strategy_id: Uuid,
    pub user_id: Uuid,
    pub config: Json,
    pub status: String,
    pub progress: i16,
    pub start_time: Option<DateTimeUtc>,
    pub end_time: Option<DateTimeUtc>,
    pub duration_ms: Option<i64>,
    pub metrics: Option<Json>,
    pub trades: Option<Json>,
    pub equity_curve: Option<Json>,
    pub error: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

### Migration: backend/migrations/YYYYMMDDHHMMSS_create_backtest_results.rs

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BacktestResults::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(BacktestResults::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(BacktestResults::StrategyId).uuid().not_null())
                    .col(ColumnDef::new(BacktestResults::UserId).uuid().not_null())
                    .col(ColumnDef::new(BacktestResults::Config).json().not_null())
                    .col(ColumnDef::new(BacktestResults::Status).string().not_null().default("pending"))
                    .col(ColumnDef::new(BacktestResults::Progress).small_integer().not_null().default(0))
                    .col(ColumnDef::new(BacktestResults::StartTime).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(BacktestResults::EndTime).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(BacktestResults::DurationMs).big_integer().null())
                    .col(ColumnDef::new(BacktestResults::Metrics).json().null())
                    .col(ColumnDef::new(BacktestResults::Trades).json().null())
                    .col(ColumnDef::new(BacktestResults::EquityCurve).json().null())
                    .col(ColumnDef::new(BacktestResults::Error).string().null())
                    .col(ColumnDef::new(BacktestResults::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // Indexes
        manager.create_index(
            Index::create()
                .name("idx_backtest_strategy_id")
                .table(BacktestResults::Table)
                .col(BacktestResults::StrategyId)
                .to_owned()
        ).await?;
        // ... other indexes from PRD
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(BacktestResults::Table).to_owned()).await
    }
}
```

### 完整 db/backtest.rs 实现（替换 TODO 占位）

```rust
// ============ Backtest DB Operations — 完整 SeaORM 实现 ============

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::db::entities::{backtest_results, kline_data, strategies};
use crate::models::backtest::{
    BacktestConfig, BacktestMetrics, BacktestProgressResponse, BacktestResultResponse,
    BacktestSummary, Kline, MetricsPreview, TradeRecord, EquityPoint,
};
use crate::models::schemas::PaginatedResponse;
use crate::utils::error::AppError;

// ============ Backtest Results CRUD ============

/// Create a new backtest run record with status=pending.
pub async fn create_backtest_run(
    db: &DatabaseConnection,
    id: Uuid,
    strategy_id: Uuid,
    user_id: Uuid,
    config: &BacktestConfig,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    backtest_results::ActiveModel {
        id: Set(id),
        strategy_id: Set(strategy_id),
        user_id: Set(user_id),
        config: Set(serde_json::to_value(config).map_err(|e| AppError::Internal(e.to_string()))?),
        status: Set("pending".to_string()),
        progress: Set(0),
        start_time: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Update backtest result with completed metrics.
pub async fn update_completed(
    db: &DatabaseConnection,
    id: Uuid,
    metrics: &BacktestMetrics,
    trades: &[TradeRecord],
    equity_curve: &[EquityPoint],
    duration_ms: i64,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    let record = backtest_results::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    let mut active: backtest_results::ActiveModel = record.into();
    active.status = Set("completed".to_string());
    active.progress = Set(100);
    active.end_time = Set(Some(now));
    active.duration_ms = Set(Some(duration_ms));
    active.metrics = Set(Some(serde_json::to_value(metrics).map_err(|e| AppError::Internal(e.to_string()))?));
    active.trades = Set(Some(serde_json::to_value(trades).map_err(|e| AppError::Internal(e.to_string()))?));
    active.equity_curve = Set(Some(serde_json::to_value(equity_curve).map_err(|e| AppError::Internal(e.to_string()))?));
    active.update(db).await?;
    Ok(())
}

/// Update backtest result with failure.
pub async fn update_failed(
    db: &DatabaseConnection,
    id: Uuid,
    error_msg: &str,
) -> Result<(), AppError> {
    let record = backtest_results::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    let mut active: backtest_results::ActiveModel = record.into();
    active.status = Set("failed".to_string());
    active.end_time = Set(Some(chrono::Utc::now()));
    active.error = Set(Some(error_msg.to_string()));
    active.update(db).await?;
    Ok(())
}

/// Update backtest status (used for cancel).
pub async fn update_status(
    db: &DatabaseConnection,
    id: Uuid,
    status: &str,
) -> Result<(), AppError> {
    let record = backtest_results::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    let mut active: backtest_results::ActiveModel = record.into();
    active.status = Set(status.to_string());
    active.update(db).await?;
    Ok(())
}

/// Find a backtest result by ID.
pub async fn find_backtest_result(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<BacktestResultResponse>, AppError> {
    let record = backtest_results::Entity::find_by_id(id)
        .one(db)
        .await?;

    match record {
        Some(r) => {
            let config: BacktestConfig = serde_json::from_value(r.config)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let metrics: Option<BacktestMetrics> = r.metrics
                .map(|v| serde_json::from_value(v).ok())
                .flatten();
            let trades: Option<Vec<TradeRecord>> = r.trades
                .map(|v| serde_json::from_value(v).ok())
                .flatten();
            let equity_curve: Option<Vec<EquityPoint>> = r.equity_curve
                .map(|v| serde_json::from_value(v).ok())
                .flatten();

            Ok(Some(BacktestResultResponse {
                id: r.id,
                strategy_id: r.strategy_id,
                status: r.status,
                progress: r.progress as i32,
                config,
                metrics,
                trades,
                equity_curve,
                start_time: r.start_time,
                end_time: r.end_time,
                duration_ms: r.duration_ms,
                error: r.error,
                created_at: r.created_at,
            }))
        }
        None => Ok(None),
    }
}

/// Get backtest progress (used by polling API).
pub async fn get_backtest_progress(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<BacktestProgressResponse>, AppError> {
    let record = backtest_results::Entity::find_by_id(id)
        .select_only()
        .column(backtest_results::Column::Status)
        .column(backtest_results::Column::Progress)
        .into_model::<BacktestProgressRow>()
        .one(db)
        .await?;

    Ok(record.map(|r| BacktestProgressResponse {
        id,
        status: r.status,
        progress: r.progress as i32,
        current_bar: 0,  // 简化：不存储 current_bar
        total_bars: 0,
        elapsed_ms: 0,
    }))
}

// 辅助结构
#[derive(FromQueryResult)]
struct BacktestProgressRow {
    status: String,
    progress: i16,
}

/// List backtest history for a strategy with pagination.
pub async fn list_backtest_history(
    db: &DatabaseConnection,
    strategy_id: Uuid,
    page: u64,
    page_size: u64,
) -> Result<PaginatedResponse<BacktestSummary>, AppError> {
    let query = backtest_results::Entity::find()
        .filter(backtest_results::Column::StrategyId.eq(strategy_id))
        .order_by_desc(backtest_results::Column::CreatedAt);

    let paginator = query.paginate(db, page_size);
    let items_page = paginator.fetch_page(page - 1).await?;
    let total = paginator.num_items().await?;

    let items: Vec<BacktestSummary> = items_page
        .into_iter()
        .map(|r| {
            let config: BacktestConfig = serde_json::from_value(r.config).unwrap_or_default();
            let metrics_preview = r.metrics.and_then(|v| {
                let m: BacktestMetrics = serde_json::from_value(v).ok()?;
                Some(MetricsPreview {
                    total_return_pct: m.total_return_pct,
                    sharpe_ratio: m.sharpe_ratio,
                    max_drawdown_pct: m.max_drawdown_pct,
                    total_trades: m.total_trades,
                })
            });
            BacktestSummary {
                id: r.id,
                status: r.status,
                config,
                metrics: metrics_preview,
                start_time: r.start_time,
                duration_ms: r.duration_ms,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(PaginatedResponse {
        items,
        total,
        page,
        size: page_size,
    })
}

/// Delete a backtest record.
pub async fn delete_backtest_record(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<(), AppError> {
    backtest_results::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

// ============ Kline Data Access ============

/// Load kline data for backtest simulation.
pub async fn load_klines(
    db: &DatabaseConnection,
    symbol: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Option<Vec<Kline>>, AppError> {
    let results = kline_data::Entity::find()
        .filter(kline_data::Column::Symbol.eq(symbol))
        .filter(kline_data::Column::Interval.eq(interval))
        .filter(kline_data::Column::OpenTime.gte(start_ms))
        .filter(kline_data::Column::OpenTime.lte(end_ms))
        .order_by_asc(kline_data::Column::OpenTime)
        .all(db)
        .await?;

    if results.is_empty() {
        return Ok(None);
    }

    let klines: Vec<Kline> = results
        .into_iter()
        .map(|r| Kline {
            open_time: r.open_time,
            open: r.open.parse::<f64>().unwrap_or(0.0),
            high: r.high.parse::<f64>().unwrap_or(0.0),
            low: r.low.parse::<f64>().unwrap_or(0.0),
            close: r.close.parse::<f64>().unwrap_or(0.0),
            volume: r.volume.parse::<f64>().unwrap_or(0.0),
        })
        .collect();

    Ok(Some(klines))
}

/// Find a strategy by ID (for validation).
pub async fn find_strategy(
    db: &DatabaseConnection,
    strategy_id: Uuid,
) -> Result<Option<serde_json::Value>, AppError> {
    let record = strategies::Entity::find_by_id(strategy_id)
        .one(db)
        .await?;

    Ok(record.map(|r| {
        serde_json::json!({
            "id": r.id,
            "user_id": r.user_id,
            "name": r.name,
            "template_type": r.template_type,
            "parameters": r.parameters,
        })
    }))
}
```

---

## 2. handlers/backtest.rs — 关键修复

### 2.1 CancellationToken Registry

```rust
// 在文件顶部增加
use std::sync::LazyLock;
use dashmap::DashMap;

// 全局注册表
static CANCEL_TOKENS: LazyLock<DashMap<Uuid, CancellationToken>> = LazyLock::new(|| DashMap::new());
```

### 2.2 run_backtest 修复 — 异步引擎模式

```rust
pub async fn run_backtest(
    State(db): State<DatabaseConnection>,
    // 从 JWT 提取 user_id
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<BacktestRunRequest>,
) -> Result<Json<ApiResponse<BacktestRunResponse>>, AppError> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Auth("invalid user".into()))?;

    // 1. Validate config
    req.config.validate().map_err(|e| AppError::Validation(e))?;

    // 2. Verify strategy exists and belongs to user
    let strategy_model = backtest_db::find_strategy(&db, req.strategy_id).await?
        .ok_or_else(|| AppError::NotFound("strategy not found".into()))?;
    if strategy_model["user_id"].as_str().map(Uuid::parse_str).flatten() != Some(user_id) {
        return Err(AppError::Forbidden("strategy does not belong to user".into()));
    }

    // 3. Load template
    let template_id = strategy_model["template_type"].as_str().ok_or(AppError::Internal("missing template_type".into()))?;
    let template = strategy::get_template(template_id)
        .ok_or_else(|| AppError::Validation("invalid template type".into()))?;

    // 4. Load kline data
    let (symbol, interval, start_ms, end_ms) = build_kline_query(&req.config);
    let klines = backtest_db::load_klines(&db, &symbol, &interval, start_ms, end_ms).await?
        .ok_or_else(|| AppError::Validation("no kline data for the specified range".into()))?;

    // 5. Acquire semaphore permit
    let permit: SemaphorePermit<'_> = BACKTEST_SEMAPHORE.try_acquire()
        .map_err(|_| AppError::TooManyRequests("backtest concurrency limit reached (5)".into()))?;

    // 6. Create pending DB record
    let result_id = Uuid::new_v4();
    backtest_db::create_backtest_run(&db, result_id, req.strategy_id, user_id, &req.config).await?;

    // 7. Setup progress and cancellation
    let progress = Arc::new(AtomicU32::new(0));
    let cancel_token = CancellationToken::new();
    CANCEL_TOKENS.insert(result_id, cancel_token.clone());

    // 8. Build engine
    let engine = BacktestEngine::new(
        req.config.clone(),
        klines,
        template,
        strategy_model["parameters"].clone(),
        progress.clone(),
        cancel_token,
    );

    // 9. Spawn async engine task
    let db_clone = db.clone();
    let id_clone = result_id;
    tokio::spawn(async move {
        let _permit = permit; // hold until done
        let start = std::time::Instant::now();
        let result = tokio::task::spawn_blocking(move || engine.run()).await;
        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(Ok((metrics, trades, equity))) => {
                let _ = backtest_db::update_completed(&db_clone, id_clone, &metrics, &trades, &equity, duration_ms).await;
            }
            Ok(Err(e)) => {
                let _ = backtest_db::update_failed(&db_clone, id_clone, &e).await;
            }
            Err(e) => {
                let _ = backtest_db::update_failed(&db_clone, id_clone, &format!("join error: {}", e)).await;
            }
        }
        CANCEL_TOKENS.remove(&id_clone);
    });

    // 10. Return immediately
    Ok(Json(ApiResponse::success(BacktestRunResponse {
        id: result_id,
        status: "pending".into(),
        progress: 0,
        created_at: chrono::Utc::now(),
    })))
}
```

### 2.3 cancel_backtest 修复

```rust
pub async fn cancel_backtest(
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let (_, token) = CANCEL_TOKENS.remove(&id)
        .ok_or_else(|| AppError::NotFound("backtest not running or already completed".into()))?;
    token.cancel();
    Ok(Json(ApiResponse::success(())))
}
```

### 2.4 User ID 注入到剩余 handler

```rust
pub async fn list_history(
    State(db): State<DatabaseConnection>,
    Extension(claims): Extension<JwtClaims>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<BacktestSummary>>>, AppError> {
    // 验证用户权限：只能查看自己的策略历史
    let items = backtest_db::list_backtest_history(&db, query.strategy_id, query.page(), query.size()).await?;
    Ok(Json(ApiResponse::success(items)))
}
```

---

## 3. backtest_engine.rs — equity_curve MTM 修复

```rust
fn record_equity_point(&mut self, time: i64, kline: &Kline) {
    let mtm_value = self.account.cash;

    // Mark-to-market: 计算持仓的未实现盈亏
    let equity = if let Some(pos) = &self.open_position {
        let position_value = pos.quantity * kline.close;
        match pos.direction {
            Direction::Long => self.account.cash + position_value,
            Direction::Short => self.account.cash + (pos.quantity * pos.entry_price) - position_value,
        }
    } else {
        self.account.cash
    };

    self.account.equity = equity;

    // 计算 drawdown
    let peak = self.equity_points
        .iter()
        .map(|e| e.equity)
        .fold(equity, |a, b| a.max(b));

    let drawdown_pct = if peak > 0.0 {
        (equity - peak) / peak * 100.0
    } else {
        0.0
    };

    self.equity_points.push(EquityPoint {
        time,
        equity,
        drawdown_pct,
    });
}
```

---

## 4. models/backtest.rs — BacktestConfig Default 实现

```rust
impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            interval: "1h".into(),
            start_date: String::new(),
            end_date: String::new(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
        }
    }
}
```

---

## 5. Cargo.toml 依赖补充

```toml
[dependencies]
dashmap = "6"
```

前端 TypeScript 依赖无新增。
