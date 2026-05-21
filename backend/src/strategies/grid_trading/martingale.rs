//! Martingale Position Manager — P3-F2
//!
//! Tracks consecutive losses per grid and doubles position size.
//! Capped at max_consecutive to prevent runaway.

use super::types::MartingaleConfig;

/// Tracks martingale state per grid level
#[derive(Debug, Clone)]
pub struct MartingaleTracker {
    config: MartingaleConfig,
    consecutive_losses: Vec<u32>,
    max_level_reached: u32,
}

impl MartingaleTracker {
    pub fn new(config: MartingaleConfig) -> Self {
        Self { config, consecutive_losses: Vec::new(), max_level_reached: 0 }
    }

    pub fn initialize_grids(&mut self, grid_count: u32) {
        self.consecutive_losses = vec![0; grid_count as usize];
    }

    pub fn get_quantity_multiplier(&self, grid_level: u32) -> f64 {
        let level = self.consecutive_losses.get(grid_level as usize).copied().unwrap_or(0);
        self.config.multiplier.powi(level as i32)
    }

    pub fn calculate_quantity(&self, base_quantity: f64, grid_level: u32) -> f64 {
        base_quantity * self.get_quantity_multiplier(grid_level)
    }

    pub fn record_loss(&mut self, grid_level: u32) {
        if grid_level >= self.consecutive_losses.len() as u32 { return; }
        let current = &mut self.consecutive_losses[grid_level as usize];
        if *current < self.config.max_consecutive { *current += 1; }
        self.max_level_reached = self.max_level_reached.max(*current);
    }

    pub fn record_profit(&mut self, grid_level: u32) {
        if grid_level >= self.consecutive_losses.len() as u32 { return; }
        self.consecutive_losses[grid_level as usize] = 0;
    }

    pub fn is_at_max_level(&self, grid_level: u32) -> bool {
        self.consecutive_losses.get(grid_level as usize).copied().unwrap_or(0) >= self.config.max_consecutive
    }

    pub fn is_globally_at_max(&self) -> bool {
        self.max_level_reached >= self.config.max_consecutive
    }

    pub fn get_level(&self, grid_level: u32) -> u32 {
        self.consecutive_losses.get(grid_level as usize).copied().unwrap_or(0)
    }

    pub fn max_level(&self) -> u32 { self.max_level_reached }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> MartingaleConfig { MartingaleConfig { multiplier: 2.0, max_consecutive: 5 } }

    #[test]
    fn test_base_is_one() {
        let t = MartingaleTracker::new(cfg());
        assert_eq!(t.get_quantity_multiplier(0), 1.0);
    }

    #[test]
    fn test_loss_doubles() {
        let mut t = MartingaleTracker::new(cfg());
        t.initialize_grids(10);
        t.record_loss(0);
        assert_eq!(t.get_quantity_multiplier(0), 2.0);
        t.record_loss(0);
        assert_eq!(t.get_quantity_multiplier(0), 4.0);
    }

    #[test]
    fn test_profit_resets() {
        let mut t = MartingaleTracker::new(cfg());
        t.initialize_grids(10);
        t.record_loss(0);
        t.record_profit(0);
        assert_eq!(t.get_quantity_multiplier(0), 1.0);
    }

    #[test]
    fn test_max_capped() {
        let mut t = MartingaleTracker::new(cfg());
        t.initialize_grids(10);
        for _ in 0..10 { t.record_loss(0); }
        assert_eq!(t.get_level(0), 5);
        assert!(t.is_at_max_level(0));
    }

    #[test]
    fn test_calculate_quantity() {
        let mut t = MartingaleTracker::new(cfg());
        t.initialize_grids(10);
        assert_eq!(t.calculate_quantity(0.01, 0), 0.01);
        t.record_loss(0);
        assert_eq!(t.calculate_quantity(0.01, 0), 0.02);
    }
}
