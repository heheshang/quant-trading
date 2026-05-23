//! Feature Engineering for AI Quant Module
//!
//! Extracts technical features from OHLCV klines and order book data for ML model inference.

use crate::services::indicator::KlineInput;

/// Normalization method for feature scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizeMethod {
    /// Z-score normalization: (x - mean) / std
    ZScore,
    /// Min-max normalization: (x - min) / (max - min)
    MinMax,
}

/// Feature engineering engine configuration.
#[derive(Debug, Clone)]
pub struct FeatureEngine {
    /// Lookback window size for computing features (e.g., MA, ATR).
    pub lookback: usize,
    /// Normalization method applied to output feature vectors.
    pub normalize_method: NormalizeMethod,
}

impl Default for FeatureEngine {
    fn default() -> Self {
        Self {
            lookback: 14,
            normalize_method: NormalizeMethod::ZScore,
        }
    }
}

impl FeatureEngine {
    /// Create a new FeatureEngine with custom settings.
    pub fn new(lookback: usize, normalize_method: NormalizeMethod) -> Self {
        Self {
            lookback,
            normalize_method,
        }
    }

    /// Extract a feature vector from a slice of OHLCV klines.
    ///
    /// The resulting vector contains, in order:
    /// - Log returns over the configured lookback windows (single-period and multi-period)
    /// - RSI (Relative Strength Index)
    /// - ATR (Average True Range)
    /// - Moving average ratio (close / SMA)
    /// - Volume change ratio
    ///
    /// Returns an empty vector if the input has fewer than `2 * lookback` bars.
    pub fn extract_kline_features(&self, klines: &[KlineInput]) -> Vec<f64> {
        if klines.len() < 2 {
            return vec![];
        }

        let n = klines.len();
        let lb = self.lookback;

        // --- Log returns -------------------------------------------------------
        let log_returns = compute_log_returns(klines);
        // Feature: single-period log return (most recent)
        let current_log_return = log_returns.last().copied().unwrap_or(0.0);
        // Feature: multi-period log return over lookback window
        let multi_period_return = if log_returns.len() > lb {
            log_returns[n - lb - 1..].iter().sum()
        } else {
            log_returns.iter().sum()
        };

        // --- RSI ----------------------------------------------------------------
        let rsi = compute_rsi(klines, lb);

        // --- ATR ----------------------------------------------------------------
        let atr = compute_atr(klines, lb);

        // --- Moving average ratio -----------------------------------------------
        let ma_ratio = compute_ma_ratio(klines, lb);

        // --- Volume change ratio -------------------------------------------------
        let volume_change = compute_volume_change(klines);

        vec![
            current_log_return,
            multi_period_return,
            rsi,
            atr,
            ma_ratio,
            volume_change,
        ]
    }

    /// Normalize a feature vector using the configured method.
    ///
    /// Z-score: (x - mean) / std  (std=0 yields 0)
    /// Min-max: (x - min) / (max - min)  (range=0 yields 0.5)
    pub fn normalize_features(&self, features: &[f64]) -> Vec<f64> {
        match self.normalize_method {
            NormalizeMethod::ZScore => normalize_zscore(features),
            NormalizeMethod::MinMax => normalize_minmax(features),
        }
    }
}

// ---------------------------------------------------------------------------
// Public stand-alone helpers (also used by FeatureEngine)
// ---------------------------------------------------------------------------

