//! Market Mode Detector — P3-F2
//!
//! Uses ADX and Bollinger Bandwidth to detect market regime:
//! - ADX < 25 → Ranging (good for grids)
//! - ADX >= 25 → Trending (grids risky)

use super::types::MarketMode;

/// Detects trending vs ranging market conditions
#[derive(Debug, Clone)]
pub struct MarketModeDetector {
    adx_period: u32,
    bb_period: u32,
    prices: Vec<f64>,
}

impl MarketModeDetector {
    pub fn new(adx_period: u32, bb_period: u32) -> Self {
        Self {
            adx_period,
            bb_period,
            prices: Vec::new(),
        }
    }

    pub fn update(&mut self, price: f64) {
        self.prices.push(price);
        let max_len = (self.adx_period * 3).max(self.bb_period * 2) as usize;
        if self.prices.len() > max_len {
            self.prices.remove(0);
        }
    }

    pub fn get_mode(&self) -> MarketMode {
        if self.prices.len() < self.adx_period as usize {
            return MarketMode::Unknown;
        }
        let adx = self.calculate_adx();
        let bb_bw = self.calculate_bb_bandwidth();
        if adx < 25.0 && bb_bw < 0.1 {
            MarketMode::Ranging
        } else if adx >= 25.0 {
            MarketMode::Trending
        } else {
            MarketMode::Ranging
        }
    }

    fn calculate_atr(&self) -> f64 {
        if self.prices.len() < 2 {
            return 0.0;
        }
        let period = self.adx_period as usize;
        let start = self.prices.len().saturating_sub(period);
        let slice = &self.prices[start..];
        let mut tr_sum = 0.0;
        for i in 1..slice.len() {
            tr_sum += (slice[i] - slice[i - 1]).abs();
        }
        tr_sum / slice.len().max(1) as f64
    }

    fn calculate_adx(&self) -> f64 {
        if self.prices.len() < self.adx_period as usize * 2 {
            return 0.0;
        }
        let atr = self.calculate_atr();
        if atr == 0.0 {
            return 0.0;
        }
        let period = self.adx_period as usize;
        let start = self.prices.len().saturating_sub(period * 2);
        let slice = &self.prices[start..];
        let mut plus_dm_sum = 0.0;
        let mut minus_dm_sum = 0.0;
        for i in 1..slice.len() {
            plus_dm_sum += (slice[i] - slice[i - 1]).max(0.0);
            minus_dm_sum += (slice[i - 1] - slice[i]).max(0.0);
        }
        let plus_di = if atr > 0.0 {
            (plus_dm_sum / atr) * 100.0 / period as f64
        } else {
            0.0
        };
        let minus_di = if atr > 0.0 {
            (minus_dm_sum / atr) * 100.0 / period as f64
        } else {
            0.0
        };
        if plus_di + minus_di > 0.0 {
            (plus_di - minus_di).abs() / (plus_di + minus_di) * 100.0
        } else {
            0.0
        }
    }

    fn calculate_bb_bandwidth(&self) -> f64 {
        let period = self.bb_period as usize;
        if self.prices.len() < period {
            return 0.0;
        }
        let start = self.prices.len() - period;
        let slice = &self.prices[start..];
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let std_dev =
            (slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / slice.len() as f64).sqrt();
        let upper = mean + 2.0 * std_dev;
        let lower = mean - 2.0 * std_dev;
        if mean > 0.0 {
            (upper - lower) / mean
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_insufficient_data() {
        let det = MarketModeDetector::new(14, 20);
        assert_eq!(det.get_mode(), MarketMode::Unknown);
    }

    #[test]
    fn test_ranging_low_volatility() {
        let mut det = MarketModeDetector::new(14, 20);
        for i in 0..30 {
            det.update(55000.0 + (i % 5) as f64 * 100.0);
        }
        assert_eq!(det.get_mode(), MarketMode::Ranging);
    }

    #[test]
    fn test_trending_upward() {
        let mut det = MarketModeDetector::new(14, 20);
        for i in 0..30 {
            det.update(50000.0 + i as f64 * 500.0);
        }
        assert_eq!(det.get_mode(), MarketMode::Trending);
    }
}
