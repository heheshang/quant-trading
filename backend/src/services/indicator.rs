//! KDJ Indicator Service
//!
//! KDJ = K=D=J indicator with golden cross / death cross / overbought / oversold signals
//!
//! Algorithm:
//!   RSV = (Close - Lowest(L, N)) / (Highest(H, N) - Lowest(L, N)) * 100
//!   K = SMA(RSV, M1)
//!   D = SMA(K, M2)
//!   J = 3*K - 2*D

use crate::models::schemas::{BollingerBar, KdjBar, KdjSignal, MaBar, MacdBar, RsiBar};
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
            .map(|window| {
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
        let _d_len = k_len.saturating_sub(m2.saturating_sub(1));

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

        for (i, _item) in klines.iter().enumerate().skip(first_valid_idx) {
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

/// Compute Simple Moving Average for given klines
pub fn compute_ma(klines: &[KlineInput], period: usize) -> Vec<MaBar> {
    if klines.len() < period {
        return vec![];
    }
    klines
        .windows(period)
        .map(|window| MaBar {
            open_time: window.last().unwrap().open_time,
            ma: window.iter().map(|b| b.close).sum::<f64>() / period as f64,
        })
        .collect()
}

/// Compute EMA (Exponential Moving Average)
pub fn compute_ema(klines: &[KlineInput], period: usize) -> Vec<MaBar> {
    if klines.len() < period || period == 0 {
        return vec![];
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let mut ema_values = Vec::with_capacity(klines.len());
    // First EMA is SMA of first `period` closes
    let first_sma = closes[..period].iter().sum::<f64>() / period as f64;
    ema_values.push(first_sma);
    for close in closes.iter().skip(period) {
        let prev_ema = *ema_values.last().unwrap();
        let new_ema = (*close - prev_ema) * multiplier + prev_ema;
        ema_values.push(new_ema);
    }
    // ema_values[0] aligns with klines[period-1]
    ema_values
        .iter()
        .enumerate()
        .map(|(i, &ema)| MaBar {
            open_time: klines[period - 1 + i].open_time,
            ma: ema,
        })
        .collect()
}

/// Compute ATR (Average True Range)
/// TR = max(high - low, |high - prev_close|, |low - prev_close|)
/// ATR = SMA(TR, period)
pub fn compute_atr(klines: &[KlineInput], period: usize) -> Vec<RsiBar> {
    if klines.len() < period + 1 || period == 0 {
        return vec![];
    }
    let mut tr_values = Vec::with_capacity(klines.len() - 1);
    for i in 1..klines.len() {
        let high_low = klines[i].high - klines[i].low;
        let high_prev = (klines[i].high - klines[i - 1].close).abs();
        let low_prev = (klines[i].low - klines[i - 1].close).abs();
        let tr = high_low.max(high_prev).max(low_prev);
        tr_values.push(tr);
    }
    let mut atr_values = Vec::with_capacity(tr_values.len() - period + 1);
    let first_atr = tr_values[..period].iter().sum::<f64>() / period as f64;
    atr_values.push(first_atr);
    for (_i, tr) in tr_values.iter().enumerate().skip(period) {
        let prev_atr = atr_values.last().unwrap();
        let new_atr = (prev_atr * (period - 1) as f64 + tr) / period as f64;
        atr_values.push(new_atr);
    }
    atr_values
        .into_iter()
        .skip(1)
        .enumerate()
        .map(|(i, atr)| RsiBar {
            open_time: klines[period + 1 + i].open_time,
            rsi: atr,
        })
        .collect()
}

/// Compute Stochastic Oscillator
/// %K = (Close - Lowest(L, k_period)) / (Highest(H, k_period) - Lowest(L, k_period)) * 100
/// %D = SMA(%K, d_period)
pub fn compute_stochastic(
    klines: &[KlineInput],
    k_period: usize,
    d_period: usize,
    smooth_k: usize,
) -> Vec<BollingerBar> {
    if klines.len() < k_period || k_period == 0 || d_period == 0 {
        return vec![];
    }
    // Compute raw %K values with sliding window
    let mut k_values: Vec<f64> = Vec::with_capacity(klines.len() - k_period + 1);
    for window in klines.windows(k_period) {
        let high = window
            .iter()
            .map(|b: &KlineInput| b.high)
            .fold(0.0_f64, |a, b| a.max(b));
        let low = window
            .iter()
            .map(|b: &KlineInput| b.low)
            .fold(f64::INFINITY, |a, b| a.min(b));
        let close = window.last().unwrap().close;
        let range = high - low;
        let k = if range.abs() < f64::EPSILON {
            50.0
        } else {
            (close - low) / range * 100.0
        };
        k_values.push(k);
    }
    // Apply additional smoothing if requested (not standard, but supported)
    let smooth_k = if smooth_k > 1 { smooth_k } else { 1 };
    if smooth_k > 1 && k_values.len() >= smooth_k {
        let mut smoothed = Vec::with_capacity(k_values.len() - smooth_k + 1);
        for window in k_values.windows(smooth_k) {
            let avg = window.iter().sum::<f64>() / smooth_k as f64;
            smoothed.push(avg);
        }
        k_values = smoothed;
    }
    // %D = SMA(%K, d_period)
    let mut d_values: Vec<f64> = Vec::with_capacity(k_values.len());
    for window in k_values.windows(d_period) {
        let d = window.iter().sum::<f64>() / d_period as f64;
        d_values.push(d);
    }
    // Return %K and %D as BollingerBar (k=middle, upper=k, lower=d)
    // Align: k_values[i] aligns with klines[k_period-1+i]
    // d_values[0] aligns with k_values[d_period-1] = klines[k_period-1+d_period-1]
    let start_idx = d_period - 1;
    k_values
        .into_iter()
        .skip(start_idx)
        .enumerate()
        .map(|(i, k)| BollingerBar {
            open_time: klines[k_period - 1 + start_idx + i].open_time,
            upper: k,
            middle: d_values.get(i).copied().unwrap_or(k),
            lower: 0.0, // not used for stochastic
        })
        .collect()
}

/// Compute RSI (Relative Strength Index)
pub fn compute_rsi(klines: &[KlineInput], period: usize) -> Vec<RsiBar> {
    if klines.len() < period + 1 || period == 0 {
        return vec![];
    }

    let mut gains = Vec::with_capacity(klines.len() - 1);
    let mut losses = Vec::with_capacity(klines.len() - 1);

    for window in klines.windows(2) {
        let diff = window[1].close - window[0].close;
        if diff > 0.0 {
            gains.push(diff);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(diff.abs());
        }
    }

    let mut avg_gain = gains[..period].iter().sum::<f64>() / period as f64;
    let mut avg_loss = losses[..period].iter().sum::<f64>() / period as f64;

    let mut results = Vec::with_capacity(klines.len() - period);
    // First RSI
    let rs = if avg_loss.abs() < f64::EPSILON {
        100.0
    } else {
        avg_gain / avg_loss
    };
    let rsi = if rs.abs() < f64::EPSILON {
        50.0
    } else {
        100.0 - 100.0 / (1.0 + rs)
    };
    results.push(RsiBar {
        open_time: klines[period].open_time,
        rsi,
    });

    for i in period..gains.len() {
        avg_gain = (avg_gain * (period - 1) as f64 + gains[i]) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + losses[i]) / period as f64;
        let rs = if avg_loss.abs() < f64::EPSILON {
            100.0
        } else {
            avg_gain / avg_loss
        };
        let rsi = if rs.abs() < f64::EPSILON {
            50.0
        } else {
            100.0 - 100.0 / (1.0 + rs)
        };
        results.push(RsiBar {
            open_time: klines[i + 1].open_time,
            rsi,
        });
    }

    results
}

/// Compute Bollinger Bands
/// middle = SMA(close, period)
/// upper = middle + std_dev * std(close, period)
/// lower = middle - std_dev * std(close, period)
pub fn compute_bollinger(klines: &[KlineInput], period: usize, std_dev: f64) -> Vec<BollingerBar> {
    if klines.len() < period || period == 0 || std_dev <= 0.0 {
        return vec![];
    }

    klines
        .windows(period)
        .map(|window| {
            let closes: Vec<f64> = window.iter().map(|b| b.close).collect();
            let middle = closes.iter().sum::<f64>() / period as f64;
            let variance = closes.iter().map(|c| (c - middle).powi(2)).sum::<f64>() / period as f64;
            let std = variance.sqrt();
            let open_time = window.last().unwrap().open_time;
            BollingerBar {
                open_time,
                upper: middle + std_dev * std,
                middle,
                lower: middle - std_dev * std,
            }
        })
        .collect()
}

/// MACD = EMA(fast) - EMA(slow)
/// Signal = EMA(MACD, signal_period)
/// Histogram = MACD - Signal
pub fn compute_macd(
    klines: &[KlineInput],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Vec<MacdBar> {
    if klines.len() < slow_period {
        return vec![];
    }

    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let multiplier = 2.0 / (fast_period as f64 + 1.0);

    // Compute fast EMA values
    let mut fast_ema_values: Vec<f64> = Vec::with_capacity(closes.len());
    let first_fast_sma = closes[..fast_period].iter().sum::<f64>() / fast_period as f64;
    fast_ema_values.push(first_fast_sma);
    for close in closes.iter().skip(fast_period) {
        let prev = *fast_ema_values.last().unwrap();
        fast_ema_values.push((*close - prev) * multiplier + prev);
    }

    let slow_multiplier = 2.0 / (slow_period as f64 + 1.0);
    let mut slow_ema_values: Vec<f64> = Vec::with_capacity(closes.len());
    let first_slow_sma = closes[..slow_period].iter().sum::<f64>() / slow_period as f64;
    slow_ema_values.push(first_slow_sma);
    for close in closes.iter().skip(slow_period) {
        let prev = *slow_ema_values.last().unwrap();
        slow_ema_values.push((*close - prev) * slow_multiplier + prev);
    }

    let offset = slow_period - fast_period;

    let mut macd_values = Vec::with_capacity(slow_ema_values.len());
    for (i, slow_val) in slow_ema_values.iter().enumerate() {
        let fast_idx = i + offset;
        if fast_idx < fast_ema_values.len() {
            macd_values.push(fast_ema_values[fast_idx] - slow_val);
        }
    }

    // Signal = EMA of macd_values
    let sig_multiplier = 2.0 / (signal_period as f64 + 1.0);
    let mut signal_values: Vec<f64> = Vec::with_capacity(macd_values.len());
    let first_sig_sma = macd_values[..signal_period].iter().sum::<f64>() / signal_period as f64;
    signal_values.push(first_sig_sma);
    for macd in macd_values.iter().skip(signal_period) {
        let prev = *signal_values.last().unwrap();
        signal_values.push((*macd - prev) * sig_multiplier + prev);
    }

    // Build output starting from when both MACD and signal are available
    let signal_offset = signal_period - 1;
    let first_valid = slow_period - 1 + signal_offset;

    if first_valid >= klines.len() {
        return vec![];
    }

    let last_valid_i = (slow_period - 1 + macd_values.len()).min(klines.len());
    let mut results = Vec::with_capacity(last_valid_i - first_valid);
    #[allow(clippy::needless_range_loop)]
    for i in first_valid..last_valid_i {
        let macd_idx = i - (slow_period - 1);
        let signal_idx = macd_idx - signal_offset;

        let macd = if macd_idx < macd_values.len() {
            macd_values[macd_idx]
        } else {
            continue;
        };
        let signal = if signal_idx < signal_values.len() {
            signal_values[signal_idx]
        } else {
            continue;
        };
        let histogram = macd - signal;

        results.push(MacdBar {
            open_time: klines[i].open_time,
            macd,
            signal,
            histogram,
        });
    }
    results
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
        let prices: Vec<(f64, f64, f64)> = (0..20).map(|_i| (100.0, 90.0, 95.0)).collect();
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
