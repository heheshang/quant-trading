//! Grid Engine — Core grid trading logic

use std::collections::HashMap;
use super::types::{GridConfig, GridOrder, GridPosition, GridStatus, MarketMode, OrderSide};
use super::market_mode::MarketModeDetector;
use super::martingale::MartingaleTracker;

#[derive(Debug, Clone)]
pub struct GridEngine {
    config: GridConfig,
    grid_spacing: f64,
    last_level: i32,
    pending_orders: Vec<GridOrder>,
    positions: HashMap<u32, GridPosition>,
    martingale: Option<MartingaleTracker>,
    market_detector: Option<MarketModeDetector>,
    realized_pnl: f64,
}

impl GridEngine {
    pub fn new(config: GridConfig) -> Self {
        let grid_spacing = config.grid_spacing();
        let martingale = config.martingale.clone().map(MartingaleTracker::new);
        let market_detector = config.dynamic.clone().map(|d| MarketModeDetector::new(d.atr_period, d.atr_period * 2));
        Self {
            config,
            grid_spacing,
            last_level: -1,
            pending_orders: Vec::new(),
            positions: HashMap::new(),
            martingale,
            market_detector,
            realized_pnl: 0.0,
        }
    }

    pub fn on_price_update(&mut self, price: f64) -> Vec<GridOrder> {
        let mut new_orders = Vec::new();

        if let Some(ref mut det) = self.market_detector {
            det.update(price);
        }

        let level = self.price_to_level(price);
        if level < 0 || level >= self.config.grid_count as i32 {
            return new_orders;
        }
        let level = level as u32;

        if self.last_level >= 0 && level != self.last_level as u32 {
            let crossed: Vec<u32> = if level > self.last_level as u32 {
                (self.last_level as u32 + 1..=level).collect()
            } else {
                (level..self.last_level as u32).collect()
            };

            for grid_idx in crossed {
                let grid_price = self.level_to_price(grid_idx);
                let (side, is_mg) = if price < grid_price {
                    (OrderSide::SELL, false)
                } else {
                    (OrderSide::BUY, false)
                };

                let base_qty = self.config.quantity_per_grid;
                let qty = if let Some(ref mg) = self.martingale {
                    if side == OrderSide::BUY {
                        mg.calculate_quantity(base_qty, grid_idx)
                    } else {
                        self.positions.get(&grid_idx).map(|p| p.buy_quantity).unwrap_or(base_qty)
                    }
                } else {
                    base_qty
                };

                let order = if side == OrderSide::BUY {
                    GridOrder::new_buy(grid_price, qty, grid_idx, is_mg)
                } else {
                    GridOrder::new_sell(grid_price, qty, grid_idx, is_mg)
                };
                new_orders.push(order);
            }
        }

        self.last_level = level as i32;
        new_orders
    }

    pub fn on_order_filled(&mut self, order: &GridOrder) {
        if order.side == OrderSide::BUY {
            let ml = self.martingale.as_ref().map(|m| m.get_level(order.grid_level)).unwrap_or(0);
            let pos = GridPosition {
                grid_level: order.grid_level,
                buy_price: order.price,
                buy_quantity: order.quantity,
                sell_price: None,
                closed: false,
                martingale_level: ml,
            };
            self.positions.insert(order.grid_level, pos);
            if order.is_martingale {
                if let Some(ref mut mg) = self.martingale { mg.record_loss(order.grid_level); }
            }
        } else {
            if let Some(mut pos) = self.positions.remove(&order.grid_level) {
                pos.sell_price = Some(order.price);
                pos.closed = true;
                let pnl = (pos.sell_price.unwrap() - pos.buy_price) * pos.buy_quantity;
                self.realized_pnl += pnl;
                if let Some(ref mut mg) = self.martingale { mg.record_profit(order.grid_level); }
            }
        }
    }

    pub fn get_status(&self) -> GridStatus {
        let mut unrealized_pnl = 0.0;
        let filled_grids: Vec<u32> = self.positions.keys().copied().collect();
        let market_mode = self.market_detector.as_ref().map(|d| d.get_mode()).unwrap_or(MarketMode::Unknown);
        let martingale_level = self.martingale.as_ref().map(|m| m.max_level()).unwrap_or(0);
        GridStatus {
            total_pnl: self.realized_pnl + unrealized_pnl,
            realized_pnl: self.realized_pnl,
            unrealized_pnl,
            open_orders_count: self.pending_orders.len() as u32,
            martingale_level,
            market_mode,
            filled_grids,
            positions: self.positions.values().cloned().collect(),
        }
    }

    fn price_to_level(&self, price: f64) -> i32 {
        ((price - self.config.lower_price) / self.grid_spacing).floor() as i32
    }

    fn level_to_price(&self, level: u32) -> f64 {
        self.config.lower_price + level as f64 * self.grid_spacing
    }

    pub fn market_mode(&self) -> MarketMode {
        self.market_detector.as_ref().map(|d| d.get_mode()).unwrap_or(MarketMode::Unknown)
    }

    pub fn adjust_grid_spacing(&mut self, new_spacing: f64) {
        self.grid_spacing = new_spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> GridConfig {
        GridConfig {
            symbol: "BTCUSDT".into(),
            lower_price: 50000.0,
            upper_price: 60000.0,
            grid_count: 10,
            quantity_per_grid: 0.01,
            martingale: None,
            dynamic: None,
        }
    }

    #[test]
    fn test_initial_state() {
        let e = GridEngine::new(cfg());
        let s = e.get_status();
        assert_eq!(s.total_pnl, 0.0);
        assert_eq!(s.market_mode, MarketMode::Unknown);
    }

    #[test]
    fn test_price_to_level() {
        let e = GridEngine::new(cfg());
        // Grid spacing = 1000, level 5 = 55000
        assert_eq!(e.price_to_level(55000.0), 5);
        assert_eq!(e.price_to_level(50000.0), 0);
        assert_eq!(e.level_to_price(5), 55000.0);
    }

    #[test]
    fn test_price_crosses_grid() {
        let mut e = GridEngine::new(cfg());
        // 51000 = level 1, 52000 = level 2 (crossed 1)
        let _ = e.on_price_update(51000.0);
        let orders = e.on_price_update(52000.0);
        assert!(!orders.is_empty());
    }
}
