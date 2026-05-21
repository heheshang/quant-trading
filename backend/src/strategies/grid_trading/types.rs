//! Grid Trading Types — P3-F2

use serde::{Deserialize, Serialize};

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
}

/// Market mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketMode {
    Ranging,  // Low volatility, good for grids
    Trending, // High volatility, grid risky
    Unknown,
}

/// Martingale config
#[derive(Debug, Clone)]
pub struct MartingaleConfig {
    pub multiplier: f64,         // Default 2.0
    pub max_consecutive: u32,   // Default 5
}

impl Default for MartingaleConfig {
    fn default() -> Self {
        Self { multiplier: 2.0, max_consecutive: 5 }
    }
}

/// Dynamic grid config
#[derive(Debug, Clone)]
pub struct DynamicGridConfig {
    pub atr_period: u32,
    pub atr_multiplier: f64,
    pub min_price_step: f64,
}

impl Default for DynamicGridConfig {
    fn default() -> Self {
        Self { atr_period: 14, atr_multiplier: 2.0, min_price_step: 0.1 }
    }
}

/// Grid strategy config
#[derive(Debug, Clone)]
pub struct GridConfig {
    pub symbol: String,
    pub lower_price: f64,
    pub upper_price: f64,
    pub grid_count: u32,
    pub quantity_per_grid: f64,
    pub martingale: Option<MartingaleConfig>,
    pub dynamic: Option<DynamicGridConfig>,
}

impl GridConfig {
    pub fn grid_spacing(&self) -> f64 {
        (self.upper_price - self.lower_price) / self.grid_count as f64
    }
}

/// Grid order
#[derive(Debug, Clone)]
pub struct GridOrder {
    pub order_id: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub grid_level: u32,
    pub is_martingale: bool,
    pub timestamp_ms: i64,
}

impl GridOrder {
    pub fn new_buy(price: f64, quantity: f64, grid_level: u32, is_martingale: bool) -> Self {
        Self {
            order_id: uuid::Uuid::new_v4().to_string(),
            side: OrderSide::BUY,
            price,
            quantity,
            grid_level,
            is_martingale,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn new_sell(price: f64, quantity: f64, grid_level: u32, is_martingale: bool) -> Self {
        Self {
            order_id: uuid::Uuid::new_v4().to_string(),
            side: OrderSide::SELL,
            price,
            quantity,
            grid_level,
            is_martingale,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Grid position
#[derive(Debug, Clone)]
pub struct GridPosition {
    pub grid_level: u32,
    pub buy_price: f64,
    pub buy_quantity: f64,
    pub sell_price: Option<f64>,
    pub closed: bool,
    pub martingale_level: u32,
}

/// Grid status
#[derive(Debug, Clone)]
pub struct GridStatus {
    pub total_pnl: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub open_orders_count: u32,
    pub martingale_level: u32,
    pub market_mode: MarketMode,
    pub filled_grids: Vec<u32>,
    pub positions: Vec<GridPosition>,
}

impl Default for GridStatus {
    fn default() -> Self {
        Self {
            total_pnl: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            open_orders_count: 0,
            martingale_level: 0,
            market_mode: MarketMode::Unknown,
            filled_grids: Vec::new(),
            positions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_spacing() {
        let cfg = GridConfig {
            symbol: "BTCUSDT".into(),
            lower_price: 50000.0,
            upper_price: 60000.0,
            grid_count: 10,
            quantity_per_grid: 0.01,
            martingale: None,
            dynamic: None,
        };
        assert_eq!(cfg.grid_spacing(), 1000.0);
    }

    #[test]
    fn test_martingale_default() {
        let mg = MartingaleConfig::default();
        assert_eq!(mg.multiplier, 2.0);
        assert_eq!(mg.max_consecutive, 5);
    }

    #[test]
    fn test_grid_order_buy() {
        let o = GridOrder::new_buy(55000.0, 0.01, 5, false);
        assert_eq!(o.side, OrderSide::BUY);
        assert_eq!(o.grid_level, 5);
        assert!(!o.is_martingale);
    }
}
