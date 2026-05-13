pub mod backtest;
pub mod backtest_results;
pub mod kline;
pub mod permission;
pub mod role;
pub mod role_permission;
pub mod strategy;
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
