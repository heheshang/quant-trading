// ============ CRUD Service Functions ============
//
// All public API lives here. Trait/registry imports are taken from sibling
// submodules; SeaORM and schema types from `crate::*` and `sea_orm::*`.
//
// `int_param!` and `float_param!` macros live in `super::templates_impls` and
// are used by tests defined at the bottom of this file.
use super::registry::{
    get_all_templates, get_template, model_to_response, template_to_info,
    template_type_to_uuid, validate_status_transition,
};
// The following imports are referenced only from the `#[cfg(test)] mod tests`
// block at the bottom of this file. They are `#[allow(unused_imports)]` so
// that `cargo check` (which does not compile test code) stays warning-free.
#[allow(unused_imports)]
use super::templates_impls::{
    AtrStopTemplate, BollingerTemplate, DoubleBollingerTemplate, IchimokuTemplate, KeltnerTemplate,
    MacdTemplate, MaCrossoverTemplate, MeanReversionTemplate, RsiTemplate, TripleMaTemplate,
};
#[allow(unused_imports)]
use crate::{float_param, int_param};
use crate::db::strategy;
#[allow(unused_imports)]
use crate::models::backtest::{Kline, Signal};
use crate::models::schemas::{
    BulkUpdateStatusRequest, CreateStrategyRequest, ImportBatchRequest, ImportBatchResponse,
    ImportStrategyRequest, PaginatedResponse, PaginationParams, StrategyResponse, TemplateInfo,
    UpdateStrategyRequest,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
#[allow(unused_imports)]
use serde_json::Value;
use uuid::Uuid;

pub async fn list_templates() -> Result<Vec<TemplateInfo>, AppError> {
    Ok(get_all_templates()
        .into_iter()
        .map(template_to_info)
        .collect())
}

// Resolve template_type from template_id UUID.
// Since templates are currently in-memory with string IDs, we use a deterministic
// UUID mapping. In production, this would query a templates database table.
fn resolve_template_type_from_id(template_id: &Uuid) -> Option<String> {
    // Check if template_id matches any known template's UUID
    let all_templates = get_all_templates();
    for t in all_templates {
        if template_id == &template_type_to_uuid(t.id()) {
            return Some(t.id().to_string());
        }
    }
    None
}

pub async fn create_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: CreateStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    // Resolve template_type from template_id
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Log the template_id being used
    tracing::info!(
        "Creating strategy with template_id: {}, resolved template_type: {}",
        req.template_id,
        template_type
    );

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol (non-empty, alphanumeric+USDT suffix)
    if req.symbol.trim().is_empty() {
        return Err(AppError::Validation("Symbol cannot be empty".into()));
    }
    if req.symbol.len() > 20 {
        return Err(AppError::Validation(
            "Symbol must be <= 20 characters".into(),
        ));
    }

    // Validate timeframe (allowed values)
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}. Allowed: trend_following/mean_reversion/grid_trading/arbitrage/custom",
            req.strategy_type
        )));
    }

    let now = chrono::Utc::now();
    let description = req.description.unwrap_or_default();
    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(req.name),
        description: Set(description),
        symbol: Set(req.symbol),
        timeframe: Set(req.timeframe),
        strategy_type: Set(req.strategy_type),
        template_type: Set(template_type),
        parameters: Set(req.parameters),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = model.insert(db).await?;
    Ok(model_to_response(saved))
}

pub async fn list_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: PaginationParams,
) -> Result<PaginatedResponse<StrategyResponse>, AppError> {
    let page = params.page();
    let size = params.size();
    let offset = params.offset();

    let total = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .count(db)
        .await? as u64;

    let items = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .order_by_desc(strategy::Column::UpdatedAt)
        .offset(offset)
        .limit(size)
        .all(db)
        .await?
        .into_iter()
        .map(model_to_response)
        .collect();

    Ok(PaginatedResponse {
        items,
        total,
        page,
        size,
    })
}

pub async fn export_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    status_filter: Option<&str>,
) -> Result<Vec<StrategyResponse>, AppError> {
    let mut query = strategy::Entity::find().filter(strategy::Column::UserId.eq(user_id));

    if let Some(status) = status_filter {
        query = query.filter(strategy::Column::Status.eq(status));
    }

    let items: Vec<StrategyResponse> = query
        .order_by_desc(strategy::Column::UpdatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(model_to_response)
        .collect();

    Ok(items)
}

pub async fn get_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    Ok(model_to_response(m))
}

