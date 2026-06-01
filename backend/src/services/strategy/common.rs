use crate::models::backtest::{Kline, Signal};
use crate::models::schemas::ParameterDef;
use serde_json::Value;

// ============ StrategyTemplate Trait ============

pub trait StrategyTemplate: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn default_parameters(&self) -> Value;
    fn parameter_schema(&self) -> Vec<ParameterDef>;
    fn validate(&self, params: &Value) -> Result<(), String>;
    /// Generate trading signal for the given kline at current_idx.
    /// `klines` contains ALL bars loaded for the backtest (sorted by time ASC),
    /// and `current_idx` is the current bar index being processed.
    /// Implementations should look backwards from `current_idx` for indicator calculations.
    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal;
}

// ============ Technical Indicator Helpers ============

/// Simple Moving Average over lookback period.
pub(crate) fn sma(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![0.0; data.len()];
    }
    let mut result = vec![0.0; data.len()];
    let mut sum = 0.0;
    for i in 0..data.len() {
        sum += data[i];
        if i >= period {
            sum -= data[i - period];
        }
        if i >= period - 1 {
            result[i] = sum / period as f64;
        }
    }
    result
}

/// Exponential Moving Average.
pub(crate) fn ema(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![0.0; data.len()];
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut result = vec![0.0; data.len()];
    // First EMA = SMA
    let mut sum = 0.0;
    for v in data.iter().take(period.min(data.len())) {
        sum += *v;
    }
    let init_period = period.min(data.len());
    if init_period > 0 {
        result[init_period - 1] = sum / init_period as f64;
    }
    for i in init_period..data.len() {
        result[i] = (data[i] - result[i - 1]) * k + result[i - 1];
    }
    result
}

/// Standard deviation over lookback period.
pub(crate) fn stddev(data: &[f64], period: usize, mean: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; data.len()];
    for i in (period - 1)..data.len() {
        let start = i + 1 - period;
        let slice = &data[start..=i];
        let m = mean[i];
        let variance = slice.iter().map(|v| (v - m).powi(2)).sum::<f64>() / period as f64;
        result[i] = variance.sqrt();
    }
    result
}

/// True Range
pub(crate) fn true_range(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let mut tr = vec![0.0; close.len()];
    for i in 1..close.len() {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    tr
}

/// ATR (Average True Range)
pub(crate) fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let tr = true_range(high, low, close);
    ema(&tr, period)
}

/// RSI (Relative Strength Index)
pub(crate) fn rsi(data: &[f64], period: usize) -> Vec<f64> {
    let mut rsi_vals = vec![50.0; data.len()];
    if data.len() < period + 1 {
        return rsi_vals;
    }
    // First average gain/loss
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let diff = data[i] - data[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += diff.abs();
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    if avg_loss == 0.0 {
        rsi_vals[period] = 100.0;
    } else {
        let rs = avg_gain / avg_loss;
        rsi_vals[period] = 100.0 - 100.0 / (1.0 + rs);
    }
    // Subsequent values with smoothed EMA
    for i in (period + 1)..data.len() {
        let diff = data[i] - data[i - 1];
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { diff.abs() } else { 0.0 };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        if avg_loss == 0.0 {
            rsi_vals[i] = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            rsi_vals[i] = 100.0 - 100.0 / (1.0 + rs);
        }
    }
    rsi_vals
}

/// MACD: returns (macd_line, signal_line, histogram)
pub(crate) fn macd(data: &[f64], fast: usize, slow: usize, signal: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let fast_ema = ema(data, fast);
    let slow_ema = ema(data, slow);
    let macd_line: Vec<f64> = fast_ema
        .iter()
        .zip(slow_ema.iter())
        .map(|(f, s)| f - s)
        .collect();
    let signal_line = ema(&macd_line, signal);
    let hist: Vec<f64> = macd_line
        .iter()
        .zip(signal_line.iter())
        .map(|(m, s)| m - s)
        .collect();
    (macd_line, signal_line, hist)
}

/// Get close prices from klines
pub(crate) fn closes(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.close).collect()
}

pub(crate) fn highs(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.high).collect()
}

pub(crate) fn lows(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.low).collect()
}

