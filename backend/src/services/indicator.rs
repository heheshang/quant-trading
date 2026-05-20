//! KDJ Indicator Service
//!
//! KDJ = K=D=J indicator with golden cross / death cross / overbought / oversold signals
//!
//! Algorithm:
//!   RSV = (Close - Lowest(L, N)) / (Highest(H, N) - Lowest(L, N)) * 100
//!   K = SMA(RSV, M1)
//!   D = SMA(K, M2)
//!   J = 3*K - 2*D

use crate::models::schemas::{KdjBar, KdjSignal};
use crate::utils::error::AppError;

/// Input for KDJ calculation — single kline bar
#[derive(Debug, Clone)]
pub struct KlineInput {
    pub open_time: i64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// KDJ Calculator
#[derive(Debug, Clone)]
pub struct Kdjk {
    /// N: period for RSV (lowest/highest window), default 9
    pub n: usize,
    /// M1: smoothing period for K, default 3
    pub m1: usize,
    /// M2: smoothing period for D, default 3
    pub m2: usize,
}

impl Default for Kdjk {
    fn default() -> Self {
        Self { n: 9, m1: 3, m2: 3 }
    }
}

impl Kdjk {
    pub fn new(n: usize, m1: usize, m2: usize) -> Self {
        Self { n, m1, m2 }
    }

