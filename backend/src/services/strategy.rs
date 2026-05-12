use crate::db::strategy;
use crate::models::schemas::{
    CreateStrategyRequest, PaginatedResponse, PaginationParams, ParameterDef,
    StrategyResponse, TemplateInfo, UpdateStrategyRequest,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::Value;
use uuid::Uuid;

// ============ StrategyTemplate Trait ============

pub trait StrategyTemplate: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn default_parameters(&self) -> Value;
    fn parameter_schema(&self) -> Vec<ParameterDef>;
    fn validate(&self, params: &Value) -> Result<(), String>;
}

// ============ Template Implementations ============

pub struct MaCrossoverTemplate;
pub struct TripleMaTemplate;
pub struct MacdTemplate;
pub struct BollingerTemplate;
pub struct RsiTemplate;
pub struct KeltnerTemplate;
pub struct AtrStopTemplate;
pub struct MeanReversionTemplate;
pub struct IchimokuTemplate;
pub struct DoubleBollingerTemplate;

macro_rules! int_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {
        ParameterDef {
            name: $name.to_string(),
            param_type: "integer".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: Value::Number(($default).into()),
            min: Some(Value::Number(($min).into())),
            max: Some(Value::Number(($max).into())),
            options: None,
        }
    };
}

macro_rules! float_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {
        ParameterDef {
            name: $name.to_string(),
            param_type: "float".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: serde_json::json!($default),
            min: Some(serde_json::json!($min)),
            max: Some(serde_json::json!($max)),
            options: None,
        }
    };
}

fn get_i64(params: &Value, key: &str) -> Option<i64> {
    params.get(key)?.as_i64()
}

fn get_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key)?.as_f64()
}

// ========== 1. MaCrossover ==========

impl StrategyTemplate for MaCrossoverTemplate {
    fn id(&self) -> &str { "ma_crossover" }
    fn name(&self) -> &str { "MA Crossover" }
    fn description(&self) -> &str { "Moving average crossover strategy using fast and slow MAs" }
    fn category(&self) -> &str { "trend" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"fast_period": 10, "slow_period": 30})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("fast_period", "Fast Period", "Fast moving average period", 10, 5, 50),
            int_param!("slow_period", "Slow Period", "Slow moving average period", 30, 10, 200),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let fast = get_i64(params, "fast_period").ok_or("fast_period is required")?;
        let slow = get_i64(params, "slow_period").ok_or("slow_period is required")?;
        if fast < 5 { return Err("fast_period must be >= 5".into()); }
        if fast > 50 { return Err("fast_period must be <= 50".into()); }
        if slow < 10 { return Err("slow_period must be >= 10".into()); }
        if slow > 200 { return Err("slow_period must be <= 200".into()); }
        if fast >= slow { return Err("fast_period must be less than slow_period".into()); }
        Ok(())
    }
}

// ========== 2. TripleMA ==========

impl StrategyTemplate for TripleMaTemplate {
    fn id(&self) -> &str { "triple_ma" }
    fn name(&self) -> &str { "Triple MA" }
    fn description(&self) -> &str { "Triple moving average crossover strategy" }
    fn category(&self) -> &str { "trend" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"short": 9, "medium": 21, "long": 50})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("short", "Short Period", "Short MA period", 9, 5, 20),
            int_param!("medium", "Medium Period", "Medium MA period", 21, 10, 50),
            int_param!("long", "Long Period", "Long MA period", 50, 20, 200),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let short = get_i64(params, "short").ok_or("short is required")?;
        let medium = get_i64(params, "medium").ok_or("medium is required")?;
        let long = get_i64(params, "long").ok_or("long is required")?;
        if short < 5 { return Err("short must be >= 5".into()); }
        if short > 20 { return Err("short must be <= 20".into()); }
        if medium < 10 { return Err("medium must be >= 10".into()); }
        if medium > 50 { return Err("medium must be <= 50".into()); }
        if long < 20 { return Err("long must be >= 20".into()); }
        if long > 200 { return Err("long must be <= 200".into()); }
        if !(short < medium && medium < long) {
            return Err("must satisfy short < medium < long".into());
        }
        Ok(())
    }
}

