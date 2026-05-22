//! 价差计算引擎
//! 支持三种计算模式：ratio / percentage / zscore

use crate::services::arbitrage::types::*;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 价差计算引擎
pub struct SpreadCalculator {
    /// pair_id -> SpreadWindow
    pub windows: RwLock<HashMap<i32, SpreadWindow>>,
    window_size: usize,
}

impl SpreadCalculator {
    pub fn new(window_size: usize) -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            window_size,
        }
    }

    /// 计算价差（基于计算模式）
    pub fn calculate_spread(
        &self,
        pair_id: i32,
        price_a: Decimal,
        price_b: Decimal,
        mode: &SpreadCalculationMode,
    ) -> Result<LiveSpread, AppError> {
        if price_b == Decimal::ZERO {
            return Err(AppError::InvalidInput("price_b cannot be zero".into()));
        }

        let (spread, spread_pct) = match mode {
            SpreadCalculationMode::Ratio => {
                let ratio = price_a / price_b;
                let spread_pct = (ratio - Decimal::ONE) * Decimal::from(100);
                (ratio, spread_pct)
            }
            SpreadCalculationMode::Percentage | SpreadCalculationMode::ZScore => {
                let diff = price_a - price_b;
                let spread_pct = (diff / price_b) * Decimal::from(100);
                (diff, spread_pct)
            }
        };

        // Z-score 需要有足够的历史数据
        let z_score = if matches!(mode, SpreadCalculationMode::ZScore) {
            let mut windows = match self.windows.write() {
                Ok(guard) => guard,
                Err(_) => return Err(AppError::Internal("Lock poisoned".into())),
            };
            let window = windows
                .entry(pair_id)
                .or_insert_with(|| SpreadWindow::new(self.window_size));
            window.push(spread);
            window.z_score(spread)
        } else {
            None
        };

        Ok(LiveSpread {
            pair_id,
            price_a,
            price_b,
            spread,
            spread_pct,
            z_score,
            timestamp: chrono::Utc::now(),
        })
    }

    /// 重置某个 pair 的历史窗口
    pub fn reset_window(&self, pair_id: i32) {
        if let Ok(mut windows) = self.windows.write() {
            windows.remove(&pair_id);
        }
    }
}

impl Default for SpreadCalculator {
    fn default() -> Self {
        Self::new(20) // 默认 20 期窗口
    }
}

/// 工厂方法
pub fn create_spread_calculator(window_size: usize) -> Arc<SpreadCalculator> {
    Arc::new(SpreadCalculator::new(window_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_spread() {
        let calc = SpreadCalculator::new(20);

        // BTCUSDT_240625 / BTCUSDT_240926 = 0.98
        let result = calc
            .calculate_spread(
                1,
                Decimal::from(98000),
                Decimal::from(100000),
                &SpreadCalculationMode::Ratio,
            )
            .unwrap();

        assert_eq!(result.spread, Decimal::from(98) / Decimal::from(100));
        assert_eq!(result.spread_pct, -Decimal::from(2)); // -2%
    }

    #[test]
    fn test_percentage_spread() {
        let calc = SpreadCalculator::new(20);

        let result = calc
            .calculate_spread(
                1,
                Decimal::from(102),
                Decimal::from(100),
                &SpreadCalculationMode::Percentage,
            )
            .unwrap();

        assert_eq!(result.spread, Decimal::from(2));
        assert_eq!(result.spread_pct, Decimal::from(2)); // +2%
    }

    #[test]
    fn test_zscore_accumulates() {
        let calc = SpreadCalculator::new(5);

        for i in 1..=5 {
            calc.calculate_spread(
                1,
                Decimal::from(i * 100),
                Decimal::from(100),
                &SpreadCalculationMode::ZScore,
            )
            .unwrap();
        }

        // 第6个值应该能计算 z-score
        let result = calc
            .calculate_spread(
                1,
                Decimal::from(600),
                Decimal::from(100),
                &SpreadCalculationMode::ZScore,
            )
            .unwrap();

        assert!(result.z_score.is_some());
    }

    #[test]
    fn test_zero_price_b_fails() {
        let calc = SpreadCalculator::new(20);
        let result = calc.calculate_spread(
            1,
            Decimal::from(100),
            Decimal::ZERO,
            &SpreadCalculationMode::Ratio,
        );
        assert!(result.is_err());
    }
}
