//! services/portfolio.rs — Portfolio aggregation service layer
//!
//! ADR: ADR-009 §3.4 Service 层
//! Implements: get_summary, list_positions, get_performance, get_equity_curve

use chrono::{NaiveDate, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::backtest_results;
use crate::db::order::{self, OrderSide, OrderStatus, PositionSide};
use crate::db::portfolio;
use crate::db::strategy;
use crate::utils::error::AppError;

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioSummary {
    pub total_equity: String,
    pub daily_pnl: String,
    pub daily_pnl_rate: String,
    pub cumulative_pnl: String,
    pub cumulative_pnl_rate: String,
    pub total_positions: i64,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioPosition {
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub avg_price: String,
    pub current_price: String,
    pub unrealized_pnl: String,
    pub unrealized_pnl_rate: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyPerformance {
    pub strategy_id: String,
    pub strategy_name: String,
    pub total_pnl: String,
    pub total_pnl_rate: String,
    pub max_drawdown: String,
    pub trade_count: u32,
    pub win_rate: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioPerformance {
    pub strategies: Vec<StrategyPerformance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EquityCurvePoint {
    pub timestamp: String,
    pub equity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EquityCurve {
    pub points: Vec<EquityCurvePoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PortfolioPositionsQuery {
    pub user_id: Option<Uuid>,
    pub symbol: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EquityCurveQuery {
    pub user_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub granularity: Option<String>,
}

// ─── Helper: format f64 as financial string ────────────────────

fn fmt_money(v: f64) -> String {
    format!("{:.2}", v)
}

fn fmt_pct(v: f64) -> String {
    format!("{:.2}", v)
}

// ─── Service Functions ──────────────────────────────────────────

/// GET /api/v1/portfolio/summary — 组合权益汇总
///
/// ADR-009: 从 positions + order + paper_accounts 聚合
pub async fn get_summary(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<PortfolioSummary, AppError> {
    // 1. Aggregate from paper_accounts
    let accounts = order::paper_accounts::Entity::find()
        .filter(order::paper_accounts::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let total_equity: f64 = accounts.iter().map(|a| a.balance).sum();
    let cumulative_pnl: f64 = accounts.iter().map(|a| a.total_pnl).sum();
    let initial_balance: f64 = accounts.iter().map(|a| a.initial_balance).sum();

    // 2. Count open positions
    let position_count = order::positions::Entity::find()
        .filter(order::positions::Column::UserId.eq(user_id))
        .count(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? as i64;

    // 3. Daily PnL from today's filled orders
    let today_start = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default();
    let today_start_utc = Utc.from_utc_datetime(&today_start);

    let today_orders = order::Entity::find()
        .filter(order::Column::UserId.eq(user_id))
        .filter(order::Column::Status.eq(OrderStatus::Filled))
        .filter(order::Column::UpdatedAt.gte(today_start_utc))
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let daily_pnl: f64 = today_orders
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

    // 4. Calculate rates
    let daily_pnl_rate = if initial_balance > 0.0 {
        daily_pnl / initial_balance * 100.0
    } else {
        0.0
    };
    let cumulative_pnl_rate = if initial_balance > 0.0 {
        cumulative_pnl / initial_balance * 100.0
    } else {
        0.0
    };

    Ok(PortfolioSummary {
        total_equity: fmt_money(total_equity),
        daily_pnl: fmt_money(daily_pnl),
        daily_pnl_rate: fmt_pct(daily_pnl_rate),
        cumulative_pnl: fmt_money(cumulative_pnl),
        cumulative_pnl_rate: fmt_pct(cumulative_pnl_rate),
        total_positions: position_count,
        updated_at: Utc::now().to_rfc3339(),
    })
}

/// GET /api/v1/portfolio/positions — 持仓汇总
///
/// ADR-009: 关联 positions + market_data(current_price)
pub async fn list_positions(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: &PortfolioPositionsQuery,
) -> Result<Paginated<PortfolioPosition>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let size = params.size.unwrap_or(20).clamp(1, 100);

    let mut query =
        order::positions::Entity::find().filter(order::positions::Column::UserId.eq(user_id));

    if let Some(ref symbol) = params.symbol {
        query = query.filter(order::positions::Column::Symbol.eq(symbol));
    }

    let paginator = query
        .order_by_desc(order::positions::Column::UpdatedAt)
        .paginate(db, size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let pages = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let items: Vec<PortfolioPosition> = pages
        .iter()
        .map(|p| {
            let side_str = match p.side {
                PositionSide::Long => "long",
                PositionSide::Short => "short",
            };
            let side_sign = match p.side {
                PositionSide::Long => 1.0,
                PositionSide::Short => -1.0,
            };
            // Use avg_entry_price as current_price placeholder
            // (real system would fetch from market data service)
            let current_price = p.avg_entry_price;
            let unrealized_pnl =
                (current_price - p.avg_entry_price) * p.quantity * side_sign + p.unrealized_pnl;
            let unrealized_pnl_rate = if p.avg_entry_price > 0.0 {
                (current_price - p.avg_entry_price) / p.avg_entry_price * 100.0 * side_sign
            } else {
                0.0
            };

            PortfolioPosition {
                symbol: p.symbol.clone(),
                side: side_str.to_string(),
                quantity: format!("{:.8}", p.quantity),
                avg_price: format!("{:.8}", p.avg_entry_price),
                current_price: format!("{:.8}", current_price),
                unrealized_pnl: fmt_money(unrealized_pnl),
                unrealized_pnl_rate: fmt_pct(unrealized_pnl_rate),
            }
        })
        .collect();

    Ok(Paginated {
        items,
        total,
        page,
        size,
    })
}

/// GET /api/v1/portfolio/performance — 多策略绩效率对比
///
/// ADR-009: 关联 strategy + backtest_results + order
pub async fn get_performance(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<PortfolioPerformance, AppError> {
    // 1. Get all strategies for user
    let strategies = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut result = Vec::new();

    for s in &strategies {
        // 2. Get backtest results for this strategy
        let bt = backtest_results::Entity::find()
            .filter(backtest_results::Column::StrategyId.eq(s.id))
            .filter(backtest_results::Column::Status.eq("completed"))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 3. Count trades from orders
        let trade_count = order::Entity::find()
            .filter(order::Column::StrategyId.eq(s.id))
            .filter(order::Column::Status.eq(OrderStatus::Filled))
            .count(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))? as u32;

        // 4. Extract metrics from backtest_results.metrics JSON
        let (total_pnl, total_pnl_rate, max_drawdown, win_rate) = if let Some(ref bt_model) = bt {
            if let Some(ref metrics) = bt_model.metrics {
                let total_return = metrics
                    .get("total_return")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let max_dd = metrics
                    .get("max_drawdown")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let wr = metrics
                    .get("win_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                (total_return, total_return, max_dd.abs(), wr)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        result.push(StrategyPerformance {
            strategy_id: s.id.to_string(),
            strategy_name: s.name.clone(),
            total_pnl: fmt_money(total_pnl),
            total_pnl_rate: fmt_pct(total_pnl_rate),
            max_drawdown: fmt_pct(max_drawdown),
            trade_count,
            win_rate: fmt_pct(win_rate),
        });
    }

    Ok(PortfolioPerformance { strategies: result })
}

/// GET /api/v1/portfolio/equity_curve — 权益曲线数据
///
/// ADR-009: 从 portfolio_equity_history 查询历史数据
pub async fn get_equity_curve(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: &EquityCurveQuery,
) -> Result<EquityCurve, AppError> {
    let mut query = portfolio::Entity::find().filter(portfolio::Column::UserId.eq(user_id));

    // Date range filter
    // Date range filter
    #[allow(clippy::collapsible_if)]
    if let Some(ref start_date) = params.start_date {
        #[allow(clippy::collapsible_if)]
        if let Ok(dt) = NaiveDate::parse_from_str(start_date, "%Y-%m-%d") {
            let start_utc = Utc.from_utc_datetime(&dt.and_hms_opt(0, 0, 0).unwrap_or_default());
            query = query.filter(portfolio::Column::Timestamp.gte(start_utc));
        }
    }

    #[allow(clippy::collapsible_if)]
    if let Some(ref end_date) = params.end_date {
        #[allow(clippy::collapsible_if)]
        if let Ok(dt) = NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
            let end_utc = Utc.from_utc_datetime(&dt.and_hms_opt(23, 59, 59).unwrap_or_default());
            query = query.filter(portfolio::Column::Timestamp.lte(end_utc));
        }
    }

    let records = query
        .order_by_asc(portfolio::Column::Timestamp)
        .all(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Granularity filtering
    let granularity = params.granularity.as_deref().unwrap_or("day");
    let mut points: Vec<EquityCurvePoint> = Vec::new();
    let mut last_key = String::new();

    for r in &records {
        let key = match granularity {
            "hour" => r.timestamp.format("%Y-%m-%dT%H").to_string(),
            _ => r.timestamp.format("%Y-%m-%d").to_string(),
        };

        // For same granularity bucket, keep the last entry
        if key != last_key {
            points.push(EquityCurvePoint {
                timestamp: r.timestamp.to_rfc3339(),
                equity: fmt_money(r.equity),
            });
            last_key = key;
        } else {
            // Update last point with more recent data
            if let Some(last) = points.last_mut() {
                last.timestamp = r.timestamp.to_rfc3339();
                last.equity = fmt_money(r.equity);
            }
        }
    }

    Ok(EquityCurve { points })
}

/// Record a new equity snapshot for the user
pub async fn record_equity_snapshot(
    db: &DatabaseConnection,
    user_id: Uuid,
    equity: f64,
) -> Result<(), AppError> {
    let now = Utc::now();
    let model = portfolio::ActiveModel {
        user_id: Set(user_id),
        equity: Set(equity),
        timestamp: Set(now),
        created_at: Set(now),
        ..Default::default()
    };

    model
        .insert(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

// ─── Unit Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_money_formatting() {
        assert_eq!(fmt_money(1234.5678), "1234.57");
        assert_eq!(fmt_money(0.0), "0.00");
        assert_eq!(fmt_money(-500.123), "-500.12");
        assert_eq!(fmt_money(999999.99), "999999.99");
    }

    #[test]
    fn test_fmt_pct_formatting() {
        assert_eq!(fmt_pct(1.234), "1.23");
        assert_eq!(fmt_pct(0.0), "0.00");
        assert_eq!(fmt_pct(-5.6789), "-5.68");
        assert_eq!(fmt_pct(100.0), "100.00");
    }

    #[test]
    fn test_portfolio_summary_serialization() {
        let summary = PortfolioSummary {
            total_equity: "100000.00".to_string(),
            daily_pnl: "1234.56".to_string(),
            daily_pnl_rate: "1.23".to_string(),
            cumulative_pnl: "15000.00".to_string(),
            cumulative_pnl_rate: "17.65".to_string(),
            total_positions: 5,
            updated_at: "2026-05-14T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["total_equity"], "100000.00");
        assert_eq!(json["daily_pnl"], "1234.56");
        assert_eq!(json["total_positions"], 5);
    }

    #[test]
    fn test_portfolio_position_serialization() {
        let pos = PortfolioPosition {
            symbol: "BTCUSDT".to_string(),
            side: "long".to_string(),
            quantity: "0.50000000".to_string(),
            avg_price: "65000.00000000".to_string(),
            current_price: "66000.00000000".to_string(),
            unrealized_pnl: "500.00".to_string(),
            unrealized_pnl_rate: "1.54".to_string(),
        };
        let json = serde_json::to_value(&pos).unwrap();
        assert_eq!(json["symbol"], "BTCUSDT");
        assert_eq!(json["side"], "long");
        assert_eq!(json["unrealized_pnl"], "500.00");
    }

    #[test]
    fn test_strategy_performance_serialization() {
        let sp = StrategyPerformance {
            strategy_id: Uuid::new_v4().to_string(),
            strategy_name: "MA Cross".to_string(),
            total_pnl: "5000.00".to_string(),
            total_pnl_rate: "12.50".to_string(),
            max_drawdown: "3.45".to_string(),
            trade_count: 42,
            win_rate: "65.00".to_string(),
        };
        let json = serde_json::to_value(&sp).unwrap();
        assert_eq!(json["trade_count"], 42);
        assert_eq!(json["win_rate"], "65.00");
    }

    #[test]
    fn test_equity_curve_serialization() {
        let curve = EquityCurve {
            points: vec![
                EquityCurvePoint {
                    timestamp: "2026-05-14T00:00:00Z".to_string(),
                    equity: "100000.00".to_string(),
                },
                EquityCurvePoint {
                    timestamp: "2026-05-15T00:00:00Z".to_string(),
                    equity: "101234.56".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&curve).unwrap();
        assert_eq!(json["points"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_paginated_structure() {
        let paginated: Paginated<i32> = Paginated {
            items: vec![1, 2, 3],
            total: 100,
            page: 1,
            size: 20,
        };
        let json = serde_json::to_value(&paginated).unwrap();
        assert_eq!(json["total"], 100);
        assert_eq!(json["page"], 1);
        assert_eq!(json["items"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_portfolio_positions_query_default() {
        let query = PortfolioPositionsQuery {
            user_id: None,
            symbol: None,
            page: None,
            size: None,
        };
        assert!(query.user_id.is_none());
        assert!(query.symbol.is_none());
        assert!(query.page.is_none());
        assert!(query.size.is_none());
    }

    #[test]
    fn test_equity_curve_query_default() {
        let query = EquityCurveQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            granularity: None,
        };
        assert!(query.start_date.is_none());
        assert!(query.end_date.is_none());
        assert!(query.granularity.is_none());
    }

    #[test]
    fn test_daily_pnl_calculation_logic() {
        // Simulate daily PnL from filled orders
        let orders_data: Vec<(String, f64, f64, f64, f64)> = vec![
            // (side, price, avg_fill_price, filled_quantity, fee)
            ("buy".to_string(), 100.0, 101.0, 10.0, 0.5),
            ("sell".to_string(), 200.0, 199.0, 5.0, 0.3),
        ];

        let daily_pnl: f64 = orders_data
            .iter()
            .map(|(side, price, avg_price, qty, fee)| {
                let sign = match side.as_str() {
                    "buy" => 1.0,
                    _ => -1.0,
                };
                sign * (avg_price - *price) * qty - fee
            })
            .sum();

        // buy:  1.0 * (101.0 - 100.0) * 10.0 - 0.5 = 9.5
        // sell: -1.0 * (199.0 - 200.0) * 5.0 - 0.3 = 4.7
        // total: 14.2
        assert!((daily_pnl - 14.2).abs() < 0.01);
    }

    #[test]
    fn test_cumulative_pnl_rate_calculation() {
        let initial_balance: f64 = 100000.0;
        let cumulative_pnl: f64 = 15000.0;
        let rate: f64 = if initial_balance > 0.0 {
            cumulative_pnl / initial_balance * 100.0
        } else {
            0.0
        };
        assert!((rate - 15.0_f64).abs() < f64::EPSILON);
    }

    #[test]
    fn test_unrealized_pnl_calculation() {
        // Long position
        let avg_entry_price: f64 = 65000.0;
        let quantity: f64 = 0.5;
        let side_sign: f64 = 1.0; // long
        let current_price: f64 = 66000.0;
        let unrealized_pnl: f64 = (current_price - avg_entry_price) * quantity * side_sign;
        assert!((unrealized_pnl - 500.0_f64).abs() < f64::EPSILON);

        // Short position
        let side_sign: f64 = -1.0;
        let unrealized_pnl: f64 = (current_price - avg_entry_price) * quantity * side_sign;
        assert!((unrealized_pnl - (-500.0_f64)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_zero_initial_balance_no_panic() {
        let initial_balance: f64 = 0.0;
        let cumulative_pnl: f64 = 0.0;
        let rate: f64 = if initial_balance > 0.0 {
            cumulative_pnl / initial_balance * 100.0
        } else {
            0.0
        };
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_granularity_key_generation() {
        use chrono::NaiveDateTime;
        let ts = chrono::Utc.from_utc_datetime(
            &NaiveDateTime::parse_from_str("2026-05-14T10:30:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        );

        let day_key = ts.format("%Y-%m-%d").to_string();
        assert_eq!(day_key, "2026-05-14");

        let hour_key = ts.format("%Y-%m-%dT%H").to_string();
        assert_eq!(hour_key, "2026-05-14T10");
    }
}
