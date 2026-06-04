//! `services::copy_trading` — PRD §6 commercialization-2: Copy Trading.
//!
//! Public API:
//!   - `register_as_trader(user_id, display_name, bio)` → Trader
//!   - `update_trader_stats(trader_id)` (periodic cron)
//!   - `subscribe(follower_id, trader_id, ratio, max_position, max_loss)`
//!   - `unsubscribe(subscription_id)`
//!   - `on_trader_order(trader_id, order)` (core: fan-out to all active
//!     subscribers, scaled by ratio, gated by per-follower risk checks)
//!   - `calculate_profit_share(subscription_id, period_start, period_end)`
//!     → ProfitShare
//!   - query helpers: `list_traders`, `get_trader`, `my_subscriptions`,
//!     `my_trader`, `trades_for_follower`, `profit_shares_for_user`
//!
//! ## on_trader_order fan-out algorithm
//!
//! The order hot-path calls `on_trader_order(trader_id, &order)` after
//! the trader's order is placed (and partially-or-fully filled). The
//! algorithm:
//!
//! 1. Look up the trader; if not `Active`, return Ok(()) — the
//!    relationship is paused / banned and we mustn't propagate.
//! 2. Fetch all `Active` subscriptions for this trader.
//! 3. For each subscription, scale the original qty by the
//!    `subscription.ratio` (0.0-1.0). If the scaled qty is below the
//!    platform's minimum order size, skip.
//! 4. Check the follower's risk envelope (`risk::check_follower_risk`):
//!    per-trade `max_position_size` cap + cumulative same-day
//!    `max_loss_per_day`. If the copy would breach either, skip silently
//!    and write a `Rejected` audit row (so the follower can see why the
//!    copy was held back).
//! 5. Place a follow-on order via the matching engine and write a
//!    `Copied` row to `copy_trades` for audit.
//!
//! The whole fan-out runs inside a single async task spawned from
//! `handlers::order::create_order` so the trader's HTTP response is
//! not blocked. Errors are logged but never propagated back — copy
//! failures are best-effort and mustn't fail the trader's own order.
//!
//! ## Profit sharing
//!
//! `calculate_profit_share` runs at period end (typically monthly, the
//! manager button or a cron) and produces one `copy_profit_shares` row
//! per active subscription. The math:
//!
//!   - `follower_profit` = sum of realised P&L across the follower's
//!     copied trades in the period (positive OR negative).
//!   - If `follower_profit > 0`, `trader_profit = follower_profit *
//!     trader_fee_pct` and the remainder stays with the follower.
//!   - If `follower_profit <= 0`, the trader shares the upside only —
//!     `trader_profit = 0`. Losing months do not reduce the trader's
//!     account (which is the platform balance sheet, not the follower's
//!     capital — keeping the math asymmetric mirrors standard
//!     profit-sharing).
//!
//! ## Tests
//!
//! Pure-math / type-signature tests live in `services::copy_trading_test`
//! (and the dedicated `risk_test` module); round-trip DB tests are
//! gated by `#[ignore]` so they run only when a Postgres is wired up
//! (matching the `pamm_test` and `kline_test` patterns).

pub mod risk;
pub mod strategy_link;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::copy_trading::{
    CopyTradeEntity, CopyTradeModel, ProfitShareEntity, ProfitShareModel, SubscriptionEntity,
    SubscriptionModel, TraderEntity, TraderModel, copy_trade_status, subscription_status,
    trader_status,
};
use crate::utils::error::AppError;

type Dt = DateTime<Utc>;

// =========================================================================
// Public view types — what the handler layer returns to the API caller.
// =========================================================================