// ========== 3. MACD ==========

impl StrategyTemplate for MacdTemplate {
    fn id(&self) -> &str { "macd" }
    fn name(&self) -> &str { "MACD" }
    fn description(&self) -> &str { "MACD (Moving Average Convergence Divergence) strategy" }
    fn category(&self) -> &str { "trend" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"fast": 12, "slow": 26, "signal": 9})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("fast", "Fast Period", "Fast EMA period", 12, 2, 50),
            int_param!("slow", "Slow Period", "Slow EMA period", 26, 5, 100),
            int_param!("signal", "Signal Period", "Signal line period", 9, 2, 30),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let fast = get_i64(params, "fast").ok_or("fast is required")?;
        let slow = get_i64(params, "slow").ok_or("slow is required")?;
        let signal = get_i64(params, "signal").ok_or("signal is required")?;
        if fast < 2 { return Err("fast must be >= 2".into()); }
        if fast > 50 { return Err("fast must be <= 50".into()); }
        if slow < 5 { return Err("slow must be >= 5".into()); }
        if slow > 100 { return Err("slow must be <= 100".into()); }
        if signal < 2 { return Err("signal must be >= 2".into()); }
        if signal > 30 { return Err("signal must be <= 30".into()); }
        if fast >= slow { return Err("fast must be less than slow".into()); }
        Ok(())
    }
}

// ========== 4. Bollinger ==========

impl StrategyTemplate for BollingerTemplate {
    fn id(&self) -> &str { "bollinger" }
    fn name(&self) -> &str { "Bollinger Bands" }
    fn description(&self) -> &str { "Bollinger Bands mean reversion strategy" }
    fn category(&self) -> &str { "volatility" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "std_dev": 2.0})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Bollinger Bands period", 20, 5, 100),
            float_param!("std_dev", "Std Dev", "Standard deviation multiplier", 2.0, 1.0, 4.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let std_dev = get_f64(params, "std_dev").ok_or("std_dev is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 100 { return Err("period must be <= 100".into()); }
        if std_dev < 1.0 { return Err("std_dev must be >= 1.0".into()); }
        if std_dev > 4.0 { return Err("std_dev must be <= 4.0".into()); }
        Ok(())
    }
}

// ========== 5. RSI ==========

impl StrategyTemplate for RsiTemplate {
    fn id(&self) -> &str { "rsi" }
    fn name(&self) -> &str { "RSI" }
    fn description(&self) -> &str { "Relative Strength Index mean reversion strategy" }
    fn category(&self) -> &str { "mean_reversion" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 14, "overbought": 75, "oversold": 25})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "RSI period", 14, 5, 50),
            float_param!("overbought", "Overbought", "Overbought threshold", 75.0, 65.0, 90.0),
            float_param!("oversold", "Oversold", "Oversold threshold", 25.0, 10.0, 35.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let overbought = get_f64(params, "overbought").ok_or("overbought is required")?;
        let oversold = get_f64(params, "oversold").ok_or("oversold is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 50 { return Err("period must be <= 50".into()); }
        if overbought < 65.0 { return Err("overbought must be >= 65".into()); }
        if overbought > 90.0 { return Err("overbought must be <= 90".into()); }
        if oversold < 10.0 { return Err("oversold must be >= 10".into()); }
        if oversold > 35.0 { return Err("oversold must be <= 35".into()); }
        if oversold >= overbought { return Err("oversold must be less than overbought".into()); }
        Ok(())
    }
}

// ========== 6. Keltner ==========

