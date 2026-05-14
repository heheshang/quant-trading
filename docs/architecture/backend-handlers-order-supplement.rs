//! handlers/order.rs — 补充缺失的 handler 骨架
//!
//! 本文件包含现有 handlers/order.rs 中缺失的 handler 实现
//! 需要手动合并到 /home/ssk/workspace/quant-trading/backend/src/handlers/order.rs
//!
//! 对照: PRD-trade-execution.md §6.1-6.2, ADR-010 D6

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::order::{OrderSide, OrderStatus, OrderType, TimeInForce, TradeMode};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::matching_engine::MatchingEngine;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════
// 缺失 Handler 1: close_position
// ═══════════════════════════════════════════════════════════════

/// POST /api/v1/positions/:symbol/close — 平仓操作
///
/// PRD: US-TE-05 (平仓操作)
/// ADR: D3 (保证金解冻), D7 (加权平均均价)
///
/// 实现逻辑：
/// 1. 查找用户在 symbol 上的持仓
/// 2. 校验平仓数量 <= available_quantity
/// 3. 生成反向市价委托 (buy→sell, sell→buy)
/// 4. 委托进入撮合引擎
/// 5. 成交后：
///    - position.quantity -= filled_quantity
///    - position.realized_pnl += (close_price - avg_entry_price) * filled_quantity (long)
///    - paper_account.frozen_balance -= released
///    - paper_account.balance += released
/// 6. 若 position.quantity == 0，删除持仓记录
pub async fn close_position(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(engine): Extension<Arc<MatchingEngine>>,
    Path(symbol): Path<String>,
    Json(req): Json<ClosePositionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<OrderResponse>>), AppError> {
    // TODO: 实现平仓逻辑
    // 1. 查找持仓
    // let position = positions::Entity::find()
    //     .filter(positions::Column::UserId.eq(user.user_id))
    //     .filter(positions::Column::Symbol.eq(&symbol))
    //     .one(&*db).await?
    //     .ok_or(AppError::NotFound("持仓不存在"))?;
    //
    // 2. 解析平仓数量
    // let close_qty: f64 = req.quantity
    //     .as_deref()
    //     .unwrap_or(&format!("{:.8}", position.quantity))
    //     .parse()?;
    //
    // 3. 校验
    // if close_qty > position.available_quantity + 1e-12 {
    //     return Err(AppError::BadRequest("持仓不足".to_string()));
    // }
    //
    // 4. 生成反向市价委托
    // let reverse_side = match position.side {
    //     PositionSide::Long => OrderSide::Sell,
    //     PositionSide::Short => OrderSide::Buy,
    // };
    // ... create_order(...) ...
    //
    // 5. 返回委托
    Err(AppError::Internal("close_position: 未实现".to_string()))
}

// ═══════════════════════════════════════════════════════════════
// 缺失 Handler 2: init_account
// ═══════════════════════════════════════════════════════════════

/// POST /api/v1/account/init — 初始化模拟账户
///
/// PRD: US-TE-09 (模拟账户初始化)
/// ADR: D3 (保证金冻结)
///
/// 实现逻辑：
/// 1. 检查用户是否已有 paper_accounts 记录
/// 2. 若无，创建新记录，initial_balance = 100000 USDT
/// 3. 返回账户信息
pub async fn init_account(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<AccountResponse>>, AppError> {
    // TODO: 实现初始化逻辑
    // 1. 检查已有账户
    // let existing = paper_accounts::Entity::find()
    //     .filter(paper_accounts::Column::UserId.eq(user.user_id))
    //     .one(&*db).await?;
    //
    // 2. 若无，创建
    // if existing.is_none() {
    //     let now = chrono::Utc::now();
    //     let account_id = Uuid::new_v4();
    //     let model = paper_accounts::ActiveModel {
    //         id: Set(account_id),
    //         user_id: Set(user.user_id),
    //         balance: Set(100000.0),
    //         frozen_balance: Set(0.0),
    //         initial_balance: Set(100000.0),
    //         total_pnl: Set(0.0),
    //         created_at: Set(now),
    //         updated_at: Set(now),
    //     };
    //     model.insert(&*db).await?;
    // }
    //
    // 3. 返回账户信息 (复用 get_account 逻辑)
    Err(AppError::Internal("init_account: 未实现".to_string()))
}

// ═══════════════════════════════════════════════════════════════
// 缺失 Handler 3: get_risk_rules (P1)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct RiskRuleResponse {
    pub id: String,
    pub rule_type: String,
    pub value: String,
    pub enabled: bool,
    pub scope: String, // "global" | "user"
}