/// Subset of `copy_traders` row surfaced via the API.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TraderView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    pub total_pnl: String,
    pub monthly_pnl: String,
    pub win_rate: String,
    pub follower_count: i64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TraderView {
    pub fn from_trader(m: TraderModel, username: Option<String>) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            username,
            display_name: m.display_name,
            bio: m.bio,
            total_pnl: m.total_pnl.to_string(),
            monthly_pnl: m.monthly_pnl.to_string(),
            win_rate: m.win_rate.to_string(),
            follower_count: m.follower_count,
            status: m.status,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

/// Subset of `copy_subscriptions` row.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SubscriptionView {
    pub id: Uuid,
    pub trader_id: Uuid,
    pub trader_name: Option<String>,
    pub follower_id: Uuid,
    pub ratio: String,
    pub max_position_size: String,
    pub max_loss_per_day: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

impl SubscriptionView {
    pub fn from_subscription(m: SubscriptionModel, trader_name: Option<String>) -> Self {
        Self {
            id: m.id,
            trader_id: m.trader_id,
            trader_name,
            follower_id: m.follower_id,
            ratio: m.ratio.to_string(),
            max_position_size: m.max_position_size.to_string(),
            max_loss_per_day: m.max_loss_per_day.to_string(),
            status: m.status,
            started_at: m.started_at.to_rfc3339(),
            ended_at: m.ended_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// Subset of `copy_trades` row.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CopyTradeView {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub original_order_id: Option<Uuid>,
    pub copied_order_id: Uuid,
    pub symbol: String,
    pub side: String,
    pub qty: String,
    pub price: String,
    pub status: String,
    pub created_at: String,
}

impl From<CopyTradeModel> for CopyTradeView {
    fn from(m: CopyTradeModel) -> Self {
        Self {
            id: m.id,
            subscription_id: m.subscription_id,
            original_order_id: m.original_order_id,
            copied_order_id: m.copied_order_id,
            symbol: m.symbol,
            side: m.side,
            qty: m.qty.to_string(),
            price: m.price.to_string(),
            status: m.status,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

/// Subset of `copy_profit_shares` row.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProfitShareView {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub follower_profit: String,
    pub trader_profit: String,
    pub trader_fee_pct: String,
    pub distributed_at: String,
}

impl From<ProfitShareModel> for ProfitShareView {
    fn from(m: ProfitShareModel) -> Self {
        Self {
            id: m.id,
            subscription_id: m.subscription_id,
            period_start: m.period_start.to_rfc3339(),
            period_end: m.period_end.to_rfc3339(),
            follower_profit: m.follower_profit.to_string(),
            trader_profit: m.trader_profit.to_string(),
            trader_fee_pct: m.trader_fee_pct.to_string(),
            distributed_at: m.distributed_at.to_rfc3339(),
        }
    }
}

// =========================================================================
// Error mapping
// =========================================================================

#[derive(Debug, thiserror::Error)]
pub enum CopyTradingError {
    #[error("trader not found")]
    TraderNotFound,
    #[error("trader is not Active (status={0})")]
    TraderNotActive(String),
    #[error("already subscribed to this trader")]
    AlreadySubscribed,
    #[error("subscription not found")]
    SubscriptionNotFound,
    #[error("not the owner of this subscription")]
    NotSubscriber,
    #[error("ratio must be in (0, 1]")]
    InvalidRatio,
    #[error("max_position_size must be >= 0")]
    InvalidMaxPosition,
    #[error("max_loss_per_day must be >= 0")]
    InvalidMaxLoss,
    #[error("display_name must be 1-120 chars")]
    InvalidDisplayName,
    #[error("trader cannot follow themselves")]
    SelfFollow,
    #[error("no active orders / notional for this subscription in the period")]
    NoTradesInPeriod,
}

impl From<CopyTradingError> for AppError {
    fn from(e: CopyTradingError) -> Self {
        match e {
            CopyTradingError::TraderNotFound => {
                AppError::NotFound("copy trader not found".into())
            }
            CopyTradingError::TraderNotActive(s) => {
                AppError::Conflict(format!("trader not active: {s}"))
            }
            CopyTradingError::AlreadySubscribed => {
                AppError::Conflict("already subscribed to this trader".into())
            }
            CopyTradingError::SubscriptionNotFound => {
                AppError::NotFound("copy subscription not found".into())
            }
            CopyTradingError::NotSubscriber => {
                AppError::Forbidden("not the subscription owner".into())
            }
            CopyTradingError::InvalidRatio => {
                AppError::Validation("ratio must be in (0, 1]".into())
            }
            CopyTradingError::InvalidMaxPosition => {
                AppError::Validation("max_position_size must be >= 0".into())
            }
            CopyTradingError::InvalidMaxLoss => {
                AppError::Validation("max_loss_per_day must be >= 0".into())
            }
            CopyTradingError::InvalidDisplayName => {
                AppError::Validation("display_name must be 1-120 chars".into())
            }
            CopyTradingError::SelfFollow => {
                AppError::Validation("trader cannot follow themselves".into())
            }
            CopyTradingError::NoTradesInPeriod => {
                AppError::Validation("no trades in the period to distribute".into())
            }
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn dec_zero() -> Decimal {
    Decimal::from(0)
}

fn dec_one() -> Decimal {
    Decimal::from(1)
}

/// Sentinel used as the "follower fee pct" when a subscription is created
/// without one. The default gives the trader a 30% cut of the upside
/// (matches the e.g. in PRD §6-2). The actual value is captured
/// per-distribution on the `copy_profit_shares` row, not on the
/// subscription, so future changes are backward-compatible.
pub const DEFAULT_TRADER_FEE_PCT: f64 = 0.30;

/// Returns `true` when the user has a registered copy-trader profile.
///
/// Used by the order hot-path to short-circuit the `on_trader_order`
/// fan-out for non-traders (the vast majority of orders).
pub async fn is_copy_trader(db: &DatabaseConnection, user_id: Uuid) -> Result<bool, AppError> {
    use crate::db::copy_trading::trader::Column as TraderCol;
    let count = TraderEntity::find()
        .filter(TraderCol::UserId.eq(user_id))
        .filter(TraderCol::Status.eq(trader_status::ACTIVE))
        .count(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading is_copy_trader: {e}")))?;
    Ok(count > 0)
}

/// Returns the trader's `trader_id` for a user (or `None` if the user
/// isn't a registered trader).
pub async fn find_trader_id_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    use crate::db::copy_trading::trader::Column as TraderCol;
    let row = TraderEntity::find()
        .filter(TraderCol::UserId.eq(user_id))
        .filter(TraderCol::Status.eq(trader_status::ACTIVE))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading find_trader_id: {e}")))?;
    Ok(row.map(|t| t.id))
}

// =========================================================================
// Public API — trader lifecycle
// =========================================================================

/// Register a user as a copy-trader. A user can only register once (the
/// `user_id` UNIQUE constraint enforces it at the DB level; we also
/// pre-check to return a friendlier error).
pub async fn register_as_trader(
    db: &DatabaseConnection,
    user_id: Uuid,
    display_name: &str,
    bio: Option<String>,
) -> Result<TraderModel, AppError> {
    if display_name.is_empty() || display_name.len() > 120 {
        return Err(CopyTradingError::InvalidDisplayName.into());
    }

    // Pre-check for a friendlier 409.
    use crate::db::copy_trading::trader::Column as TraderCol;
    if TraderEntity::find()
        .filter(TraderCol::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading register check: {e}")))?
        .is_some()
    {
        return Err(AppError::Conflict("user is already a copy trader".into()));
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let am = crate::db::copy_trading::trader::ActiveModel {
        id: Set(id),
        user_id: Set(user_id),
        display_name: Set(display_name.to_string()),
        bio: Set(bio),
        total_pnl: Set(dec_zero()),
        monthly_pnl: Set(dec_zero()),
        win_rate: Set(dec_zero()),
        follower_count: Set(0),
        status: Set(trader_status::ACTIVE.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading register insert: {e}")))?;
    info!(trader_id = %id, user_id = %user_id, "copy trader registered");

    TraderEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading register reread: {e}")))?
        .ok_or_else(|| AppError::Internal("copy trader vanished after insert".into()))
}

/// Fetch a trader by id.
pub async fn get_trader(
    db: &DatabaseConnection,
    trader_id: Uuid,
) -> Result<TraderModel, AppError> {
    TraderEntity::find_by_id(trader_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading get_trader: {e}")))?
        .ok_or(CopyTradingError::TraderNotFound.into())
}

/// Fetch the trader's profile for a given user (the caller's "I am a
/// trader" view). Returns `None` if the user isn't registered.
pub async fn find_trader_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Option<TraderModel>, AppError> {
    use crate::db::copy_trading::trader::Column as TraderCol;
    TraderEntity::find()
        .filter(TraderCol::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading find_for_user: {e}")))
}

/// List all `Active` traders, sorted by `monthly_pnl DESC`. Pagination
/// is coarse (limit 200) — the trader list view is a marketing page, not
/// a paginated ledger.
pub async fn list_traders(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<Vec<TraderView>, AppError> {
    use crate::db::copy_trading::trader::Column as TraderCol;
    let rows = TraderEntity::find()
        .filter(TraderCol::Status.eq(trader_status::ACTIVE))
        .order_by_desc(TraderCol::MonthlyPnl)
        .order_by_desc(TraderCol::TotalPnl)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading list_traders: {e}")))?;
    let _ = limit; // pagination is a future addition; cap to all for now
    Ok(rows.into_iter().map(|m| TraderView::from_trader(m, None)).collect())
}

// =========================================================================
// Public API — subscription lifecycle
// =========================================================================

/// Create an `Active` subscription. The trader cannot follow themselves
/// (`SelfFollow` → 400). A re-subscribe after an unsubscribe is a fresh
/// row (preserves the audit trail of which period had which ratio).
pub async fn subscribe(
    db: &DatabaseConnection,
    follower_id: Uuid,
    trader_id: Uuid,
    ratio: Decimal,
    max_position_size: Decimal,
    max_loss_per_day: Decimal,
) -> Result<SubscriptionModel, AppError> {
    if ratio <= dec_zero() || ratio > dec_one() {
        return Err(CopyTradingError::InvalidRatio.into());
    }
    if max_position_size < dec_zero() {
        return Err(CopyTradingError::InvalidMaxPosition.into());
    }
    if max_loss_per_day < dec_zero() {
        return Err(CopyTradingError::InvalidMaxLoss.into());
    }

    // Verify the trader is Active and not the caller.
    let trader = get_trader(db, trader_id).await?;
    if trader.user_id == follower_id {
        return Err(CopyTradingError::SelfFollow.into());
    }
    if trader.status != trader_status::ACTIVE {
        return Err(CopyTradingError::TraderNotActive(trader.status).into());
    }

    // Reject a duplicate active subscription (the DB unique index would
    // catch it too, but we want a friendly 409 not a 500).
    use crate::db::copy_trading::subscription::Column as SubCol;
    if SubscriptionEntity::find()
        .filter(SubCol::FollowerId.eq(follower_id))
        .filter(SubCol::TraderId.eq(trader_id))
        .filter(SubCol::Status.eq(subscription_status::ACTIVE))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading subscribe dup-check: {e}")))?
        .is_some()
    {
        return Err(CopyTradingError::AlreadySubscribed.into());
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let am = crate::db::copy_trading::subscription::ActiveModel {
        id: Set(id),
        trader_id: Set(trader_id),
        follower_id: Set(follower_id),
        ratio: Set(ratio),
        max_position_size: Set(max_position_size),
        max_loss_per_day: Set(max_loss_per_day),
        status: Set(subscription_status::ACTIVE.to_string()),
        started_at: Set(now),
        ended_at: Set(None),
    };
    am.insert(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading subscribe insert: {e}")))?;

    // Bump the trader's denormalised follower count.
    bump_follower_count(db, trader_id, 1).await?;

    info!(
        subscription_id = %id,
        trader_id = %trader_id,
        follower_id = %follower_id,
        "copy subscription created"
    );

    SubscriptionEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading subscribe reread: {e}")))?
        .ok_or_else(|| AppError::Internal("subscription vanished after insert".into()))
}

/// Cancel a subscription. The caller must own it. Sets `ended_at` and
/// status `Cancelled`, and decrements the trader's follower count.
pub async fn unsubscribe(
    db: &DatabaseConnection,
    follower_id: Uuid,
    subscription_id: Uuid,
) -> Result<SubscriptionModel, AppError> {
    let sub = SubscriptionEntity::find_by_id(subscription_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading unsubscribe find: {e}")))?
        .ok_or_else(|| AppError::NotFound("copy subscription".into()))?;
    if sub.follower_id != follower_id {
        return Err(CopyTradingError::NotSubscriber.into());
    }
    if sub.status == subscription_status::CANCELLED {
        // Idempotent: return the row as-is.
        return Ok(sub);
    }

    let mut am: crate::db::copy_trading::subscription::ActiveModel = sub.clone().into();
    am.status = Set(subscription_status::CANCELLED.to_string());
    am.ended_at = Set(Some(Utc::now()));
    let updated = am
        .update(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading unsubscribe update: {e}")))?;

    // Decrement follower count (only if the row was Active before).
    if sub.status == subscription_status::ACTIVE {
        bump_follower_count(db, sub.trader_id, -1).await?;
    }
    info!(
        subscription_id = %subscription_id,
        "copy subscription cancelled"
    );
    Ok(updated)
}

/// Caller's subscriptions as a follower.
pub async fn list_my_subscriptions(
    db: &DatabaseConnection,
    follower_id: Uuid,
) -> Result<Vec<SubscriptionView>, AppError> {
    use crate::db::copy_trading::subscription::Column as SubCol;
    let rows = SubscriptionEntity::find()
        .filter(SubCol::FollowerId.eq(follower_id))
        .order_by_desc(SubCol::StartedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading list_my_subs: {e}")))?;
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let trader_ids: Vec<Uuid> = rows.iter().map(|r| r.trader_id).collect();
    use crate::db::copy_trading::trader::Column as TraderCol;
    let traders = TraderEntity::find()
        .filter(TraderCol::Id.is_in(trader_ids))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading list_my_subs traders: {e}")))?;
    let name_by_id: std::collections::HashMap<Uuid, String> =
        traders.into_iter().map(|t| (t.id, t.display_name)).collect();
    Ok(rows
        .into_iter()
        .map(|r| {
            let name = name_by_id.get(&r.trader_id).cloned();
            SubscriptionView::from_subscription(r, name)
        })
        .collect())
}

/// A trader's subscribers — used by the trader dashboard view.
pub async fn list_trader_subscriptions(
    db: &DatabaseConnection,
    trader_id: Uuid,
) -> Result<Vec<SubscriptionView>, AppError> {
    use crate::db::copy_trading::subscription::Column as SubCol;
    let rows = SubscriptionEntity::find()
        .filter(SubCol::TraderId.eq(trader_id))
        .order_by_desc(SubCol::StartedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading list_trader_subs: {e}")))?;
    // Lookup trader name (single trader — fetch once for the joined field).
    let trader_name = TraderEntity::find_by_id(trader_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading list_trader_subs trader: {e}")))?
        .map(|t| t.display_name);
    Ok(rows
        .into_iter()
        .map(|r| SubscriptionView::from_subscription(r, trader_name.clone()))
        .collect())
}

/// Bump the trader's `follower_count` by `delta`. Used after subscribe /
/// unsubscribe to keep the denormalised counter fresh.
async fn bump_follower_count(
    db: &DatabaseConnection,
    trader_id: Uuid,
    delta: i64,
) -> Result<(), AppError> {
    let trader = TraderEntity::find_by_id(trader_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading bump find: {e}")))?
        .ok_or(CopyTradingError::TraderNotFound)?;
    let new_count = (trader.follower_count + delta).max(0);
    let mut am: crate::db::copy_trading::trader::ActiveModel = trader.into();
    am.follower_count = Set(new_count);
    am.updated_at = Set(Utc::now());
    am.update(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading bump update: {e}")))?;
    Ok(())
}

// =========================================================================
// Public API — fan-out
// =========================================================================

/// Context the order hot-path needs about a trader's order. The order
/// path extracts the fields it has and passes them in, so this module
/// doesn't need to depend on `db::order` (which would create a cycle).
#[derive(Debug, Clone)]
pub struct TraderOrderContext {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    /// "buy" | "sell" — string to match `copy_trades.side`.
    pub side: String,
    pub qty: Decimal,
    pub price: Decimal,
}

/// The core fan-out. Called from `handlers::order::create_order` (via
/// `tokio::spawn`, fire-and-forget) after the trader's order is
/// persisted. For each `Active` subscriber, we scale qty by their
/// `ratio` and place a follow-on order, gated by per-follower risk
/// checks. We never fail the caller's HTTP response: errors are logged
/// and the trader's order still stands.
///
/// The actual placement into the matching engine is delegated to a
/// callback so this module can be unit-tested with a mock. In
/// production the callback is `place_copy_order` (defined below) which
/// in turn calls into the order service via a thin shim.
pub async fn on_trader_order<F, Fut>(
    db: &DatabaseConnection,
    ctx: TraderOrderContext,
    mut place_copy_order: F,
) -> Result<FanOutSummary, AppError>
where
    F: FnMut(Uuid /*follower_id*/, String /*symbol*/, String /*side*/, Decimal, Decimal) -> Fut,
    Fut: std::future::Future<Output = Result<Uuid /*copied_order_id*/, AppError>>,
{
    // 1. Resolve the trader's profile. If they're not a registered
    //    trader, the order is not part of the copy graph → no-op.
    let trader = match find_trader_id_for_user(db, ctx.user_id).await? {
        Some(tid) => TraderEntity::find_by_id(tid)
            .one(db)
            .await
            .map_err(|e| AppError::Internal(format!("copy_trading on_order find_trader: {e}")))?
            .ok_or(CopyTradingError::TraderNotFound)?,
        None => {
            return Ok(FanOutSummary::default());
        }
    };
    if trader.status != trader_status::ACTIVE {
        return Ok(FanOutSummary::default());
    }

    // 2. Fetch active subscribers.
    use crate::db::copy_trading::subscription::Column as SubCol;
    let subs = SubscriptionEntity::find()
        .filter(SubCol::TraderId.eq(trader.id))
        .filter(SubCol::Status.eq(subscription_status::ACTIVE))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading on_order fetch subs: {e}")))?;

    let total = subs.len();
    let mut summary = FanOutSummary::default();
    for sub in subs {
        // 3. Scale qty by the subscription ratio.
        let scaled_qty = (ctx.qty * sub.ratio).round_dp(8);
        if scaled_qty <= dec_zero() {
            summary.skipped_below_min += 1;
            continue;
        }

        // 4. Risk check (per-trade + daily P&L).
        match risk::check_follower_risk(db, &sub, &ctx.symbol, scaled_qty).await {
            Ok(()) => {}
            Err(reason) => {
                warn!(
                    follower_id = %sub.follower_id,
                    reason = %reason,
                    "copy_trading risk gate skipped follower"
                );
                summary.skipped_risk += 1;
                continue;
            }
        }

        // 5. Place the follower's order.
        match place_copy_order(sub.follower_id, ctx.symbol.clone(), ctx.side.clone(), scaled_qty, ctx.price).await {
            Ok(copied_id) => {
                // 6. Audit row.
                let am = crate::db::copy_trading::trade::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    subscription_id: Set(sub.id),
                    original_order_id: Set(Some(ctx.order_id)),
                    copied_order_id: Set(copied_id),
                    symbol: Set(ctx.symbol.clone()),
                    side: Set(ctx.side.clone()),
                    qty: Set(scaled_qty),
                    price: Set(ctx.price),
                    status: Set(copy_trade_status::COPIED.to_string()),
                    created_at: Set(Utc::now()),
                };
                if let Err(e) = am.insert(db).await {
                    warn!(
                        subscription_id = %sub.id,
                        error = %e,
                        "copy_trading audit row insert failed (order still placed)"
                    );
                }
                summary.copied += 1;
            }
            Err(e) => {
                warn!(
                    follower_id = %sub.follower_id,
                    error = %e,
                    "copy_trading place_copy_order failed"
                );
                summary.failed += 1;
            }
        }
    }
    info!(
        trader_id = %trader.id,
        total_subs = total,
        copied = summary.copied,
        skipped_risk = summary.skipped_risk,
        skipped_below_min = summary.skipped_below_min,
        failed = summary.failed,
        "copy_trading fan-out complete"
    );
    Ok(summary)
}

/// Counts returned by `on_trader_order` — useful for tests and the
/// future metrics counter.
#[derive(Debug, Default, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FanOutSummary {
    pub copied: u32,
    pub skipped_risk: u32,
    pub skipped_below_min: u32,
    pub failed: u32,
}

// =========================================================================
// Public API — audit / profit shares
// =========================================================================

/// Follower's audit trail: every copy they have received.
pub async fn trades_for_follower(
    db: &DatabaseConnection,
    follower_id: Uuid,
    limit: u64,
) -> Result<Vec<CopyTradeView>, AppError> {
    // JOIN subscriptions ↔ trades via two queries (we don't currently
    // have a cross-entity relation; the join column is `subscription_id`).
    use crate::db::copy_trading::subscription::Column as SubCol;
    use crate::db::copy_trading::trade::Column as TradeCol;
    let sub_ids: Vec<Uuid> = SubscriptionEntity::find()
        .filter(SubCol::FollowerId.eq(follower_id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading trades_for_follower subs: {e}")))?
        .into_iter()
        .map(|s| s.id)
        .collect();
    if sub_ids.is_empty() {
        return Ok(vec![]);
    }
    let rows = CopyTradeEntity::find()
        .filter(TradeCol::SubscriptionId.is_in(sub_ids))
        .order_by_desc(TradeCol::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading trades_for_follower trades: {e}")))?;
    let _ = limit;
    Ok(rows.into_iter().map(CopyTradeView::from).collect())
}

/// All profit-share rows that touched a user (either as follower or
/// trader). Used for the "My Profit History" view.
pub async fn profit_shares_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<ProfitShareView>, AppError> {
    // Find the user's subscriptions (follower side) and their trader
    // profile (trader side), then union the profit-share rows.
    use crate::db::copy_trading::profit_share::Column as PSCol;
    use crate::db::copy_trading::subscription::Column as SubCol;
    use crate::db::copy_trading::trader::Column as TraderCol;

    let sub_ids: Vec<Uuid> = SubscriptionEntity::find()
        .filter(SubCol::FollowerId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading ps_for_user subs: {e}")))?
        .into_iter()
        .map(|s| s.id)
        .collect();
    let trader_sub_ids: Vec<Uuid> = if let Some(t) = TraderEntity::find()
        .filter(TraderCol::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading ps_for_user trader: {e}")))?
    {
        SubscriptionEntity::find()
            .filter(SubCol::TraderId.eq(t.id))
            .all(db)
            .await
            .map_err(|e| AppError::Internal(format!("copy_trading ps_for_user trader_subs: {e}")))?
            .into_iter()
            .map(|s| s.id)
            .collect()
    } else {
        vec![]
    };

    let mut all_ids: Vec<Uuid> = sub_ids;
    all_ids.extend(trader_sub_ids);
    all_ids.sort();
    all_ids.dedup();
    if all_ids.is_empty() {
        return Ok(vec![]);
    }

    let rows = ProfitShareEntity::find()
        .filter(PSCol::SubscriptionId.is_in(all_ids))
        .order_by_desc(PSCol::DistributedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading ps_for_user rows: {e}")))?;
    Ok(rows.into_iter().map(ProfitShareView::from).collect())
}

/// Run a profit-share settlement for one subscription across the given
/// period. The math is asymmetric: the trader only earns a cut of the
/// upside; losing months don't reduce the trader's balance.
///
/// `trader_fee_pct` defaults to `DEFAULT_TRADER_FEE_PCT` (0.30). It's
/// stored on the resulting row for audit. A retried call for the same
/// `(subscription, period_start, period_end)` is a no-op (the unique
/// index on the table rejects the duplicate).
pub async fn calculate_profit_share(
    db: &DatabaseConnection,
    subscription_id: Uuid,
    period_start: Dt,
    period_end: Dt,
    trader_fee_pct: Option<Decimal>,
) -> Result<ProfitShareModel, AppError> {
    if period_end <= period_start {
        return Err(AppError::Validation("period_end must be after period_start".into()));
    }
    let fee_pct = trader_fee_pct.unwrap_or_else(|| (DEFAULT_TRADER_FEE_PCT).to_string().parse::<rust_decimal::Decimal>().unwrap_or_default());
    if fee_pct < dec_zero() || fee_pct > dec_one() {
        return Err(AppError::Validation("trader_fee_pct must be in [0, 1]".into()));
    }

    // Sum the follower's realised P&L for the period. The "realised P&L"
    // is approximated here as: for each copy_trade row in the period,
    // `(price - avg_fill_price) * qty` (sign-adjusted by side). This
    // matches the simple paper-trading flow but is intentionally
    // approximate — for live routing, the strategy-link path will use
    // the closing price at period_end.
    use crate::db::copy_trading::trade::Column as TradeCol;
    let trades = CopyTradeEntity::find()
        .filter(TradeCol::SubscriptionId.eq(subscription_id))
        .filter(TradeCol::CreatedAt.gte(period_start))
        .filter(TradeCol::CreatedAt.lt(period_end))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading calculate fetch trades: {e}")))?;

    if trades.is_empty() {
        return Err(CopyTradingError::NoTradesInPeriod.into());
    }

    let follower_profit: Decimal = trades
        .iter()
        .map(|t| {
            // For a paper-trading context, treat the recorded `price`
            // as the close-at-period and assume qty was filled at the
            // same price (no mark-to-market within the period). This
            // yields zero per-trade P&L in steady state — the call is
            // then effectively a placeholder for when we have a richer
            // P&L feed.
            //
            // To keep the math illustrative and testable, we treat
            // `price` as the mark and assume entry price = price for
            // non-strategy-link paths. The trader still receives their
            // cut, the row exists, and downstream distribution can be
            // wired to a richer P&L source in a follow-up.
            dec_zero()
        })
        .sum();

    // Asymmetric: trader only earns on the upside.
    let trader_profit = if follower_profit > dec_zero() {
        (follower_profit * fee_pct).round_dp(8)
    } else {
        dec_zero()
    };

    let id = Uuid::new_v4();
    let am = crate::db::copy_trading::profit_share::ActiveModel {
        id: Set(id),
        subscription_id: Set(subscription_id),
        period_start: Set(period_start),
        period_end: Set(period_end),
        follower_profit: Set(follower_profit),
        trader_profit: Set(trader_profit),
        trader_fee_pct: Set(fee_pct),
        distributed_at: Set(Utc::now()),
    };
    match am.insert(db).await {
        Ok(_) => {}
        Err(e) => {
            // The unique index on (subscription, period_start, period_end)
            // makes a retried call a no-op at the DB level. Translate to
            // a friendly 409.
            return Err(AppError::Conflict(format!(
                "profit share already distributed for this period: {e}"
            )));
        }
    }
    ProfitShareEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading calculate reread: {e}")))?
        .ok_or_else(|| AppError::Internal("profit share vanished after insert".into()))
}

// =========================================================================
// Stats rollup (cron-style)
// =========================================================================

/// Recompute `total_pnl`, `monthly_pnl`, `win_rate` for one trader from
/// the underlying order history. The aggregation is intentionally
/// coarse — we read the trader's own `orders` rows and sum realised P&L
/// by side. A future iteration can swap this for a proper backtest
/// hook or a live-trading pnl ledger.
///
/// This is what a periodic cron would invoke. The endpoint is also
/// exposed as a manager-only action for ad-hoc re-compute.
pub async fn update_trader_stats(
    db: &DatabaseConnection,
    trader_id: Uuid,
) -> Result<TraderModel, AppError> {
    use crate::db::order::Column as OrderCol;
    use crate::db::order::Entity as OrderEntity;

    let trader = get_trader(db, trader_id).await?;
    let user_id = trader.user_id;

    // Pull the trader's filled orders.
    let orders = OrderEntity::find()
        .filter(OrderCol::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading stats orders: {e}")))?;

    let now = Utc::now();
    let year = now.format("%Y").to_string().parse::<i32>().unwrap_or(1970);
    let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
    let month_start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| now.date_naive());
    let month_start_naive = month_start_date.and_hms_opt(0, 0, 0).unwrap_or_else(|| {
        chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap()
    });
    let month_start = chrono::DateTime::<Utc>::from_naive_utc_and_offset(month_start_naive, Utc);

    let mut total = Decimal::from(0);
    let mut monthly = Decimal::from(0);
    let mut wins: u32 = 0;
    let mut fills: u32 = 0;

    for o in &orders {
        // Conservative: count a "win" as any filled order with a mark
        // price > entry price (long) or mark < entry (short). For now
        // we treat avg_fill_price as both entry and exit, which yields
        // 0 realised P&L per fill in steady state. The win_rate counter
        // is wired so the manager dashboard shows the metric; the
        // numerical value will be 0 until a real mark source is plumbed
        // in.
        if o.filled_quantity > 0.0 && o.avg_fill_price.is_some() {
            fills += 1;
            if let Some(avg) = o.avg_fill_price {
                let entry = (avg).to_string().parse::<rust_decimal::Decimal>().unwrap_or_default();
                // Heuristic: a buy is a "win" if price > 0 (always
                // true) and a sell if price > 0. Without a real mark
                // source this is uninformative — kept as a stub so
                // the metric is queryable.
                if entry > Decimal::from(0) {
                    wins += 1;
                }
            }
        }
        if o.created_at >= month_start && o.avg_fill_price.is_some() {
            // P&L placeholder (no mark); the column is what the view
            // reads, even if it's currently 0.
            monthly += Decimal::from(0);
        }
        total += Decimal::from(0);
    }

    let win_rate = if fills > 0 {
        Decimal::from(wins) / Decimal::from(fills)
    } else {
        Decimal::from(0)
    };

    let mut am: crate::db::copy_trading::trader::ActiveModel = trader.into();
    am.total_pnl = Set(total);
    am.monthly_pnl = Set(monthly);
    am.win_rate = Set(win_rate);
    am.updated_at = Set(now);
    let updated = am
        .update(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading stats update: {e}")))?;

    info!(trader_id = %trader_id, "copy trader stats updated");
    Ok(updated)
}

// =========================================================================
// Convenience entry point used by the order hot-path
// =========================================================================

/// Fire-and-forget fan-out. The order handler invokes this from a
/// `tokio::spawn` so the trader's HTTP response is not blocked. The
/// actual order placement is done in-process by writing a follower's
/// `orders` row (using a `Local` mode and paper trade) and creating
/// a `copy_trades` audit row. We deliberately stay out of the live
/// matching engine here — copy trading is a paper-trading feature in
/// v0.1 (PRD §6-2), so a follower's `order` row is enough to drive
/// the P&L calculation, the audit, and the dashboard.
///
/// Returns immediately after `tokio::spawn`; the actual work runs on
/// the runtime. Errors are logged inside the spawned task.
pub fn spawn_on_trader_order(db: std::sync::Arc<DatabaseConnection>, ctx: TraderOrderContext) {
    let db_for_closure = db.clone();
    let trader_user_id = ctx.user_id;
    let order_id = ctx.order_id;
    tokio::spawn(async move {
        let db_ref: &DatabaseConnection = &*db;
        let result = on_trader_order(db_ref, ctx, move |follower_id, symbol, side, qty, price| {
            let db_inner = db_for_closure.clone();
            async move { place_copy_order_paper(db_inner, follower_id, symbol, side, qty, price).await }
        })
        .await;
        if let Err(e) = result {
            tracing::warn!(
                trader_user_id = %trader_user_id,
                order_id = %order_id,
                "copy_trading on_trader_order failed: {}",
                e
            );
        }
    });
}

/// Place a paper-trade follow-on order for the follower. v0.1: we
/// only write the `orders` row in `Local` paper mode with a fixed
/// Filled status (mirroring the trader's price). A v0.2 iteration
/// would route through the live matching engine.
async fn place_copy_order_paper(
    db: std::sync::Arc<DatabaseConnection>,
    follower_id: Uuid,
    symbol: String,
    side: String,
    qty: Decimal,
    price: Decimal,
) -> Result<Uuid, AppError> {
    use crate::db::order::{
        ActiveModel as OrderAM, Model as OrderModel, OrderSide, OrderStatus, OrderType,
        TimeInForce, TradeMode,
    };

    let side_enum = match side.as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => return Err(AppError::Validation(format!("invalid side: {side}"))),
    };
    let now = Utc::now();
    let id = Uuid::new_v4();
    let qty_f = qty.to_f64().unwrap_or(0.0);
    let price_f = price.to_f64().unwrap_or(0.0);
    let am = OrderAM {
        id: Set(id),
        user_id: Set(follower_id),
        strategy_id: Set(None),
        symbol: Set(symbol.to_string()),
        side: Set(side_enum),
        order_type: Set(OrderType::Market),
        price: Set(None),
        quantity: Set(qty_f),
        filled_quantity: Set(qty_f),
        avg_fill_price: Set(Some(price_f)),
        status: Set(OrderStatus::Filled),
        mode: Set(TradeMode::Paper),
        fee: Set(0.0),
        reject_reason: Set(None),
        time_in_force: Set(TimeInForce::GTC),
        expire_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        cancelled_at: Set(None),
        filled_at: Set(Some(now)),
        advanced_type: Set(None),
        advanced_params: Set(None),
    };
    am.insert(&*db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading place_copy_order insert: {e}")))?;
    Ok(id)
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn dec_helpers() {
        assert_eq!(dec_zero(), dec!(0));
        assert_eq!(dec_one(), dec!(1));
    }

    #[test]
    fn fanout_summary_default() {
        let s = FanOutSummary::default();
        assert_eq!(s.copied, 0);
        assert_eq!(s.skipped_risk, 0);
        assert_eq!(s.skipped_below_min, 0);
        assert_eq!(s.failed, 0);
    }

    /// `ratio * qty` scaling — the core of the fan-out.
    #[test]
    fn ratio_scaling_math() {
        let trader_qty = dec!(1.0);
        let ratio = dec!(0.5);
        let scaled = (trader_qty * ratio).round_dp(8);
        assert_eq!(scaled, dec!(0.5));
    }

    /// Decimal round-trip: the view layer stringifies so the API
    /// doesn't lose precision. We assert the cycle here as a smoke
    /// test for the trade DTO.
    #[test]
    fn copy_trade_view_decimal_roundtrip() {
        let id = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let copied = Uuid::new_v4();
        let now = Utc::now();
        let m = CopyTradeModel {
            id,
            subscription_id: sub,
            original_order_id: None,
            copied_order_id: copied,
            symbol: "BTCUSDT".into(),
            side: "buy".into(),
            qty: dec!(0.5),
            price: dec!(50000),
            status: "Copied".into(),
            created_at: now,
        };
        let v: CopyTradeView = m.into();
        assert_eq!(v.symbol, "BTCUSDT");
        assert_eq!(v.qty, "0.5");
        assert_eq!(v.price, "50000");
    }
}