impl StrategyTemplate for KeltnerTemplate {
    fn id(&self) -> &str { "keltner" }
    fn name(&self) -> &str { "Keltner Channels" }
    fn description(&self) -> &str { "Keltner Channels volatility strategy" }
    fn category(&self) -> &str { "volatility" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "atr_multiplier": 1.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Keltner period", 20, 5, 100),
            float_param!("atr_multiplier", "ATR Multiplier", "ATR multiplier", 1.5, 1.0, 3.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let atr = get_f64(params, "atr_multiplier").ok_or("atr_multiplier is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 100 { return Err("period must be <= 100".into()); }
        if atr < 1.0 { return Err("atr_multiplier must be >= 1.0".into()); }
        if atr > 3.0 { return Err("atr_multiplier must be <= 3.0".into()); }
        Ok(())
    }
}

// ========== 7. ATR Stop ==========

impl StrategyTemplate for AtrStopTemplate {
    fn id(&self) -> &str { "atr_stop" }
    fn name(&self) -> &str { "ATR Stop Loss" }
    fn description(&self) -> &str { "Average True Range based stop loss strategy" }
    fn category(&self) -> &str { "volatility" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 14, "multiplier": 3.0})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "ATR period", 14, 5, 50),
            float_param!("multiplier", "Multiplier", "ATR multiplier for stop distance", 3.0, 1.0, 5.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let mult = get_f64(params, "multiplier").ok_or("multiplier is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 50 { return Err("period must be <= 50".into()); }
        if mult < 1.0 { return Err("multiplier must be >= 1.0".into()); }
        if mult > 5.0 { return Err("multiplier must be <= 5.0".into()); }
        Ok(())
    }
}

// ========== 8. Mean Reversion ==========

impl StrategyTemplate for MeanReversionTemplate {
    fn id(&self) -> &str { "mean_reversion" }
    fn name(&self) -> &str { "Mean Reversion" }
    fn description(&self) -> &str { "Statistical mean reversion strategy using standard deviation bands" }
    fn category(&self) -> &str { "mean_reversion" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "entry_std": 2.0, "exit_std": 0.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Lookback period", 20, 5, 100),
            float_param!("entry_std", "Entry Std Dev", "Entry threshold in std devs", 2.0, 1.0, 3.0),
            float_param!("exit_std", "Exit Std Dev", "Exit threshold in std devs", 0.5, 0.0, 1.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let entry = get_f64(params, "entry_std").ok_or("entry_std is required")?;
        let exit = get_f64(params, "exit_std").ok_or("exit_std is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 100 { return Err("period must be <= 100".into()); }
        if entry < 1.0 { return Err("entry_std must be >= 1.0".into()); }
        if entry > 3.0 { return Err("entry_std must be <= 3.0".into()); }
        if exit < 0.0 { return Err("exit_std must be >= 0.0".into()); }
        if exit > 1.0 { return Err("exit_std must be <= 1.0".into()); }
        if exit >= entry { return Err("exit_std must be less than entry_std".into()); }
        Ok(())
    }
}

// ========== 9. Ichimoku ==========

impl StrategyTemplate for IchimokuTemplate {
    fn id(&self) -> &str { "ichimoku" }
    fn name(&self) -> &str { "Ichimoku Cloud" }
    fn description(&self) -> &str { "Ichimoku Kinko Hyo cloud trading system" }
    fn category(&self) -> &str { "trend" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"conversion": 9, "base": 26, "span": 52, "displ": 26})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("conversion", "Conversion", "Conversion line period", 9, 5, 20),
            int_param!("base", "Base", "Base line period", 26, 10, 60),
            int_param!("span", "Span B", "Leading span B period", 52, 20, 120),
            int_param!("displ", "Displacement", "Displacement period", 26, 5, 60),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let conversion = get_i64(params, "conversion").ok_or("conversion is required")?;
        let base = get_i64(params, "base").ok_or("base is required")?;
        let span = get_i64(params, "span").ok_or("span is required")?;
        let displ = get_i64(params, "displ").ok_or("displ is required")?;
        if conversion < 5 { return Err("conversion must be >= 5".into()); }
        if conversion > 20 { return Err("conversion must be <= 20".into()); }
        if base < 10 { return Err("base must be >= 10".into()); }
        if base > 60 { return Err("base must be <= 60".into()); }
        if span < 20 { return Err("span must be >= 20".into()); }
        if span > 120 { return Err("span must be <= 120".into()); }
        if displ < 5 { return Err("displ must be >= 5".into()); }
        if displ > 60 { return Err("displ must be <= 60".into()); }
        Ok(())
    }
}

// ========== 10. Double Bollinger ==========

impl StrategyTemplate for DoubleBollingerTemplate {
    fn id(&self) -> &str { "double_bollinger" }
    fn name(&self) -> &str { "Double Bollinger Bands" }
    fn description(&self) -> &str { "Double Bollinger Bands strategy with inner and outer bands" }
    fn category(&self) -> &str { "composite" }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "inner_std": 1.5, "outer_std": 2.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Bollinger Bands period", 20, 5, 100),
            float_param!("inner_std", "Inner Std Dev", "Inner band std dev", 1.5, 1.0, 2.5),
            float_param!("outer_std", "Outer Std Dev", "Outer band std dev", 2.5, 2.0, 4.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let inner = get_f64(params, "inner_std").ok_or("inner_std is required")?;
        let outer = get_f64(params, "outer_std").ok_or("outer_std is required")?;
        if period < 5 { return Err("period must be >= 5".into()); }
        if period > 100 { return Err("period must be <= 100".into()); }
        if inner < 1.0 { return Err("inner_std must be >= 1.0".into()); }
        if inner > 2.5 { return Err("inner_std must be <= 2.5".into()); }
        if outer < 2.0 { return Err("outer_std must be >= 2.0".into()); }
        if outer > 4.0 { return Err("outer_std must be <= 4.0".into()); }
        if inner >= outer { return Err("inner_std must be less than outer_std".into()); }
        Ok(())
    }
}

// ============ Template Registry ============

pub fn get_all_templates() -> Vec<Box<dyn StrategyTemplate>> {
    vec![
        Box::new(MaCrossoverTemplate),
        Box::new(TripleMaTemplate),
        Box::new(MacdTemplate),
        Box::new(BollingerTemplate),
        Box::new(RsiTemplate),
        Box::new(KeltnerTemplate),
        Box::new(AtrStopTemplate),
        Box::new(MeanReversionTemplate),
        Box::new(IchimokuTemplate),
        Box::new(DoubleBollingerTemplate),
    ]
}

pub fn get_template(id: &str) -> Option<Box<dyn StrategyTemplate>> {
    get_all_templates().into_iter().find(|t| t.id() == id)
}

fn template_to_info(t: Box<dyn StrategyTemplate>) -> TemplateInfo {
    TemplateInfo {
        id: t.id().to_string(),
        name: t.name().to_string(),
        description: t.description().to_string(),
        category: t.category().to_string(),
        default_parameters: t.default_parameters(),
        parameter_schema: t.parameter_schema(),
    }
}

// ============ Status Validation ============

fn validate_status_transition(current: &str, next: &str) -> Result<(), AppError> {
    let valid = matches!(
        (current, next),
        ("draft", "active") | ("active", "paused") | ("paused", "active") | ("paused", "stopped")
    );
    if !valid {
        return Err(AppError::Validation(format!(
            "Invalid status transition: {} -> {}",
            current, next
        )));
    }
    Ok(())
}

fn model_to_response(m: strategy::Model) -> StrategyResponse {
    StrategyResponse {
        id: m.id,
        user_id: m.user_id,
        name: m.name,
        description: m.description,
        template_type: m.template_type,
        parameters: m.parameters,
        status: m.status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

// ============ CRUD Service Functions ============

pub async fn list_templates() -> Result<Vec<TemplateInfo>, AppError> {
    Ok(get_all_templates().into_iter().map(template_to_info).collect())
}

pub async fn create_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: CreateStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    // Validate template exists
    let template = get_template(&req.template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", req.template_type)))?;

    // Validate parameters
    template.validate(&req.parameters).map_err(|e| {
        AppError::Validation(format!("Parameter validation failed: {}", e))
    })?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation("Strategy name must be <= 100 characters".into()));
    }

    let now = chrono::Utc::now();
    let description = req.description.unwrap_or_default();
    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(req.name),
        description: Set(description),
        template_type: Set(req.template_type),
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
            return Err(AppError::Validation("Strategy name must be <= 100 characters".into()));
        }
        active.name = Set(name);
    }

    if let Some(description) = req.description {
        active.description = Set(description);
    }

    if let Some(params) = req.parameters {
        let template = get_template(&m.template_type)
            .ok_or_else(|| AppError::Internal("Template not found for existing strategy".into()))?;
        template.validate(&params).map_err(|e| {
            AppError::Validation(format!("Parameter validation failed: {}", e))
        })?;
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

    strategy::Entity::delete_by_id(strategy_id)
        .exec(db)
        .await?;

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

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

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
        // min boundary
        assert!(t.validate(&serde_json::json!({"fast_period": 5, "slow_period": 10})).is_ok());
        // max boundary
        assert!(t.validate(&serde_json::json!({"fast_period": 50, "slow_period": 200})).is_ok());
    }

    #[test]
    fn test_ma_crossover_fast_gte_slow() {
        let t = MaCrossoverTemplate;
        assert!(t.validate(&serde_json::json!({"fast_period": 30, "slow_period": 10})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 10})).is_err());
    }

    #[test]
    fn test_ma_crossover_invalid_values() {
        let t = MaCrossoverTemplate;
        assert!(t.validate(&serde_json::json!({"fast_period": 0, "slow_period": 30})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 5})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 100, "slow_period": 30})).is_err());
    }

    #[test]
    fn test_ma_crossover_missing_fields() {
        let t = MaCrossoverTemplate;
        assert!(t.validate(&serde_json::json!({})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 10})).is_err());
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
        assert!(t.validate(&serde_json::json!({"short": 10, "medium": 5, "long": 50})).is_err());
        assert!(t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 5})).is_err());
    }

    #[test]
    fn test_triple_ma_boundary() {
        let t = TripleMaTemplate;
        assert!(t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 20})).is_ok());
        assert!(t.validate(&serde_json::json!({"short": 20, "medium": 50, "long": 200})).is_ok());
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
        assert!(t.validate(&serde_json::json!({"fast": 26, "slow": 12, "signal": 9})).is_err());
        assert!(t.validate(&serde_json::json!({"fast": 12, "slow": 12, "signal": 9})).is_err());
    }

    #[test]
    fn test_macd_signal_range() {
        let t = MacdTemplate;
        assert!(t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 1})).is_err());
        assert!(t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 50})).is_err());
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
        assert!(t.validate(&serde_json::json!({"period": 20, "std_dev": 0.5})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 20, "std_dev": 5.0})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 20, "std_dev": 1.5})).is_ok());
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
        assert!(t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 30})).is_ok());
        assert!(t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 75})).is_err());
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
        assert!(t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 0.5})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 4.0})).is_err());
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
        assert!(t.validate(&serde_json::json!({"period": 14, "multiplier": 0.5})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 14, "multiplier": 6.0})).is_err());
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
        assert!(t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.5})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.0})).is_err());
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
        assert!(t.validate(&serde_json::json!({"conversion": 5, "base": 10, "span": 20, "displ": 5})).is_ok());
        assert!(t.validate(&serde_json::json!({"conversion": 20, "base": 60, "span": 120, "displ": 60})).is_ok());
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
        assert!(t.validate(&serde_json::json!({"period": 20, "inner_std": 2.5, "outer_std": 1.5})).is_err());
        assert!(t.validate(&serde_json::json!({"period": 20, "inner_std": 2.0, "outer_std": 2.0})).is_err());
    }

    // ===== All Templates Tests =====
    #[test]
    fn test_all_templates_have_unique_ids() {
        let templates = get_all_templates();
        let mut ids: Vec<String> = templates.iter().map(|t| t.id().to_string()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn test_get_template_by_id() {
        let t = get_template("ma_crossover").unwrap();
        assert_eq!(t.id(), "ma_crossover");
        assert!(get_template("nonexistent").is_none());
    }

    #[test]
    fn test_all_templates_categories_valid() {
        let valid = ["trend", "mean_reversion", "volatility", "composite"];
        for t in get_all_templates() {
            assert!(valid.contains(&t.category()), "Invalid category: {}", t.category());
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
}