/// GET /api/v1/trade/risk/rules — 查询风控规则
///
/// PRD: US-TE-06 (风控规则可配置)
/// ADR: D8 (风控前置)
/// P1 — MVP 不实现
pub async fn list_risk_rules(
    _user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<RiskRuleResponse>>>, AppError> {
    // TODO: P1 实现
    // 1. 查询 risk_rules 表
    // 2. 合并全局规则和用户级规则（用户级覆盖全局）
    Err(AppError::Internal("list_risk_rules: P1 未实现".to_string()))
}

// ═══════════════════════════════════════════════════════════════
// 缺失 Handler 4: update_risk_rule (P1)
// ═══════════════════════════════════════════════════════════════

/// PUT /api/v1/trade/risk/rules/:id — 修改风控规则 (admin only)
///
/// PRD: US-TE-06 (风控规则可配置)
/// ADR: D8 (风控前置)
/// P1 — MVP 不实现
pub async fn update_risk_rule(
    _user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Path(_rule_id): Path<Uuid>,
    Json(_req): Json<UpdateRiskRuleRequest>,
) -> Result<Json<ApiResponse<RiskRuleResponse>>, AppError> {
    // TODO: P1 实现
    // 1. 校验 user.role == "admin"
    // 2. 更新 risk_rules 表
    // 3. 清除 risk:check:{user_id} 缓存
    Err(AppError::Internal("update_risk_rule: P1 未实现".to_string()))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRiskRuleRequest {
    pub value: Option<String>,
    pub enabled: Option<bool>,
}

// ═══════════════════════════════════════════════════════════════
// 路由注册指南 (需合并到 main.rs)
// ═══════════════════════════════════════════════════════════════
//
// 现有路由 (已注册):
//   POST   /api/v1/orders              → create_order
//   GET    /api/v1/orders              → list_orders
//   GET    /api/v1/orders/:id          → get_order
//   POST   /api/v1/orders/:id/cancel   → cancel_order
//   POST   /api/v1/orders/cancel-all   → cancel_all_orders
//   GET    /api/v1/trades              → list_trades
//   GET    /api/v1/positions           → list_positions
//   GET    /api/v1/account             → get_account
//   GET    /api/v1/symbols             → list_symbols
//
// 需新增路由:
//   POST   /api/v1/positions/:symbol/close  → close_position     (P0)
//   POST   /api/v1/account/init             → init_account       (P1)
//   GET    /api/v1/trade/ws                 → trade_ws_handler   (P0, 需 TradeWsHub)
//   GET    /api/v1/trade/risk/rules         → list_risk_rules    (P1)
//   PUT    /api/v1/trade/risk/rules/:id     → update_risk_rule   (P1)
//
// main.rs 路由注册示例:
//
// ```rust
// let app = Router::new()
//     // 现有路由...
//     .route("/api/v1/positions/:symbol/close", post(handlers::order::close_position))
//     .route("/api/v1/account/init", post(handlers::order::init_account))
//     // Trade WS (P0)
//     .route("/api/v1/trade/ws", get(handlers::ws::trade_ws_handler))
//     // 风控 (P1)
//     .route("/api/v1/trade/risk/rules", get(handlers::order::list_risk_rules))
//     .route("/api/v1/trade/risk/rules/:id", put(handlers::order::update_risk_rule))
//     .with_state(db_state)
//     .layer(Extension(matching_engine))
//     .layer(middleware::from_fn(auth_middleware));
// ```

// ═══════════════════════════════════════════════════════════════
// 关键补丁: create_order 需补充 D3 保证金冻结 + D8 风控前置
// ═══════════════════════════════════════════════════════════════
//
// 在 create_order handler 中，插入 "2. Create order in DB" 之前，需补充：
//
// ```rust
// // ── D8: 风控前置 ──
// // 2a. 余额检查 (P0 MVP 基础风控)
// let account = paper_accounts::Entity::find()
//     .filter(paper_accounts::Column::UserId.eq(user.user_id))
//     .one(&*db).await?
//     .ok_or(AppError::NotFound("模拟账户不存在"))?;
//
// if side == OrderSide::Buy {
//     let notional = price.unwrap_or(0.0) * quantity; // 市价单暂无法估算
//     if order_type == OrderType::Limit && account.balance < notional {
//         return Err(AppError::BadRequest(
//             format!("余额不足: 可用 {:.8}, 需要 {:.8}", account.balance, notional)
//         ));
//     }
// }
//
// // ── D3: 保证金冻结 ──
// // 2b. 冻结保证金 (限价买单)
// if side == OrderSide::Buy && order_type == OrderType::Limit {
//     let notional = price.unwrap() * quantity;
//     let mut acc_active: paper_accounts::ActiveModel = account.into();
//     acc_active.balance = Set(acc_active.balance.unwrap() - notional);
//     acc_active.frozen_balance = Set(acc_active.frozen_balance.unwrap() + notional);
//     acc_active.updated_at = Set(chrono::Utc::now());
//     acc_active.update(&*db).await?;
// }
// ```
//
// 在 cancel_order handler 中，状态更新之前需补充：
//
// ```rust
// // ── D3: 解冻保证金 ──
// if order.side == OrderSide::Buy && order.order_type == OrderType::Limit {
//     let unreleased_qty = order.quantity - order.filled_quantity;
//     let unreleased_notional = order.price.unwrap_or(0.0) * unreleased_qty;
//     let account = paper_accounts::Entity::find()
//         .filter(paper_accounts::Column::UserId.eq(user.user_id))
//         .one(&*db).await?
//         .ok_or(AppError::NotFound("模拟账户不存在"))?;
//     let mut acc_active: paper_accounts::ActiveModel = account.into();
//     acc_active.frozen_balance = Set(acc_active.frozen_balance.unwrap() - unreleased_notional);
//     acc_active.balance = Set(acc_active.balance.unwrap() + unreleased_notional);
//     acc_active.updated_at = Set(chrono::Utc::now());
//     acc_active.update(&*db).await?;
// }
// ```

// ═══════════════════════════════════════════════════════════════
// 关键补丁: cancel_order 需补充 D2 PG 行锁
// ═══════════════════════════════════════════════════════════════
//
// 将 cancel_order 中的 find_by_id 替换为行锁查询：
//
// ```rust
// use sea_orm::LockType;
//
// let order = Entity::find_by_id(order_id)
//     .lock(LockType::Update)   // SELECT ... FOR UPDATE
//     .one(&*db)
//     .await?
//     .ok_or(AppError::NotFound(...))?;
// ```