    /// Calculate KDJ for a slice of kline data.
    /// Returns one KdjBar per input bar (starting from index `n-1` where RSV is first defined).
    pub fn calculate(&self, klines: &[KlineInput]) -> Vec<KdjBar> {
        if klines.len() < self.n {
            return vec![];
        }

        let n = self.n;
        let m1 = self.m1;
        let m2 = self.m2;

        // --- Pass 1: Compute RSV for each valid window ---
        let rsv: Vec<f64> = klines
            .windows(n)
            .enumerate()
            .map(|(i, window)| {
                let low_min = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
                let high_max = window
                    .iter()
                    .map(|b| b.high)
                    .fold(f64::NEG_INFINITY, f64::max);
                let close = window.last().unwrap().close;
                if (high_max - low_min).abs() < f64::EPSILON {
                    50.0 // neutral RSV when high == low
                } else {
                    ((close - low_min) / (high_max - low_min)) * 100.0
                }
            })
            .collect();

        // rsv[0] corresponds to window ending at index n-1
        // So rsv[i] corresponds to klines[i + n - 1]

        // --- Pass 2: SMA with the "newest-first" convention used in traditional KDJ ---
        // Traditional KDJ uses a recursive smoothing:
        //   K_t = (M1-1)/M1 * K_{t-1} + 1/M1 * RSV_t   (exponential-like SMA)
        //   D_t = (M2-1)/M2 * D_{t-1} + 1/M2 * K_t
        // We use simple moving average (SMA) here for clarity:
        //   K[i] = avg(rsv[i-M1+1 .. i+1])
        //   D[i] = avg(K[i-M2+1 .. i+1])
        // This matches the standard pandas_ta ta.sma() pattern.

        let rsv_len = rsv.len();
        let k_len = rsv_len.saturating_sub(m1.saturating_sub(1));
        let d_len = k_len.saturating_sub(m2.saturating_sub(1));

        // K SMA
        let k_values: Vec<f64> = if m1 == 1 {
            rsv.clone()
        } else {
            (0..rsv_len)
                .map(|i| {
                    let start = i.saturating_sub(m1 - 1);
                    let slice = &rsv[start..=i];
                    slice.iter().sum::<f64>() / slice.len() as f64
                })
                .collect()
        };

        // D SMA
        let d_values: Vec<f64> = if m2 == 1 {
            k_values.clone()
        } else {
            (0..k_values.len())
                .map(|i| {
                    let start = i.saturating_sub(m2 - 1);
                    let slice = &k_values[start..=i];
                    slice.iter().sum::<f64>() / slice.len() as f64
                })
                .collect()
        };

        // --- Build output starting from d_len (which corresponds to the first valid bar) ---
        let first_valid_idx = (n - 1) + (m1 - 1) + (m2 - 1);
        if first_valid_idx >= klines.len() {
            return vec![];
        }

        let mut results = Vec::with_capacity(klines.len() - first_valid_idx);

        for i in first_valid_idx..klines.len() {
            // k_idx: position in k_values corresponding to klines[i]
            // klines[i] corresponds to rsv[i - (n - 1)]
            // k_values[i] is the SMA of rsv[max(0,i-m1+1)..=i]
            let rsv_idx = i - (n - 1);
            let k_idx = rsv_idx;
            let d_idx = k_idx.saturating_sub(m2 - 1);

            let k = if k_idx < k_values.len() {
                k_values[k_idx]
            } else {
                50.0
            };
            let d = if d_idx < d_values.len() {
                d_values[d_idx]
            } else {
                50.0
            };
            let j = 3.0 * k - 2.0 * d;

            let signal = detect_signal(&k_values, &d_values, k_idx, d_idx);

            results.push(KdjBar {
                open_time: klines[i].open_time,
                k: clamp(k, 0.0, 100.0),
                d: clamp(d, 0.0, 100.0),
                j: clamp(j, -100.0, 300.0),
                signal,
            });
        }

        results
    }
}

/// Detect signal based on K/D crossover and overbought/oversold levels.
fn detect_signal(k_values: &[f64], d_values: &[f64], k_idx: usize, d_idx: usize) -> KdjSignal {
    const OVERBOUGHT: f64 = 80.0;
    const OVERSOLD: f64 = 20.0;

    let k = if k_idx < k_values.len() {
        k_values[k_idx]
    } else {
        50.0
    };
    let d = if d_idx < d_values.len() {
        d_values[d_idx]
    } else {
        50.0
    };
    let prev_k = if k_idx > 0 { k_values[k_idx - 1] } else { k };
    let prev_d = if d_idx > 0 { d_values[d_idx - 1] } else { d };

    // Overbought / Oversold (only fire when both K and D cross the threshold together)
    if k > OVERBOUGHT && d > OVERBOUGHT {
        return KdjSignal::Overbought;
    }
    if k < OVERSOLD && d < OVERSOLD {
        return KdjSignal::Oversold;
    }

    // Golden cross: K crosses above D from below
    if prev_k <= prev_d && k > d {
        return KdjSignal::GoldenCross;
    }

    // Death cross: K crosses below D from above
    if prev_k >= prev_d && k < d {
        return KdjSignal::DeathCross;
    }

    KdjSignal::None
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// Compute KDJ for given klines — exposed as a public helper.
pub fn compute_kdj(klines: &[KlineInput], n: usize, m1: usize, m2: usize) -> Vec<KdjBar> {
    let calculator = Kdjk::new(n, m1, m2);
    calculator.calculate(klines)
}

/// Validate KDJ parameters
pub fn validate_kdj_params(n: usize, m1: usize, m2: usize) -> Result<(), AppError> {
    if n == 0 || n > 100 {
        return Err(AppError::Validation("n must be between 1 and 100".into()));
    }
    if m1 == 0 || m1 > 100 {
        return Err(AppError::Validation("m1 must be between 1 and 100".into()));
    }
    if m2 == 0 || m2 > 100 {
        return Err(AppError::Validation("m2 must be between 1 and 100".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_klines(prices: &[(f64, f64, f64)]) -> Vec<KlineInput> {
        // (high, low, close) — open_time increments by 1
        prices
            .iter()
            .enumerate()
            .map(|(i, (h, l, c))| KlineInput {
                open_time: (i as i64) * 60000,
                high: *h,
                low: *l,
                close: *c,
            })
            .collect()
    }

    #[test]
    fn test_kdj_basic() {
        // Simple ascending then descending price series (20 bars for n=9, m1=3, m2=3)
        let prices = vec![
            (10.0, 5.0, 6.0),
            (12.0, 5.0, 11.0),
            (14.0, 5.0, 13.0),
            (15.0, 5.0, 14.0),
            (16.0, 5.0, 15.0),
            (17.0, 5.0, 16.0),
            (18.0, 5.0, 17.0),
            (19.0, 5.0, 18.0),
            (20.0, 5.0, 19.0),
            (21.0, 5.0, 20.0),
            (22.0, 5.0, 21.0),
            (23.0, 5.0, 22.0),
            (24.0, 5.0, 23.0),
            (25.0, 5.0, 24.0),
            (26.0, 5.0, 25.0),
            (27.0, 5.0, 26.0),
            (28.0, 5.0, 27.0),
            (29.0, 5.0, 28.0),
            (30.0, 5.0, 29.0),
            (31.0, 5.0, 30.0), // 20th bar
        ];
        let klines = make_klines(&prices);
        let result = compute_kdj(&klines, 9, 3, 3);
        assert!(
            !result.is_empty(),
            "KDJ result should not be empty for 20 bars"
        );
        for bar in &result {
            assert!(bar.k >= 0.0 && bar.k <= 100.0);
            assert!(bar.d >= 0.0 && bar.d <= 100.0);
        }
    }

    #[test]
    fn test_kdj_flat_price() {
        // Flat price — RSV should be 50, K and D should converge to 50
        let prices: Vec<(f64, f64, f64)> = (0..20).map(|i| (100.0, 90.0, 95.0)).collect();
        let klines = make_klines(&prices);
        let result = compute_kdj(&klines, 9, 3, 3);
        assert!(!result.is_empty());
        // All K/D values should be close to 50
        for bar in &result {
            assert!((bar.k - 50.0).abs() < 30.0);
            assert!((bar.d - 50.0).abs() < 30.0);
        }
    }

    #[test]
    fn test_kdj_short_data() {
        let prices = vec![(10.0, 5.0, 6.0), (12.0, 5.0, 8.0), (14.0, 5.0, 10.0)];
        let klines = make_klines(&prices);
        // Not enough data for n=9
        let result = compute_kdj(&klines, 9, 3, 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_validate_params() {
        assert!(validate_kdj_params(9, 3, 3).is_ok());
        assert!(validate_kdj_params(0, 3, 3).is_err());
        assert!(validate_kdj_params(9, 0, 3).is_err());
        assert!(validate_kdj_params(9, 3, 0).is_err());
        assert!(validate_kdj_params(101, 3, 3).is_err());
    }
}
