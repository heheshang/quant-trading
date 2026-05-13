// ============ Backtest Request/Response Types ============
// TODO: Merge into schemas.rs during implementation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============ Request Types ============

#[derive(Debug, Deserialize)]
pub struct BacktestRunRequest {
    pub strategy_id: Uuid,
    pub config: BacktestConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BacktestConfig {
    pub symbol: String,
    pub interval: String,
    pub start_date: String, // "2024-01-01"
    pub end_date: String,   // "2024-12-31"
    pub initial_capital: f64,
    #[serde(default = "default_fee_rate")]
    pub fee_rate: f64,
    #[serde(default = "default_slippage_rate")]
    pub slippage_rate: f64,
}

fn default_fee_rate() -> f64 {
    0.001
}
fn default_slippage_rate() -> f64 {
    0.0005
}

// ============ Response Types ============

#[derive(Debug, Serialize)]
pub struct BacktestRunResponse {
    pub id: Uuid,
    pub status: String,
    pub progress: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BacktestResultResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub strategy_id: Uuid,
    pub status: String,
    pub progress: i32,
    pub config: BacktestConfig,
    pub metrics: Option<BacktestMetrics>,
    pub trades: Option<Vec<TradeRecord>>,
    pub equity_curve: Option<Vec<EquityPoint>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BacktestProgressResponse {
    pub id: Uuid,
    pub status: String,
    pub progress: i32,
    pub current_bar: i64,
    pub total_bars: i64,
    pub elapsed_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct BacktestSummary {
    pub id: Uuid,
    pub status: String,
    pub config: BacktestConfig,
    pub metrics: Option<MetricsPreview>,
    pub start_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MetricsPreview {
    pub total_return_pct: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown_pct: f64,
    pub total_trades: i32,
}

// ============ Internal Types ============

#[derive(Debug, Clone, Copy)]
pub struct Kline {
    pub open_time: i64, // ms
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    Buy { quantity_pct: f64 }, // 0.0 ..= 1.0
    Sell { quantity_pct: f64 },
    CloseAll,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Long,
    Short,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub initial_capital: f64,
    pub cash: f64,
    pub equity: f64,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub direction: Direction,
    pub quantity: f64,
    pub entry_price: f64,
    pub entry_time: i64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub fee_paid: f64,
    pub slippage_paid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub entry_time: i64,
    pub exit_time: i64,
    pub direction: String, // "long" | "short"
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl_usdt: f64,
    pub pnl_pct: f64,
    pub holding_period_ms: i64,
    pub exit_reason: String, // "signal" | "stop_loss" | "take_profit" | "liquidation"
    pub fee: f64,
    pub slippage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub time: i64,
    pub equity: f64,
    pub drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return_pct: f64,
    pub annualized_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub win_rate: f64,
    pub total_trades: i32,
    pub profit_factor: f64,
    pub avg_win_pct: f64,
    pub avg_loss_pct: f64,
    pub avg_trade_pct: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
}

// ============ Validation ============

impl BacktestConfig {
    pub fn validate(&self) -> Result<(), String> {
        // Parse dates
        let start = chrono::NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d")
            .map_err(|_| "start_date must be YYYY-MM-DD".to_string())?;
        let end = chrono::NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d")
            .map_err(|_| "end_date must be YYYY-MM-DD".to_string())?;

        if start >= end {
            return Err("start_date must be before end_date".into());
        }

        let days = (end - start).num_days();
        if days < 7 {
            return Err("date range must be at least 7 days".into());
        }

        if self.initial_capital < 100.0 {
            return Err("initial_capital must be >= 100".into());
        }

        if !(0.0..=0.01).contains(&self.fee_rate) {
            return Err("fee_rate must be 0..0.01".into());
        }

        if !(0.0..=0.01).contains(&self.slippage_rate) {
            return Err("slippage_rate must be 0..0.01".into());
        }

        Ok(())
    }
}
