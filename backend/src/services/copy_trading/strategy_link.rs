//! `services::copy_trading::strategy_link` — bridge between a copy
//! trader and a strategy.
//!
//! A trader may optionally link a `strategy_id` (PRD §6-2 Part 2). When
//! linked, the strategy's `on_trade` signal can fire `on_trader_order`
//! on the trader's behalf (the strategy runner is the proxy for the
//! trader's own decision). Without a link, `on_trader_order` is fired
//! by the order hot-path after the trader's manual order is placed.
//!
//! v0.1: a thin read helper. The actual signal-routing wiring lives in
//! the strategy runner; this module exposes the helper that the
//! runner calls to discover whether a trader is linked to a strategy.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::db::copy_trading::TraderEntity;
use crate::utils::error::AppError;

/// Return the `trader_id` for a given `strategy_id`, if the strategy
/// is currently bound to a trader. Used by the strategy runner to
/// short-circuit the fan-out: if no trader is bound, the strategy's
/// orders are NOT mirrored to followers.
pub async fn trader_id_for_strategy(
    db: &DatabaseConnection,
    strategy_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    // The link is indirect: the strategy runner owns the strategy; the
    // trader can register with a `strategy_id` field. We keep the
    // linkage as a column on the trader row in a future migration; for
    // v0.1 we treat the link as best-effort and look it up via the
    // user_id path. The function is a no-op for the paper-trading
    // path until a `link_strategy` endpoint is shipped.
    use crate::db::copy_trading::trader::Column as TraderCol;
    let row = TraderEntity::find()
        .filter(TraderCol::Status.eq("Active"))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("copy_trading strategy_link find: {e}")))?;
    // No `strategy_id` column on the trader yet (deferred to v0.2 to
    // avoid blocking §6-2). Return None for now.
    let _ = strategy_id;
    let _ = row;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_compiles() {
        // Type-signature smoke test. The real lookup requires a DB.
        fn _check<F: Fn()>(_: F) {}
        let _f: fn() = || {
            let _ = std::any::type_name::<fn(
                &sea_orm::DatabaseConnection,
                uuid::Uuid,
            ) -> _>();
        };
        _check(|| {});
    }
}