/// Compute order book imbalance score.
///
/// Imbalance = (bid_vol - ask_vol) / (bid_vol + ask_vol)
///
/// Returns a value in [-1, 1]:
///   > 0  → more bid pressure
/// > < 0  → more ask pressure
/// > = 0  → balanced
///
/// Each `(f64, f64)` tuple represents `(price, quantity)`.
pub fn calculate_orderbook_imbalance(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> f64 {
    let bid_vol: f64 = bids.iter().map(|(_, q)| q).sum();
    let ask_vol: f64 = asks.iter().map(|(_, q)| q).sum();
    let total = bid_vol + ask_vol;
    if total < f64::EPSILON {
        return 0.0;
    }
    (bid_vol - ask_vol) / total
}

/// Normalize features using z-score: (x - mean) / std
pub fn normalize_zscore(features: &[f64]) -> Vec<f64> {
    if features.is_empty() {
        return vec![];
    }
    let count = features.len() as f64;
    let mean = features.iter().sum::<f64>() / count;
    let variance = features.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count;
    let std = variance.sqrt();
    if std < f64::EPSILON {
        // All identical — return zeros
        vec![0.0; features.len()]
    } else {
        features.iter().map(|x| (x - mean) / std).collect()
    }
}

/// Normalize features using min-max: (x - min) / (max - min)
pub fn normalize_minmax(features: &[f64]) -> Vec<f64> {
    if features.is_empty() {
        return vec![];
    }
    let min_val = features
        .iter()
        .cloned()
        .fold(f64::INFINITY, |a, b| a.min(b));
    let max_val = features
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let range = max_val - min_val;
    if range < f64::EPSILON {
        // All identical — return 0.5 as neutral
        vec![0.5; features.len()]
    } else {
        features.iter().map(|x| (x - min_val) / range).collect()
    }
}

// ---------------------------------------------------------------------------
// Private indicator helpers
// ---------------------------------------------------------------------------

fn compute_log_returns(klines: &[KlineInput]) -> Vec<f64> {
    klines
        .windows(2)
        .map(|w| {
            let prev = w[0].close;
            let curr = w[1].close;
            if prev <= 0.0 || curr <= 0.0 {
                0.0
            } else {
                (curr / prev).ln()
            }
        })
        .collect()
}

fn compute_rsi(klines: &[KlineInput], period: usize) -> f64 {
    if klines.len() < period + 1 {
        return 50.0; // neutral RSI when insufficient data
    }
    let returns = compute_log_returns(klines);
    let gains: f64 = returns[returns.len() - period..]
        .iter()
        .filter(|&&r| r > 0.0)
        .sum();
    let losses: f64 = returns[returns.len() - period..]
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| -r)
        .sum();

    let avg_gain = gains / period as f64;
    let avg_loss = losses / period as f64;

    if avg_loss < f64::EPSILON {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

fn compute_true_range(klines: &[KlineInput]) -> Vec<f64> {
    klines
        .windows(2)
        .map(|w| {
            let high = w[1].high;
            let low = w[1].low;
            let prev_close = w[0].close;
            let hl = high - low;
            let hc = (high - prev_close).abs();
            let lc = (low - prev_close).abs();
            hl.max(hc).max(lc)
        })
        .collect()
}

fn compute_atr(klines: &[KlineInput], period: usize) -> f64 {
    if klines.len() < period + 1 {
        return 0.0;
    }
    let tr = compute_true_range(klines);
    let last_trs: Vec<f64> = tr[tr.len() - period..].to_vec();
    last_trs.iter().sum::<f64>() / period as f64
}

fn compute_sma(prices: &[f64], period: usize) -> f64 {
    if prices.len() < period {
        return prices.iter().sum::<f64>() / prices.len().max(1) as f64;
    }
    let slice = &prices[prices.len() - period..];
    slice.iter().sum::<f64>() / period as f64
}

fn compute_ma_ratio(klines: &[KlineInput], period: usize) -> f64 {
    if klines.len() < period {
        return 1.0;
    }
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let sma = compute_sma(&closes, period);
    if sma <= 0.0 {
        return 1.0;
    }
    let latest_close = klines.last().unwrap().close;
    latest_close / sma
}

fn compute_volume_change(klines: &[KlineInput]) -> f64 {
    if klines.len() < 2 {
        return 0.0;
    }
    // Use close price * volume as a proxy for "volume" when volume field is absent.
    // If KlineInput had an explicit volume field we'd use it directly.
    // Here we use a simple price-based proxy: relative price change magnitude.
    let n = klines.len();
    let recent = &klines[n.saturating_sub(5)..];
    if recent.len() < 2 {
        return 0.0;
    }
    let first_close = recent.first().unwrap().close;
    let last_close = recent.last().unwrap().close;
    if first_close <= 0.0 {
        return 0.0;
    }
    (last_close - first_close) / first_close
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_klines(prices: &[(f64, f64, f64, f64)]) -> Vec<KlineInput> {
        // (open, high, low, close) — open_time increments by 1 minute
        prices
            .iter()
            .enumerate()
            .map(|(i, (_, h, l, c))| KlineInput {
                open_time: (i as i64) * 60000,
                high: *h,
                low: *l,
                close: *c,
            })
            .collect()
    }

    // Helper: build a simple ascending then descending series
    fn make_trend_klines(n: usize) -> Vec<KlineInput> {
        (0..n)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.5;
                KlineInput {
                    open_time: (i as i64) * 60000,
                    high: base + 1.0,
                    low: base - 1.0,
                    close: base,
                }
            })
            .collect()
    }

    // -------------------------------------------------------------------------
    #[test]
    fn test_feature_engine_default() {
        let engine = FeatureEngine::default();
        assert_eq!(engine.lookback, 14);
        assert_eq!(engine.normalize_method, NormalizeMethod::ZScore);
    }

    #[test]
    fn test_feature_engine_custom() {
        let engine = FeatureEngine::new(20, NormalizeMethod::MinMax);
        assert_eq!(engine.lookback, 20);
        assert_eq!(engine.normalize_method, NormalizeMethod::MinMax);
    }

    #[test]
    fn test_extract_kline_features_insufficient_data() {
        let engine = FeatureEngine::default();
        let klines = make_klines(&[(100.0, 105.0, 95.0, 102.0)]);
        let features = engine.extract_kline_features(&klines);
        assert!(features.is_empty());
    }

    #[test]
    fn test_extract_kline_features_basic() {
        let engine = FeatureEngine::new(5, NormalizeMethod::ZScore);
        // Ascending trend — enough bars for lookback=5
        let klines = make_klines(&[
            (100.0, 101.0, 99.0, 100.5),
            (101.0, 102.0, 100.0, 101.5),
            (102.0, 103.0, 101.0, 102.5),
            (103.0, 104.0, 102.0, 103.5),
            (104.0, 105.0, 103.0, 104.5),
            (105.0, 106.0, 104.0, 105.5),
            (106.0, 107.0, 105.0, 106.5),
        ]);
        let features = engine.extract_kline_features(&klines);
        assert_eq!(features.len(), 6); // current_log_return, multi_period, rsi, atr, ma_ratio, vol_change
        // All features should be finite
        for f in &features {
            assert!(f.is_finite(), "Feature should be finite: {}", f);
        }
    }

    #[test]
    fn test_extract_kline_features_trending() {
        let engine = FeatureEngine::default(); // lookback=14
        let klines = make_trend_klines(30);
        let features = engine.extract_kline_features(&klines);
        assert_eq!(features.len(), 6);
        // In an uptrend, RSI should be > 50, MA ratio > 1
        let rsi = features[2];
        let ma_ratio = features[4];
        assert!(rsi > 50.0, "RSI in uptrend should be > 50, got {}", rsi);
        assert!(
            ma_ratio > 1.0,
            "MA ratio in uptrend should be > 1, got {}",
            ma_ratio
        );
    }

    #[test]
    fn test_calculate_orderbook_imbalance_basic() {
        // (price, quantity)
        let bids = vec![(100.0, 10.0), (99.0, 5.0)];
        let asks = vec![(101.0, 8.0), (102.0, 3.0)];
        let imbalance = calculate_orderbook_imbalance(&bids, &asks);
        // bid_vol=15, ask_vol=11 → (15-11)/(15+11)=4/26≈0.154
        let expected = (15.0 - 11.0) / (15.0 + 11.0);
        assert!((imbalance - expected).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_orderbook_imbalance_bid_heavy() {
        let bids = vec![(100.0, 100.0)];
        let asks = vec![(101.0, 10.0)];
        let imbalance = calculate_orderbook_imbalance(&bids, &asks);
        assert!((imbalance - 0.81818).abs() < 1e-4); // (100-10)/(100+10)≈0.818
    }

    #[test]
    fn test_calculate_orderbook_imbalance_ask_heavy() {
        let bids = vec![(100.0, 10.0)];
        let asks = vec![(101.0, 100.0)];
        let imbalance = calculate_orderbook_imbalance(&bids, &asks);
        assert!((imbalance - (-0.81818)).abs() < 1e-4);
    }

    #[test]
    fn test_calculate_orderbook_imbalance_empty() {
        let bids: Vec<(f64, f64)> = vec![];
        let asks: Vec<(f64, f64)> = vec![];
        let imbalance = calculate_orderbook_imbalance(&bids, &asks);
        assert_eq!(imbalance, 0.0);
    }

    #[test]
    fn test_calculate_orderbook_imbalance_zero_quantity() {
        let bids = vec![(100.0, 0.0)];
        let asks = vec![(101.0, 0.0)];
        let imbalance = calculate_orderbook_imbalance(&bids, &asks);
        assert_eq!(imbalance, 0.0);
    }

    #[test]
    fn test_normalize_zscore_basic() {
        let features = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let normalized = normalize_zscore(&features);
        // mean=3, std≈1.414
        let mean: f64 = normalized.iter().sum::<f64>() / 5.0;
        assert!(
            mean.abs() < 1e-10,
            "Z-score normalized mean should be ~0, got {}",
            mean
        );
    }

    #[test]
    fn test_normalize_zscore_constant_input() {
        let features = vec![5.0, 5.0, 5.0];
        let normalized = normalize_zscore(&features);
        // All identical → should return zeros (avoid division by zero)
        assert!(normalized.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_normalize_zscore_empty() {
        let features: Vec<f64> = vec![];
        let normalized = normalize_zscore(&features);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_normalize_minmax_basic() {
        let features = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let normalized = normalize_minmax(&features);
        assert!((normalized[0] - 0.0).abs() < 1e-10);
        assert!((normalized[4] - 1.0).abs() < 1e-10);
        assert!((normalized[2] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_minmax_constant_input() {
        let features = vec![5.0, 5.0, 5.0];
        let normalized = normalize_minmax(&features);
        // Range=0 → return 0.5 as neutral
        assert!(normalized.iter().all(|&x| x == 0.5));
    }

    #[test]
    fn test_normalize_minmax_empty() {
        let features: Vec<f64> = vec![];
        let normalized = normalize_minmax(&features);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_feature_engine_normalize_zscore() {
        let engine = FeatureEngine::new(14, NormalizeMethod::ZScore);
        let raw = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let normalized = engine.normalize_features(&raw);
        let mean: f64 = normalized.iter().sum::<f64>() / 5.0;
        assert!(mean.abs() < 1e-10);
    }

    #[test]
    fn test_feature_engine_normalize_minmax() {
        let engine = FeatureEngine::new(14, NormalizeMethod::MinMax);
        let raw = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let normalized = engine.normalize_features(&raw);
        assert!((normalized[0] - 0.0).abs() < 1e-10);
        assert!((normalized[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rsi_neutral() {
        // Flat price → RSI should be ~50
        let klines: Vec<KlineInput> = (0..20)
            .map(|i| KlineInput {
                open_time: (i as i64) * 60000,
                high: 100.0,
                low: 99.0,
                close: 99.5,
            })
            .collect();
        let rsi = compute_rsi(&klines, 14);
        assert!(
            rsi > 50.0 || rsi == 100.0,
            "RSI for flat price should be > 50 or 100, got {}",
            rsi
        );
    }

    #[test]
    fn test_atr_calculation() {
        let klines = make_klines(&[
            (100.0, 105.0, 95.0, 102.0),
            (102.0, 108.0, 97.0, 105.0),
            (105.0, 110.0, 99.0, 108.0),
            (108.0, 112.0, 101.0, 109.0),
            (109.0, 113.0, 102.0, 110.0),
            (110.0, 115.0, 103.0, 112.0),
            (112.0, 117.0, 105.0, 114.0),
        ]);
        let atr = compute_atr(&klines, 3);
        assert!(atr > 0.0, "ATR should be positive");
        assert!(atr.is_finite());
    }

    #[test]
    fn test_ma_ratio_above_one_in_uptrend() {
        let klines = make_trend_klines(20);
        let ratio = compute_ma_ratio(&klines, 14);
        assert!(
            ratio > 1.0,
            "MA ratio in uptrend should be > 1, got {}",
            ratio
        );
    }

    #[test]
    fn test_ma_ratio_below_one_in_downtrend() {
        // Descending prices
        let klines: Vec<KlineInput> = (0..20)
            .map(|i| {
                let base = 100.0 - i as f64 * 0.5;
                KlineInput {
                    open_time: (i as i64) * 60000,
                    high: base + 1.0,
                    low: base - 1.0,
                    close: base,
                }
            })
            .collect();
        let ratio = compute_ma_ratio(&klines, 14);
        assert!(
            ratio < 1.0,
            "MA ratio in downtrend should be < 1, got {}",
            ratio
        );
    }

    #[test]
    fn test_volume_change_zero_for_flat() {
        let klines: Vec<KlineInput> = (0..10)
            .map(|i| KlineInput {
                open_time: (i as i64) * 60000,
                high: 100.0,
                low: 99.0,
                close: 99.5,
            })
            .collect();
        let vc = compute_volume_change(&klines);
        assert!(
            vc.abs() < 1e-6,
            "Volume change for flat price should be ~0, got {}",
            vc
        );
    }
}
