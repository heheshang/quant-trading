pub mod backtest;
pub mod backtest_results;
pub mod dashboard;
pub mod kline;
pub mod order;
pub mod permission;
pub mod portfolio;
pub mod role;
pub mod role_permission;
pub mod strategy;
pub mod ticker_snapshot;
pub mod user;
pub mod user_session;

use sea_orm::PaginatorTrait;
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, EntityTrait, Schema,
};
use std::sync::Arc;
use tracing::info;

pub type DbPool = Arc<DatabaseConnection>;

/// Initialize database connection pool
pub async fn init_db(database_url: &str) -> Result<DbPool, sea_orm::DbErr> {
    info!("Connecting to database...");
    let conn = Database::connect(database_url).await?;
    info!("Database connected successfully");
    Ok(Arc::new(conn))
}

/// Run database migrations (creates tables if they don't exist)
pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let backend = DatabaseBackend::Postgres;
    let schema = Schema::new(backend);

    // Create roles table
    let stmt = backend.build(
        schema
            .create_table_from_entity(role::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create permissions table
    let stmt = backend.build(
        schema
            .create_table_from_entity(permission::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create role_permissions table
    let stmt = backend.build(
        schema
            .create_table_from_entity(role_permission::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create users table
    let stmt = backend.build(
        schema
            .create_table_from_entity(user::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create user_sessions table
    let stmt = backend.build(
        schema
            .create_table_from_entity(user_session::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create strategies table
    let stmt = backend.build(
        schema
            .create_table_from_entity(strategy::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Add description column to strategies table (migration for existing DBs)
    let alter_sql = "ALTER TABLE strategies ADD COLUMN IF NOT EXISTS description VARCHAR(500) NOT NULL DEFAULT ''";
    db.execute(sea_orm::Statement::from_string(
        backend,
        alter_sql.to_string(),
    ))
    .await?;

    // Create backtest_results table
    let stmt = backend.build(
        schema
            .create_table_from_entity(backtest_results::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create klines table
    let stmt = backend.build(
        schema
            .create_table_from_entity(kline::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Add user_id column (kline_entity uses UUID but existing table may have bigserial id)
    let alter_user_id = "ALTER TABLE klines ADD COLUMN IF NOT EXISTS user_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'";
    db.execute(sea_orm::Statement::from_string(
        backend,
        alter_user_id.to_string(),
    ))
    .await?;

    // Add deleted_at column to klines table for soft delete (migration for existing DBs)
    let alter_sql = "ALTER TABLE klines ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ";
    db.execute(sea_orm::Statement::from_string(
        backend,
        alter_sql.to_string(),
    ))
    .await?;

    // Create klines_phase4 partitioned table (Phase 4 T4: Historical Kline Persistence)
    // Each statement must be executed separately — PostgreSQL prepared statements support only one command.
    db.execute(sea_orm::Statement::from_string(
        backend,
        r#"
        CREATE TABLE IF NOT EXISTS klines_phase4 (
            id              UUID        NOT NULL DEFAULT gen_random_uuid(),
            symbol          VARCHAR(20) NOT NULL,
            interval        VARCHAR(10) NOT NULL,
            open_time       TIMESTAMPTZ NOT NULL,
            close_time      TIMESTAMPTZ NOT NULL,
            open            DECIMAL(20,8) NOT NULL,
            high            DECIMAL(20,8) NOT NULL,
            low             DECIMAL(20,8) NOT NULL,
            close           DECIMAL(20,8) NOT NULL,
            volume          DECIMAL(20,8) NOT NULL,
            quote_volume    DECIMAL(20,8) NOT NULL,
            trades          INT         NOT NULL DEFAULT 0,
            source          VARCHAR(20) NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at      TIMESTAMPTZ,
            UNIQUE (symbol, interval, open_time) DEFERRABLE INITIALLY DEFERRED
        ) PARTITION BY RANGE (open_time)
    "#
        .to_string(),
    ))
    .await?;

    for (partition, start, end) in [
        ("klines_phase4_202605", "2026-05-01", "2026-06-01"),
        ("klines_phase4_202606", "2026-06-01", "2026-07-01"),
        ("klines_phase4_202607", "2026-07-01", "2026-08-01"),
        ("klines_phase4_202608", "2026-08-01", "2026-09-01"),
        ("klines_phase4_202609", "2026-09-01", "2026-10-01"),
        ("klines_phase4_202610", "2026-10-01", "2026-11-01"),
    ] {
        db.execute(sea_orm::Statement::from_string(backend, format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF klines_phase4 FOR VALUES FROM ('{}') TO ('{}')",
            partition, start, end
        ))).await?;
    }

    db.execute(sea_orm::Statement::from_string(backend,
        "CREATE UNIQUE INDEX IF NOT EXISTS uq_klines_phase4 ON klines_phase4 (symbol, interval, open_time)".to_string()
    )).await?;
    db.execute(sea_orm::Statement::from_string(backend,
        "CREATE INDEX IF NOT EXISTS idx_klines_phase4_lookup ON klines_phase4 (symbol, interval, open_time DESC)".to_string()
    )).await?;

    // Create orders table (trading module)
    let stmt = backend.build(
        schema
            .create_table_from_entity(order::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create trades table
    let stmt = backend.build(
        schema
            .create_table_from_entity(order::trades::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create positions table
    let stmt = backend.build(
        schema
            .create_table_from_entity(order::positions::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create paper_accounts table
    let stmt = backend.build(
        schema
            .create_table_from_entity(order::paper_accounts::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create portfolio_equity_history table
    let stmt = backend.build(
        schema
            .create_table_from_entity(portfolio::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create dashboard_stats table
    let stmt = backend.build(
        schema
            .create_table_from_entity(dashboard::dashboard_stats::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create pnl_history table
    let stmt = backend.build(
        schema
            .create_table_from_entity(dashboard::pnl_history::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Add FK from portfolio_equity_history.user_id → users.id (if not already present)
    // This FK cannot be created in docker-init SQL because users table is created here.
    let fk_sql = r#"
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM information_schema.table_constraints
                WHERE constraint_name = 'fk_portfolio_equity_user'
                  AND table_name = 'portfolio_equity_history'
            ) THEN
                ALTER TABLE portfolio_equity_history
                    ADD CONSTRAINT fk_portfolio_equity_user
                    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
            END IF;
        END $$;
    "#;
    db.execute(sea_orm::Statement::from_string(backend, fk_sql.to_string()))
        .await?;

    // Create symbol_configs table
    let stmt = backend.build(
        schema
            .create_table_from_entity(order::symbol_configs::Entity)
            .if_not_exists(),
    );
    db.execute(stmt).await?;

    // Create ticker_snapshots table using raw SQL (partitioned table not supported by SeaORM schema builder)
    db.execute(sea_orm::Statement::from_string(
        backend,
        r#"
        CREATE TABLE IF NOT EXISTS ticker_snapshots (
            id          UUID        NOT NULL DEFAULT gen_random_uuid(),
            symbol      VARCHAR(20) NOT NULL,
            price       DECIMAL(20,8) NOT NULL,
            change      DECIMAL(20,8) NOT NULL DEFAULT 0,
            change_percent DECIMAL(10,4) NOT NULL DEFAULT 0,
            volume      DECIMAL(20,8) NOT NULL DEFAULT 0,
            high        DECIMAL(20,8) NOT NULL DEFAULT 0,
            low         DECIMAL(20,8) NOT NULL DEFAULT 0,
            bid         DECIMAL(20,8) NOT NULL DEFAULT 0,
            ask         DECIMAL(20,8) NOT NULL DEFAULT 0,
            timestamp   TIMESTAMPTZ NOT NULL,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (id, timestamp)
        ) PARTITION BY RANGE (timestamp)
    "#
        .to_string(),
    ))
    .await?;

    for (partition, start, end) in [
        ("ticker_snapshots_202605", "2026-05-01", "2026-06-01"),
        ("ticker_snapshots_202606", "2026-06-01", "2026-07-01"),
        ("ticker_snapshots_202607", "2026-07-01", "2026-08-01"),
        ("ticker_snapshots_202608", "2026-08-01", "2026-09-01"),
        ("ticker_snapshots_202609", "2026-09-01", "2026-10-01"),
        ("ticker_snapshots_202610", "2026-10-01", "2026-11-01"),
    ] {
        db.execute(sea_orm::Statement::from_string(backend, format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF ticker_snapshots FOR VALUES FROM ('{}') TO ('{}')",
            partition, start, end
        ))).await?;
    }

    db.execute(sea_orm::Statement::from_string(backend,
        "CREATE INDEX IF NOT EXISTS idx_ticker_snapshots_lookup ON ticker_snapshots (symbol, timestamp DESC)".to_string()
    )).await?;

    info!("Database migrations completed");

    // Seed default roles if none exist
    seed_default_roles(db).await?;

    Ok(())
}

async fn seed_default_roles(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let count = role::Entity::find().count(db).await?;

    if count == 0 {
        info!("Seeding default roles...");

        let now = chrono::Utc::now();

        // Admin role
        let admin = role::ActiveModel {
            id: sea_orm::Set(uuid::Uuid::new_v4()),
            name: sea_orm::Set("admin".into()),
            display_name: sea_orm::Set("管理员".into()),
            description: sea_orm::Set(Some("系统管理员，拥有所有权限".into())),
            is_system: sea_orm::Set(true),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        role::Entity::insert(admin).exec(db).await?;

        // User role
        let user_role = role::ActiveModel {
            id: sea_orm::Set(uuid::Uuid::new_v4()),
            name: sea_orm::Set("user".into()),
            display_name: sea_orm::Set("普通用户".into()),
            description: sea_orm::Set(Some("普通用户，可访问基本功能".into())),
            is_system: sea_orm::Set(true),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        role::Entity::insert(user_role).exec(db).await?;

        info!("Default roles seeded");
    }

    Ok(())
}
