//! End-to-end integration tests for `services::trigger_order::TriggerOrderService`.
//!
//! These tests run against a live Postgres instance (Docker container
//! `quant-postgres`) on `localhost:5432`. We use the `tests/` directory so
//! the binary is linked against the public lib API exactly like production
//! (i.e. exercises the raw-SQL enum-cast paths, not the lib's `#[cfg(test)]`
//! stubs).
//!
//! ## Why we need a real Postgres
//!
//! `TriggerOrderService` writes to the `trigger_orders` table, whose
//! `trigger_type` / `status` / `trigger_direction` columns are real Postgres
//! enums. SeaORM 1.1.x's `DeriveActiveEnum` generates a writer that emits
//! the variant's Rust name as plain `text`, which Postgres rejects
//! (`text → trigger_type` type-mismatch). The service now uses raw SQL
//! `INSERT … '{string_value}'::trigger_type` for writes and
//! `SELECT … '…'::text` for reads. This test suite guards those paths so
//! a future SeaORM upgrade that silently reverts the workaround will fail
//! in CI.
//!
//! ## pg_hba.conf requirement
//!
//! The dev container must allow trust auth on the docker bridge
//! (see `references/integration-test-postgres-trust-vs-scram-pitfall.md`).
//! Run the cargo tests with `DATABASE_URL` set to the same URL the backend
//! uses; the service will trust the connection if pg_hba has been
//! preconfigured.

use rand::Rng;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

use quant_trading_backend::db::order::positions::ActiveModel as PositionActive;
use quant_trading_backend::db::order::{PositionSide, TradeMode};
use quant_trading_backend::db::role::ActiveModel as RoleActive;
use quant_trading_backend::db::trigger_order::{
    TriggerStatus, TriggerType as DbTriggerType,
};
use quant_trading_backend::db::user::ActiveModel as UserActive;
use quant_trading_backend::services::trigger_order::TriggerOrderService;
use quant_trading_backend::utils::error::AppError;

// ───────────────────────────── fixtures ─────────────────────────────

const DEFAULT_SYMBOL: &str = "BTCUSDT";

async fn connect() -> Arc<sea_orm::DatabaseConnection> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://quant:***@localhost:5432/quant_trading".to_string()
    });
    Arc::new(
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to test Postgres"),
    )
}