pub async fn update_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
    req: UpdateStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    let mut active: strategy::ActiveModel = m.clone().into();
    active.updated_at = Set(chrono::Utc::now());

    if let Some(name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::Validation("Strategy name cannot be empty".into()));
        }
        if name.len() > 100 {
            return Err(AppError::Validation(
                "Strategy name must be <= 100 characters".into(),
            ));
        }
        active.name = Set(name);
    }

    if let Some(description) = req.description {
        active.description = Set(description);
    }

    if let Some(params) = req.parameters {
        let template = get_template(&m.template_type)
            .ok_or_else(|| AppError::Internal("Template not found for existing strategy".into()))?;
        template
            .validate(&params)
            .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;
        active.parameters = Set(params);
    }

    let saved = active.update(db).await?;
    Ok(model_to_response(saved))
}

pub async fn delete_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
) -> Result<(), AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    strategy::Entity::delete_by_id(strategy_id).exec(db).await?;

    Ok(())
}

pub async fn update_strategy_status(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
    new_status: String,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    validate_status_transition(&m.status, &new_status)?;

    let mut active: strategy::ActiveModel = m.into();
    active.status = Set(new_status);
    active.updated_at = Set(chrono::Utc::now());

    let saved = active.update(db).await?;
    Ok(model_to_response(saved))
}

pub async fn bulk_update_status(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: BulkUpdateStatusRequest,
) -> Result<Vec<StrategyResponse>, AppError> {
    if req.ids.is_empty() {
        return Err(AppError::Validation("ids cannot be empty".into()));
    }

    let valid_statuses = ["active", "paused", "draft"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid status: {}. Must be one of: active, paused, draft",
            req.status
        )));
    }

    let mut results = Vec::new();
    for strategy_id in req.ids {
        let m = strategy::Entity::find_by_id(strategy_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

        if m.user_id != user_id {
            return Err(AppError::NotFound("Strategy not found".into()));
        }

        validate_status_transition(&m.status, &req.status)?;

        let mut active: strategy::ActiveModel = m.into();
        active.status = Set(req.status.clone());
        active.updated_at = Set(chrono::Utc::now());

        let saved = active.update(db).await?;
        results.push(model_to_response(saved));
    }
    Ok(results)
}

// ============ Import/Export ============

pub async fn import_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: crate::models::schemas::ImportStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    // Resolve template_type from template_id or fallback to provided template_type
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Log the template_id being used
    tracing::info!(
        "Importing strategy with template_id: {}, resolved template_type: {}",
        req.template_id,
        template_type
    );

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol
    if req.symbol.trim().is_empty() || req.symbol.len() > 20 {
        return Err(AppError::Validation("Invalid symbol".into()));
    }

    // Validate timeframe
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}",
            req.strategy_type
        )));
    }

    // Check for duplicate name within user's strategies
    let existing = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .filter(strategy::Column::Name.eq(&req.name))
        .one(db)
        .await?;
    let final_name = if existing.is_some() {
        // Append import suffix with counter
        let mut counter = 2;
        loop {
            let candidate = format!("{} (import-{})", req.name, counter);
            let exists = strategy::Entity::find()
                .filter(strategy::Column::UserId.eq(user_id))
                .filter(strategy::Column::Name.eq(&candidate))
                .one(db)
                .await?;
            if exists.is_none() {
                break candidate;
            }
            counter += 1;
        }
    } else {
        req.name.clone()
    };

    let now = chrono::Utc::now();
    let description = req.description.unwrap_or_default();
    let status = req.status.unwrap_or_else(|| "draft".to_string());

    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(final_name),
        description: Set(description),
        symbol: Set(req.symbol),
        timeframe: Set(req.timeframe),
        strategy_type: Set(req.strategy_type),
        template_type: Set(template_type),
        parameters: Set(req.parameters),
        status: Set(status),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = model.insert(db).await?;
    Ok(model_to_response(saved))
}

/// Import a batch of strategies from JSON export (ADR D2)
pub async fn import_batch(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: ImportBatchRequest,
) -> Result<ImportBatchResponse, AppError> {
    let mut imported = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, strategy_req) in req.strategies.iter().enumerate() {
        match import_single_strategy(db, user_id, strategy_req).await {
            Ok(_) => imported += 1,
            Err(e) => {
                errors.push(format!(
                    "策略[{}]「{}」导入失败: {}",
                    i + 1,
                    &strategy_req.name,
                    e
                ));
            }
        }
    }

    Ok(ImportBatchResponse { imported, errors })
}

