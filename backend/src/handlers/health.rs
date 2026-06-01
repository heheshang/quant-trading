//! 健康检查 / 探针端点
//!
//! - [`liveness`]：进程在跑（k8s `livenessProbe`）
//! - [`readiness`]：依赖（DB / Redis）可达（k8s `readinessProbe`）

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::state::AppState;
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Liveness probe: 进程在跑就返回 200。
///
/// 不检查任何外部依赖。即使下游 DB/Redis 全挂，liveness 仍应返回 OK，
/// 否则 k8s 会重启 Pod，而重启并不能恢复下游。
#[tracing::instrument]
pub async fn liveness() -> impl IntoResponse {
    Json(json!({
        "code": 0,
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        },
        "message": "success"
    }))
}

/// Readiness probe: DB 与 Redis 都可达时返回 200，否则 503。
///
/// 任何一个依赖 ping 失败就把整体标记为 unready。响应体里逐项列出结果，
/// 方便 dashboard / pagerDuty 看到具体哪个依赖挂了。
#[tracing::instrument(skip(state))]
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx_query_check(&state).await;
    let redis_ok = redis_ping_check(&state).await;

    let overall_ok = db_ok && redis_ok;
    let status_code = if overall_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "code": if overall_ok { 0 } else { -1 },
            "data": {
                "status": if overall_ok { "ready" } else { "not_ready" },
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "checks": {
                    "database": if db_ok { "ok" } else { "down" },
                    "redis": if redis_ok { "ok" } else { "down" },
                }
            },
            "message": if overall_ok { "success" } else { "dependencies unhealthy" }
        })),
    )
}

/// `SELECT 1` 通过 SeaORM 直接执行，确认 DB 可达。
async fn sqlx_query_check(state: &AppState) -> bool {
    state
        .db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1".to_string(),
        ))
        .await
        .is_ok()
}

/// 通过 `redis::cmd("PING")` 探活。
async fn redis_ping_check(state: &AppState) -> bool {
    use redis::cmd;
    let mut conn = state.redis.conn();
    let pong: redis::RedisResult<String> = cmd("PING").query_async(&mut conn).await;
    pong.is_ok()
}