async fn insert_role(db: &sea_orm::DatabaseConnection) -> Uuid {
    let id = Uuid::new_v4();
    let suffix: u32 = rand::thread_rng().gen_range(0..u32::MAX);
    let name = format!("test-role-{}-{}", id, suffix);
    let now = chrono::Utc::now();
    let role = RoleActive {
        id: Set(id),
        name: Set(name),
        display_name: Set(format!("Test Role {}", suffix)),
        description: Set(Some("test fixture".to_string())),
        is_system: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    role.insert(db).await.expect("insert role fixture");
    id
}

async fn insert_user(db: &sea_orm::DatabaseConnection, role_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let suffix: u32 = rand::thread_rng().gen_range(0..u32::MAX);
    let now = chrono::Utc::now();
    let user = UserActive {
        id: Set(id),
        username: Set(format!("test_user_{}", suffix)),
        email: Set(format!("test_{}@example.com", suffix)),
        password_hash: Set("test-hash-fixture-001".to_string()),
        display_name: Set(None),
        avatar_url: Set(None),
        role_id: Set(role_id),
        is_active: Set(true),
        last_login_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    user.insert(db).await.expect("insert user fixture");
    id
}

async fn insert_position(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    side: PositionSide,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let pos = PositionActive {
        id: Set(id),
        user_id: Set(user_id),
        symbol: Set(DEFAULT_SYMBOL.to_string()),
        side: Set(side),
        quantity: Set(1.0),
        available_quantity: Set(1.0),
        avg_entry_price: Set(50_000.0),
        unrealized_pnl: Set(0.0),
        realized_pnl: Set(0.0),
        mode: Set(TradeMode::Paper),
        created_at: Set(now),
        updated_at: Set(now),
    };
    pos.insert(db).await.expect("insert position fixture");
    id
}

async fn setup_fixture(db: &sea_orm::DatabaseConnection) -> (Uuid, Uuid) {
    let role_id = insert_role(db).await;
    let user_id = insert_user(db, role_id).await;
    (user_id, role_id)
}

async fn cleanup_fixture(db: &sea_orm::DatabaseConnection, user_id: Uuid, role_id: Uuid) {
    use quant_trading_backend::db::order::positions::Entity as PositionEntity;
    use quant_trading_backend::db::order::positions::Column as PositionColumn;
    use quant_trading_backend::db::role::Entity as RoleEntity;
    use quant_trading_backend::db::user::Entity as UserEntity;
    let pos_ids: Vec<Uuid> = PositionEntity::find()
        .filter(PositionColumn::UserId.eq(user_id))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.id)
        .collect();
    if !pos_ids.is_empty() {
        let _ = PositionEntity::delete_many()
            .filter(PositionColumn::Id.is_in(pos_ids))
            .exec(db)
            .await;
    }
    let _ = UserEntity::delete_by_id(user_id).exec(db).await;
    let _ = RoleEntity::delete_by_id(role_id).exec(db).await;
}

// ───────────────────────────── tests ─────────────────────────────

#[tokio::test]
async fn test_create_stop_loss_long_position_triggers_down() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let pos_id = insert_position(&db, user_id, PositionSide::Long).await;

    let result = svc
        .create_stop_loss(user_id, pos_id, DEFAULT_SYMBOL, 49_000.0, Some(50_000.0), 1.0)
        .await
        .expect("create_stop_loss should succeed for Long");

    assert_eq!(result.trigger_type, DbTriggerType::StopLoss);
    assert_eq!(result.symbol, DEFAULT_SYMBOL);
    assert_eq!(result.trigger_price, 49_000.0);
    assert_eq!(result.quantity, 1.0);
    assert_eq!(result.user_id, user_id);
    assert_eq!(result.position_id, Some(pos_id));
    // Long position stop-loss triggers when price drops
    assert_eq!(
        result.trigger_direction,
        quant_trading_backend::db::trigger_order::TriggerDirection::Down
    );

    // Round-trip via get_trigger_order (which now also goes through the
    // raw-text cast path) must return identical data.
    let fetched = svc
        .get_trigger_order(user_id, result.id)
        .await
        .expect("get_trigger_order");
    assert_eq!(fetched.id, result.id);
    assert_eq!(fetched.trigger_type, DbTriggerType::StopLoss);
    assert_eq!(fetched.trigger_price, 49_000.0);

    cleanup_fixture(&db, user_id, role_id).await;
}

#[tokio::test]
async fn test_create_stop_loss_short_position_triggers_up() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let pos_id = insert_position(&db, user_id, PositionSide::Short).await;

    let result = svc
        .create_stop_loss(user_id, pos_id, DEFAULT_SYMBOL, 51_000.0, Some(50_000.0), 0.5)
        .await
        .expect("create_stop_loss should succeed for Short");

    assert_eq!(result.trigger_type, DbTriggerType::StopLoss);
    assert_eq!(result.quantity, 0.5);
    // Short position stop-loss triggers when price rises
    assert_eq!(
        result.trigger_direction,
        quant_trading_backend::db::trigger_order::TriggerDirection::Up
    );

    cleanup_fixture(&db, user_id, role_id).await;
}

#[tokio::test]
async fn test_create_stop_loss_position_not_found() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let fake_pos_id = Uuid::new_v4();

    let result = svc
        .create_stop_loss(user_id, fake_pos_id, DEFAULT_SYMBOL, 49_000.0, None, 1.0)
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));

    cleanup_fixture(&db, user_id, role_id).await;
}