/// Shared logic for importing a single strategy (validates, deduplicates name, inserts)
async fn import_single_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: &ImportStrategyRequest,
) -> Result<(), AppError> {
    // Resolve template_type from template_id or fallback to provided template_type
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol
    if req.symbol.trim().is_empty() || req.symbol.len() > 20 {
        return Err(AppError::Validation("Invalid symbol".into()));
    }

    // Validate timeframe
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}",
            req.strategy_type
        )));
    }

    // Check for duplicate name within user's strategies
    let existing = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .filter(strategy::Column::Name.eq(&req.name))
        .one(db)
        .await?;
    let final_name = if existing.is_some() {
        // Append import suffix with counter
        let mut counter = 2;
        loop {
            let candidate = format!("{} (import-{})", req.name, counter);
            let exists = strategy::Entity::find()
                .filter(strategy::Column::UserId.eq(user_id))
                .filter(strategy::Column::Name.eq(&candidate))
                .one(db)
                .await?;
            if exists.is_none() {
                break candidate;
            }
            counter += 1;
        }
    } else {
        req.name.clone()
    };

    let now = chrono::Utc::now();
    let description = req.description.clone().unwrap_or_default();
    let status = req.status.clone().unwrap_or_else(|| "draft".to_string());

    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(final_name),
        description: Set(description),
        symbol: Set(req.symbol.clone()),
        timeframe: Set(req.timeframe.clone()),
        strategy_type: Set(req.strategy_type.clone()),
        template_type: Set(template_type.clone()),
        parameters: Set(req.parameters.clone()),
        status: Set(status),
        created_at: Set(now),
        updated_at: Set(now),
    };

    model.insert(db).await?;
    Ok(())
}

// ============ Bulk Delete ============

