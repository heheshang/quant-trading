//! services/dashboard.rs — Dashboard aggregation service layer
//!
//! Implements: get_stats, get_pnl_history

use chrono::{Duration, TimeZone, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::dashboard;
use crate::db::order::{self, OrderSide, OrderStatus};
use crate::db::strategy;
use crate::db::user;
use crate::utils::error::AppError;

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardStats {
    pub total_users: i64,
    pub active_strategies: i64,
    pub total_orders_today: i64,
    pub total_pnl_today: f64,
    pub win_rate: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PnLPoint {
    pub timestamp: String,
    pub pnl: f64,
    pub equity: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PnLHistory {
    pub points: Vec<PnLPoint>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PnLQuery {
    pub range: String, // "7d", "30d", "90d"
}

// ─── Helper: format f64 as financial string ────────────────────

    #[allow(dead_code)]
    fn fmt_money(v: f64) -> String {
        format!("{:.2}", v)
    }

    #[allow(dead_code)]
    fn fmt_pct(v: f64) -> String {
        format!("{:.2}", v)
    }

// ─── Service Functions ──────────────────────────────────────────

/// GET /api/v1/dashboard/stats — Dashboard statistics
///
/// Aggregates: total_users, active_strategies, orders_today, pnl_today, win_rate
pub async fn get_stats(db: &DatabaseConnection, user_id: Uuid) -> Result<DashboardStats, AppError> {
    // 1. Count total users (system-wide for admin, or just self)
    // For simplicity, we count all users in the system
    let total_users = user::Entity::find()
        .filter(user::Column::IsActive.eq(true))
        .count(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? as i64;

    // 2. Count active strategies for this user
    let active_strategies = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .filter(strategy::Column::Status.eq("active"))
        .count(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? as i64;

    // 3. Count today's orders
    let today_start = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default();
    let today_start_utc = Utc.from_utc_datetime(&today_start);

    let total_orders_today = order::Entity::find()
        .filter(order::Column::UserId.eq(user_id))
        .filter(order::Column::UpdatedAt.gte(today_start_utc))
        .count(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? as i64;

    // 4. Calculate today's PnL from filled orders
    let today_orders = order::Entity::find()
        .filter(order::Column::UserId.eq(user_id))
        .filter(order::Column::Status.eq(OrderStatus::Filled))
        .filter(order::Column::UpdatedAt.gte(today_start_utc))
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let total_pnl_today: f64 = today_orders
        .iter()
        .map(|o| {
            let sign = match o.side {
                OrderSide::Buy => 1.0,
                OrderSide::Sell => -1.0,
            };
            if let Some(avg_price) = o.avg_fill_price {
                sign * (avg_price - o.price.unwrap_or(0.0)) * o.filled_quantity - o.fee
            } else {
                -o.fee
            }
        })
        .sum();

    // 5. Calculate win rate from orders
    let all_orders = order::Entity::find()
        .filter(order::Column::UserId.eq(user_id))
        .filter(order::Column::Status.eq(OrderStatus::Filled))
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let total_trades = all_orders.len() as i64;
    let winning_trades = all_orders
        .iter()
        .filter(|o| {
            let sign = match o.side {
                OrderSide::Buy => 1.0,
                OrderSide::Sell => -1.0,
            };
            if let Some(avg_price) = o.avg_fill_price {
                let pnl = sign * (avg_price - o.price.unwrap_or(0.0)) * o.filled_quantity - o.fee;
                pnl > 0.0
            } else {
                false
            }
        })
        .count() as i64;

    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    Ok(DashboardStats {
        total_users,
        active_strategies,
        total_orders_today,
        total_pnl_today,
        win_rate,
    })
}

/// GET /api/v1/dashboard/pnl?range=7d|30d|90d — PnL time series data
///
/// Returns daily PnL and equity curve for the specified range
pub async fn get_pnl_history(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: &PnLQuery,
) -> Result<PnLHistory, AppError> {
    // Parse range to get number of days
    let days = match params.range.as_str() {
        "7d" => 7,
        "30d" => 30,
        "90d" => 90,
        _ => 7, // default to 7 days
    };

    let start_time = Utc::now() - Duration::days(days);

    // Query pnl_history table for records in range
    let records = dashboard::pnl_history::Entity::find()
        .filter(dashboard::pnl_history::Column::UserId.eq(user_id))
        .filter(dashboard::pnl_history::Column::Timestamp.gte(start_time))
        .order_by_asc(dashboard::pnl_history::Column::Timestamp)
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // If no records in pnl_history, generate from orders
    if records.is_empty() {
        return generate_pnl_from_orders(db, user_id, days).await;
    }

    let points: Vec<PnLPoint> = records
        .iter()
        .map(|r| PnLPoint {
            timestamp: r.timestamp.to_rfc3339(),
            pnl: r.pnl,
            equity: r.equity,
        })
        .collect();

    Ok(PnLHistory { points })
}

/// Generate PnL history from orders when pnl_history table is empty
async fn generate_pnl_from_orders(
    db: &DatabaseConnection,
    user_id: Uuid,
    days: i64,
) -> Result<PnLHistory, AppError> {
    let start_time = Utc::now() - Duration::days(days);

    // Get all filled orders in range
    let orders = order::Entity::find()
        .filter(order::Column::UserId.eq(user_id))
        .filter(order::Column::Status.eq(OrderStatus::Filled))
        .filter(order::Column::UpdatedAt.gte(start_time))
        .order_by_asc(order::Column::UpdatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Group by day and calculate daily PnL
    let mut daily_pnl: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();

    for o in &orders {
        let day_key = o.updated_at.format("%Y-%m-%d").to_string();
        let sign = match o.side {
            OrderSide::Buy => 1.0,
            OrderSide::Sell => -1.0,
        };
        let pnl = if let Some(avg_price) = o.avg_fill_price {
            sign * (avg_price - o.price.unwrap_or(0.0)) * o.filled_quantity - o.fee
        } else {
            -o.fee
        };

        let entry = daily_pnl.entry(day_key).or_insert((0.0, 0.0));
        entry.0 += pnl;
        entry.1 += 1.0; // count for equity estimation
    }

    // Convert to points
    let mut points: Vec<PnLPoint> = Vec::new();
    let mut running_equity = 100000.0; // default initial equity

    for i in 0..days {
        let date = Utc::now() - Duration::days(days - i - 1);
        let day_key = date.format("%Y-%m-%d").to_string();

        if let Some((pnl, _)) = daily_pnl.get(&day_key) {
            running_equity += pnl;
            points.push(PnLPoint {
                timestamp: date.to_rfc3339(),
                pnl: *pnl,
                equity: running_equity,
            });
        } else {
            points.push(PnLPoint {
                timestamp: date.to_rfc3339(),
                pnl: 0.0,
                equity: running_equity,
            });
        }
    }

    Ok(PnLHistory { points })
}

// ─── Unit Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_stats_serialization() {
        let stats = DashboardStats {
            total_users: 100,
            active_strategies: 25,
            total_orders_today: 150,
            total_pnl_today: 1234.56,
            win_rate: 65.5,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_users"], 100);
        assert_eq!(json["active_strategies"], 25);
        assert!((json["total_pnl_today"].as_f64().unwrap() - 1234.56).abs() < 0.01);
    }

    #[test]
    fn test_pnl_point_serialization() {
        let point = PnLPoint {
            timestamp: "2026-05-14T00:00:00Z".to_string(),
            pnl: 500.25,
            equity: 100500.75,
        };
        let json = serde_json::to_value(&point).unwrap();
        assert_eq!(json["timestamp"], "2026-05-14T00:00:00Z");
        assert!((json["pnl"].as_f64().unwrap() - 500.25).abs() < 0.01);
    }

    #[test]
    fn test_pnl_history_serialization() {
        let history = PnLHistory {
            points: vec![
                PnLPoint {
                    timestamp: "2026-05-14T00:00:00Z".to_string(),
                    pnl: 500.0,
                    equity: 100500.0,
                },
                PnLPoint {
                    timestamp: "2026-05-15T00:00:00Z".to_string(),
                    pnl: 250.0,
                    equity: 100750.0,
                },
            ],
        };
        let json = serde_json::to_value(&history).unwrap();
        assert_eq!(json["points"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_pnl_query_deserialization() {
        let json = serde_json::json!({ "range": "30d" });
        let query: PnLQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.range, "30d");
    }

    #[test]
    fn test_fmt_money_formatting() {
        assert_eq!(fmt_money(1234.5678), "1234.57");
        assert_eq!(fmt_money(0.0), "0.00");
        assert_eq!(fmt_money(-500.123), "-500.12");
    }

    #[test]
    fn test_fmt_pct_formatting() {
        assert_eq!(fmt_pct(65.5), "65.50");
        assert_eq!(fmt_pct(0.0), "0.00");
        assert_eq!(fmt_pct(100.0), "100.00");
    }
}