#[tokio::test]
async fn test_create_take_profit_long_position_triggers_up() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let pos_id = insert_position(&db, user_id, PositionSide::Long).await;

    let result = svc
        .create_take_profit(
            user_id,
            pos_id,
            DEFAULT_SYMBOL,
            52_000.0,
            Some(50_000.0),
            0.25,
        )
        .await
        .expect("create_take_profit should succeed for Long");

    assert_eq!(result.trigger_type, DbTriggerType::TakeProfit);
    assert_eq!(result.quantity, 0.25);
    // Long take-profit triggers when price rises
    assert_eq!(
        result.trigger_direction,
        quant_trading_backend::db::trigger_order::TriggerDirection::Up
    );

    cleanup_fixture(&db, user_id, role_id).await;
}

#[tokio::test]
async fn test_list_trigger_orders_filters_by_user() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id_a, role_a) = setup_fixture(&db).await;
    let pos_a = insert_position(&db, user_id_a, PositionSide::Long).await;

    // user A creates two orders
    svc.create_stop_loss(user_id_a, pos_a, DEFAULT_SYMBOL, 49_000.0, None, 0.1)
        .await
        .expect("first stop loss");
    svc.create_stop_loss(user_id_a, pos_a, DEFAULT_SYMBOL, 48_000.0, None, 0.2)
        .await
        .expect("second stop loss");

    let (user_id_b, role_b) = setup_fixture(&db).await;
    let pos_b = insert_position(&db, user_id_b, PositionSide::Long).await;
    svc.create_stop_loss(user_id_b, pos_b, DEFAULT_SYMBOL, 47_000.0, None, 0.3)
        .await
        .expect("user b stop loss");

    // user A should only see their own orders
    let a_orders = svc
        .list_trigger_orders(user_id_a, None, None)
        .await
        .expect("list A");
    assert!(
        a_orders.iter().all(|o| o.user_id == user_id_a),
        "user A list must not contain user B orders"
    );
    assert!(
        a_orders.len() >= 2,
        "user A should see at least 2 orders, got {}",
        a_orders.len()
    );

    // user B should only see their own
    let b_orders = svc
        .list_trigger_orders(user_id_b, None, None)
        .await
        .expect("list B");
    assert_eq!(b_orders.len(), 1);
    assert_eq!(b_orders[0].user_id, user_id_b);

    cleanup_fixture(&db, user_id_a, role_a).await;
    cleanup_fixture(&db, user_id_b, role_b).await;
}

#[tokio::test]
async fn test_cancel_trigger_order_marks_cancelled() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let pos_id = insert_position(&db, user_id, PositionSide::Long).await;

    let created = svc
        .create_stop_loss(user_id, pos_id, DEFAULT_SYMBOL, 49_000.0, None, 0.5)
        .await
        .expect("create");

    svc.cancel_trigger_order(created.id, "user cancelled")
        .await
        .expect("cancel should succeed");

    let fetched = svc
        .get_trigger_order(user_id, created.id)
        .await
        .expect("fetch after cancel");
    assert_eq!(fetched.status, TriggerStatus::Cancelled);

    // Cancelling a cancelled order should fail with Conflict
    let second = svc
        .cancel_trigger_order(created.id, "again")
        .await;
    assert!(matches!(second, Err(AppError::Conflict(_))));

    cleanup_fixture(&db, user_id, role_id).await;
}

#[tokio::test]
async fn test_create_oco_pair_links_both_sides() {
    let db = connect().await;
    let svc = TriggerOrderService::new(db.clone());
    let (user_id, role_id) = setup_fixture(&db).await;
    let pos_id = insert_position(&db, user_id, PositionSide::Long).await;

    let (stop, profit) = svc
        .create_oco(
            user_id,
            pos_id,
            DEFAULT_SYMBOL,
            48_000.0,  // stop loss
            52_000.0,  // take profit
            Some(50_000.0),
            0.5,
        )
        .await
        .expect("create_oco");

    // Both sides are OCO type and link to each other
    assert_eq!(stop.trigger_type, DbTriggerType::Oco);
    assert_eq!(profit.trigger_type, DbTriggerType::Oco);
    assert_eq!(stop.oco_pair_id, Some(profit.id));
    assert_eq!(profit.oco_pair_id, Some(stop.id));
    // Direction is mirrored for the two sides
    assert_ne!(stop.trigger_direction, profit.trigger_direction);

    cleanup_fixture(&db, user_id, role_id).await;
}