/// Delete multiple strategies by IDs for a user
pub async fn bulk_delete_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    ids: &[Uuid],
) -> Result<BulkDeleteResponse, AppError> {
    if ids.is_empty() {
        return Ok(BulkDeleteResponse { deleted: 0 });
    }

    let mut deleted = 0_i64;
    for id in ids {
        let strategy = strategy::Entity::find_by_id(*id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

        if strategy.user_id != user_id {
            return Err(AppError::NotFound("Strategy not found".into()));
        }

        strategy::Entity::delete_by_id(*id).exec(db).await?;
        deleted += 1;
    }

    Ok(BulkDeleteResponse { deleted })
}

// ============ Strategy Code Storage ============

/// Store strategy code content (simple file-based storage)
pub async fn store_strategy_code(
    _db: &DatabaseConnection,
    user_id: Uuid,
    file_name: &str,
    content: &str,
) -> Result<String, AppError> {
    use std::fs;

    // Determine storage directory
    let storage_dir =
        std::env::var("STRATEGY_CODE_DIR").unwrap_or_else(|_| "/tmp/strategy_codes".to_string());

    let user_dir = format!("{}/{}", storage_dir, user_id);
    fs::create_dir_all(&user_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create strategy directory: {}", e)))?;

    let safe_name = file_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect::<String>();

    let path = format!("{}/{}", user_dir, safe_name);
    fs::write(&path, content)
        .map_err(|e| AppError::Internal(format!("Failed to write strategy code file: {}", e)))?;

    Ok(path)
}

#[derive(Debug, serde::Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    // `sma`, `rsi`, `ema` etc. are `pub(crate)` helpers defined in the sibling
    // `common` module — tests need them explicitly because `use super::*` does
    // not reach sibling submodules.
    use super::super::common::{atr, ema, macd, rsi, sma, stddev};
    use super::super::common::StrategyTemplate;

    // Helper to create klines for testing
    fn make_klines(prices: &[f64]) -> Vec<Kline> {
        prices
            .iter()
            .enumerate()
            .map(|(i, p)| Kline {
                open_time: i as i64 * 3600000,
                open: *p,
                high: *p * 1.01,
                low: *p * 0.99,
                close: *p,
                volume: 1000.0,
            })
            .collect()
    }

    // ===== MaCrossover Tests =====
    #[test]
    fn test_ma_crossover_default_valid() {
        let t = MaCrossoverTemplate;
        let params = t.default_parameters();
        assert!(t.validate(&params).is_ok());
    }

    #[test]
    fn test_ma_crossover_boundary() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 5, "slow_period": 10}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 50, "slow_period": 200}))
                .is_ok()
        );
    }

    #[test]
    fn test_ma_crossover_fast_gte_slow() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 30, "slow_period": 10}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 10}))
                .is_err()
        );
    }

    #[test]
    fn test_ma_crossover_invalid_values() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 0, "slow_period": 30}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 100, "slow_period": 30}))
                .is_err()
        );
    }

    #[test]
    fn test_ma_crossover_missing_fields() {
        let t = MaCrossoverTemplate;
        assert!(t.validate(&serde_json::json!({})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 10})).is_err());
    }

    // ===== MaCrossover Signal Tests =====
    #[test]
    fn test_ma_crossover_signal_buy() {
        let t = MaCrossoverTemplate;
        let params = serde_json::json!({"fast_period": 3, "slow_period": 5});
        // Uptrend: price goes 100→110→120→130→140, fast MA should cross above slow
        let klines = make_klines(&[
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0,
        ]);
        let signal = t.generate_signal(&klines, klines.len() - 1, &params);
        // In a strong uptrend, fast MA > slow MA, but we need a crossover event
        // Previous: fast > slow already, so this should be Hold
        assert_eq!(signal, Signal::Hold);
    }

    #[test]
    fn test_ma_crossover_signal_immature() {
        let t = MaCrossoverTemplate;
        let params = serde_json::json!({"fast_period": 3, "slow_period": 5});
        let klines = make_klines(&[100.0, 101.0, 102.0]); // not enough bars
        let signal = t.generate_signal(&klines, 2, &params);
        assert_eq!(signal, Signal::Hold);
    }

    // ===== TripleMA Tests =====
    #[test]
    fn test_triple_ma_default_valid() {
        let t = TripleMaTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_triple_ma_ordering() {
        let t = TripleMaTemplate;
        assert!(
            t.validate(&serde_json::json!({"short": 10, "medium": 5, "long": 50}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 5}))
                .is_err()
        );
    }

    #[test]
    fn test_triple_ma_boundary() {
        let t = TripleMaTemplate;
        assert!(
            t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 20}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"short": 20, "medium": 50, "long": 200}))
                .is_ok()
        );
    }

    // ===== MACD Tests =====
    #[test]
    fn test_macd_default_valid() {
        let t = MacdTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_macd_fast_lt_slow() {
        let t = MacdTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast": 26, "slow": 12, "signal": 9}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 12, "signal": 9}))
                .is_err()
        );
    }

    #[test]
    fn test_macd_signal_range() {
        let t = MacdTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 1}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 50}))
                .is_err()
        );
    }

    // ===== Bollinger Tests =====
    #[test]
    fn test_bollinger_default_valid() {
        let t = BollingerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_bollinger_std_dev_range() {
        let t = BollingerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 5.0}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 1.5}))
                .is_ok()
        );
    }

    // ===== RSI Tests =====
    #[test]
    fn test_rsi_default_valid() {
        let t = RsiTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_rsi_overbought_oversold() {
        let t = RsiTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 30}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 75}))
                .is_err()
        );
    }

    // ===== Keltner Tests =====
    #[test]
    fn test_keltner_default_valid() {
        let t = KeltnerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_keltner_atr_range() {
        let t = KeltnerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 4.0}))
                .is_err()
        );
    }

    // ===== ATR Stop Tests =====
    #[test]
    fn test_atr_stop_default_valid() {
        let t = AtrStopTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_atr_stop_multiplier_range() {
        let t = AtrStopTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 14, "multiplier": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 14, "multiplier": 6.0}))
                .is_err()
        );
    }

    // ===== Mean Reversion Tests =====
    #[test]
    fn test_mean_reversion_default_valid() {
        let t = MeanReversionTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_mean_reversion_std_ordering() {
        let t = MeanReversionTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.0}))
                .is_err()
        );
    }

    // ===== Ichimoku Tests =====
    #[test]
    fn test_ichimoku_default_valid() {
        let t = IchimokuTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_ichimoku_boundary() {
        let t = IchimokuTemplate;
        assert!(
            t.validate(&serde_json::json!({"conversion": 5, "base": 10, "span": 20, "displ": 5}))
                .is_ok()
        );
        assert!(
            t.validate(
                &serde_json::json!({"conversion": 20, "base": 60, "span": 120, "displ": 60})
            )
            .is_ok()
        );
    }

    // ===== Double Bollinger Tests =====
    #[test]
    fn test_double_bollinger_default_valid() {
        let t = DoubleBollingerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_double_bollinger_inner_lt_outer() {
        let t = DoubleBollingerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "inner_std": 2.5, "outer_std": 1.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "inner_std": 2.0, "outer_std": 2.0}))
                .is_err()
        );
    }

    // ===== All Templates Tests =====
    #[test]
    fn test_all_templates_have_unique_ids() {
        let templates = get_all_templates();
        let mut ids: Vec<String> = templates.iter().map(|t| t.id().to_string()).collect();
        ids.sort();
        ids.dedup();
        // P1-1: 10 → 13 templates (added grid / martingale / breakout)
        assert_eq!(ids.len(), 13);
    }

    #[test]
    fn test_get_template_by_id() {
        let t = get_template("ma_crossover").unwrap();
        assert_eq!(t.id(), "ma_crossover");
        assert!(get_template("nonexistent").is_none());
    }

    #[test]
    fn test_all_templates_categories_valid() {
        // P1-1: added "momentum" (breakout) and "recovery" (martingale)
        let valid = ["trend", "mean_reversion", "volatility", "composite", "momentum", "recovery"];
        for t in get_all_templates() {
            assert!(
                valid.contains(&t.category()),
                "Invalid category: {}",
                t.category()
            );
        }
    }

    // ===== Status Transition Tests =====
    #[test]
    fn test_valid_status_transitions() {
        assert!(validate_status_transition("draft", "active").is_ok());
        assert!(validate_status_transition("active", "paused").is_ok());
        assert!(validate_status_transition("paused", "active").is_ok());
        assert!(validate_status_transition("paused", "stopped").is_ok());
    }

    #[test]
    fn test_invalid_status_transitions() {
        assert!(validate_status_transition("draft", "paused").is_err());
        assert!(validate_status_transition("draft", "stopped").is_err());
        assert!(validate_status_transition("active", "draft").is_err());
        assert!(validate_status_transition("active", "stopped").is_err());
        assert!(validate_status_transition("stopped", "active").is_err());
        assert!(validate_status_transition("stopped", "draft").is_err());
    }

    // ===== ParameterDef Tests =====
    #[test]
    fn test_int_param_values() {
        let p = int_param!("period", "Period", "The period", 20, 5, 100);
        assert_eq!(p.param_type, "integer");
        assert_eq!(p.default.as_i64(), Some(20));
        assert_eq!(p.min.as_ref().and_then(|v| v.as_i64()), Some(5));
    }

    #[test]
    fn test_float_param_values() {
        let p = float_param!("std_dev", "Std Dev", "Standard deviation", 2.5, 1.0, 5.0);
        assert_eq!(p.param_type, "float");
        assert!((p.default.as_f64().unwrap() - 2.5).abs() < 1e-10);
    }

    // ===== Technical Indicator Tests =====
    #[test]
    fn test_sma_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3);
        assert!((result[2] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2
        assert!((result[3] - 3.0).abs() < 1e-10); // (2+3+4)/3 = 3
        assert!((result[4] - 4.0).abs() < 1e-10); // (3+4+5)/3 = 4
    }

    #[test]
    fn test_sma_empty() {
        let result = sma(&[], 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rsi_all_rising() {
        let prices: Vec<f64> = (100..=130).map(|i| i as f64).collect();
        let rsi_vals = rsi(&prices, 14);
        // In a continuously rising market, RSI should be high (near 100)
        let last = rsi_vals[rsi_vals.len() - 1];
        assert!(last > 50.0, "RSI should be > 50 in uptrend, got {}", last);
    }

    #[test]
    fn test_rsi_all_falling() {
        let prices: Vec<f64> = (0..=30).rev().map(|i| 100.0 + i as f64).collect();
        let rsi_vals = rsi(&prices, 14);
        let last = rsi_vals[rsi_vals.len() - 1];
        assert!(last < 50.0, "RSI should be < 50 in downtrend, got {}", last);
    }

    #[test]
    fn test_ema_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = ema(&data, 3);
        // First value (idx 2) = SMA = 2.0
        assert!((result[2] - 2.0).abs() < 1e-10);
        // EMA[3] = (4.0 - 2.0) * 0.5 + 2.0 = 3.0
        assert!((result[3] - 3.0).abs() < 1e-10);
        // EMA[4] = (5.0 - 3.0) * 0.5 + 3.0 = 4.0
        assert!((result[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_stddev_constant() {
        let data = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let ma = sma(&data, 3);
        let sd = stddev(&data, 3, &ma);
        assert!(sd[4].abs() < 1e-10); // no variance
    }

    #[test]
    fn test_stddev_non_constant() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = sma(&data, 3);
        let sd = stddev(&data, 3, &ma);
        assert!(sd[2] > 0.0); // should have variance
    }

    #[test]
    fn test_macd_structure() {
        let data: Vec<f64> = (100..=200).map(|i| i as f64).collect();
        let (macd_line, signal_line, hist) = macd(&data, 12, 26, 9);
        assert_eq!(macd_line.len(), data.len());
        assert_eq!(signal_line.len(), data.len());
        assert_eq!(hist.len(), data.len());
    }

    #[test]
    fn test_atr_basic() {
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![11.0, 12.0, 13.0, 14.0, 15.0];
        let atr_vals = atr(&high, &low, &close, 3);
        assert!(atr_vals[atr_vals.len() - 1] > 0.0);
    }
}
