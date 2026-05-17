use crate::db::strategy;
use crate::models::backtest::{Kline, Signal};
use crate::models::schemas::{
    BulkUpdateStatusRequest, CreateStrategyRequest, ImportBatchRequest, ImportBatchResponse,
    ImportStrategyRequest, PaginatedResponse, PaginationParams, ParameterDef, StrategyResponse,
    TemplateInfo, UpdateStrategyRequest,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde_json::Value;
use uuid::Uuid;

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
fn sma(data: &[f64], period: usize) -> Vec<f64> {
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
fn ema(data: &[f64], period: usize) -> Vec<f64> {
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
fn stddev(data: &[f64], period: usize, mean: &[f64]) -> Vec<f64> {
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
fn true_range(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
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
fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let tr = true_range(high, low, close);
    ema(&tr, period)
}

/// RSI (Relative Strength Index)
fn rsi(data: &[f64], period: usize) -> Vec<f64> {
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
fn macd(data: &[f64], fast: usize, slow: usize, signal: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
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
fn closes(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.close).collect()
}

fn highs(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.high).collect()
}

fn lows(klines: &[Kline]) -> Vec<f64> {
    klines.iter().map(|k| k.low).collect()
}

// ============ Template Implementations ============

pub struct MaCrossoverTemplate;
pub struct TripleMaTemplate;
pub struct MacdTemplate;
pub struct BollingerTemplate;
pub struct RsiTemplate;
pub struct KeltnerTemplate;
pub struct AtrStopTemplate;
pub struct MeanReversionTemplate;
pub struct IchimokuTemplate;
pub struct DoubleBollingerTemplate;

macro_rules! int_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {
        ParameterDef {
            name: $name.to_string(),
            param_type: "integer".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: Value::Number(($default).into()),
            min: Some(Value::Number(($min).into())),
            max: Some(Value::Number(($max).into())),
            options: None,
        }
    };
}

macro_rules! float_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {
        ParameterDef {
            name: $name.to_string(),
            param_type: "float".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: serde_json::json!($default),
            min: Some(serde_json::json!($min)),
            max: Some(serde_json::json!($max)),
            options: None,
        }
    };
}

fn get_i64(params: &Value, key: &str) -> Option<i64> {
    params.get(key)?.as_i64()
}

fn get_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key)?.as_f64()
}

// ========== 1. MaCrossover ==========

impl StrategyTemplate for MaCrossoverTemplate {
    fn id(&self) -> &str {
        "ma_crossover"
    }
    fn name(&self) -> &str {
        "MA Crossover"
    }
    fn description(&self) -> &str {
        "Moving average crossover strategy using fast and slow MAs"
    }
    fn category(&self) -> &str {
        "trend"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"fast_period": 10, "slow_period": 30})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!(
                "fast_period",
                "Fast Period",
                "Fast moving average period",
                10,
                5,
                50
            ),
            int_param!(
                "slow_period",
                "Slow Period",
                "Slow moving average period",
                30,
                10,
                200
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let fast = get_i64(params, "fast_period").ok_or("fast_period is required")?;
        let slow = get_i64(params, "slow_period").ok_or("slow_period is required")?;
        if fast < 5 {
            return Err("fast_period must be >= 5".into());
        }
        if fast > 50 {
            return Err("fast_period must be <= 50".into());
        }
        if slow < 10 {
            return Err("slow_period must be >= 10".into());
        }
        if slow > 200 {
            return Err("slow_period must be <= 200".into());
        }
        if fast >= slow {
            return Err("fast_period must be less than slow_period".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let fast = get_i64(params, "fast_period").unwrap_or(10) as usize;
        let slow = get_i64(params, "slow_period").unwrap_or(30) as usize;
        if current_idx < slow {
            return Signal::Hold;
        }
        let close = closes(klines);
        let fast_ma = sma(&close, fast);
        let slow_ma = sma(&close, slow);
        let prev_fast = fast_ma[current_idx - 1];
        let prev_slow = slow_ma[current_idx - 1];
        let curr_fast = fast_ma[current_idx];
        let curr_slow = slow_ma[current_idx];
        if prev_fast <= prev_slow && curr_fast > curr_slow {
            Signal::Buy { quantity_pct: 1.0 }
        } else if prev_fast >= prev_slow && curr_fast < curr_slow {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 2. TripleMA ==========

impl StrategyTemplate for TripleMaTemplate {
    fn id(&self) -> &str {
        "triple_ma"
    }
    fn name(&self) -> &str {
        "Triple MA"
    }
    fn description(&self) -> &str {
        "Triple moving average crossover strategy"
    }
    fn category(&self) -> &str {
        "trend"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"short": 9, "medium": 21, "long": 50})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("short", "Short Period", "Short MA period", 9, 5, 20),
            int_param!("medium", "Medium Period", "Medium MA period", 21, 10, 50),
            int_param!("long", "Long Period", "Long MA period", 50, 20, 200),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let short = get_i64(params, "short").ok_or("short is required")?;
        let medium = get_i64(params, "medium").ok_or("medium is required")?;
        let long = get_i64(params, "long").ok_or("long is required")?;
        if short < 5 {
            return Err("short must be >= 5".into());
        }
        if short > 20 {
            return Err("short must be <= 20".into());
        }
        if medium < 10 {
            return Err("medium must be >= 10".into());
        }
        if medium > 50 {
            return Err("medium must be <= 50".into());
        }
        if long < 20 {
            return Err("long must be >= 20".into());
        }
        if long > 200 {
            return Err("long must be <= 200".into());
        }
        if !(short < medium && medium < long) {
            return Err("must satisfy short < medium < long".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let short_p = get_i64(params, "short").unwrap_or(9) as usize;
        let medium_p = get_i64(params, "medium").unwrap_or(21) as usize;
        let long_p = get_i64(params, "long").unwrap_or(50) as usize;
        if current_idx < long_p {
            return Signal::Hold;
        }
        let close = closes(klines);
        let short_ma = sma(&close, short_p);
        let medium_ma = sma(&close, medium_p);
        let long_ma = sma(&close, long_p);
        let (s, m, l) = (
            short_ma[current_idx],
            medium_ma[current_idx],
            long_ma[current_idx],
        );
        let (ps, pm, pl) = (
            short_ma[current_idx - 1],
            medium_ma[current_idx - 1],
            long_ma[current_idx - 1],
        );
        // Bullish: short > medium > long (stacked)
        if s > m && m > l && !(ps > pm && pm > pl) {
            Signal::Buy { quantity_pct: 1.0 }
        } else if s < m && m < l && !(ps < pm && pm < pl) {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 3. MACD ==========

impl StrategyTemplate for MacdTemplate {
    fn id(&self) -> &str {
        "macd"
    }
    fn name(&self) -> &str {
        "MACD"
    }
    fn description(&self) -> &str {
        "MACD (Moving Average Convergence Divergence) strategy"
    }
    fn category(&self) -> &str {
        "trend"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"fast": 12, "slow": 26, "signal": 9})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("fast", "Fast Period", "Fast EMA period", 12, 2, 50),
            int_param!("slow", "Slow Period", "Slow EMA period", 26, 5, 100),
            int_param!("signal", "Signal Period", "Signal line period", 9, 2, 30),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let fast = get_i64(params, "fast").ok_or("fast is required")?;
        let slow = get_i64(params, "slow").ok_or("slow is required")?;
        let signal = get_i64(params, "signal").ok_or("signal is required")?;
        if fast < 2 {
            return Err("fast must be >= 2".into());
        }
        if fast > 50 {
            return Err("fast must be <= 50".into());
        }
        if slow < 5 {
            return Err("slow must be >= 5".into());
        }
        if slow > 100 {
            return Err("slow must be <= 100".into());
        }
        if signal < 2 {
            return Err("signal must be >= 2".into());
        }
        if signal > 30 {
            return Err("signal must be <= 30".into());
        }
        if fast >= slow {
            return Err("fast must be less than slow".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let fast = get_i64(params, "fast").unwrap_or(12) as usize;
        let slow = get_i64(params, "slow").unwrap_or(26) as usize;
        let signal_p = get_i64(params, "signal").unwrap_or(9) as usize;
        let min_bars = slow + signal_p;
        if current_idx < min_bars {
            return Signal::Hold;
        }
        let close = closes(klines);
        let (_, _, hist) = macd(&close, fast, slow, signal_p);
        let curr_hist = hist[current_idx];
        let prev_hist = hist[current_idx - 1];
        // Histogram crossover: from negative to positive = buy signal
        if prev_hist <= 0.0 && curr_hist > 0.0 {
            Signal::Buy { quantity_pct: 1.0 }
        } else if prev_hist >= 0.0 && curr_hist < 0.0 {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 4. Bollinger ==========

impl StrategyTemplate for BollingerTemplate {
    fn id(&self) -> &str {
        "bollinger"
    }
    fn name(&self) -> &str {
        "Bollinger Bands"
    }
    fn description(&self) -> &str {
        "Bollinger Bands mean reversion strategy"
    }
    fn category(&self) -> &str {
        "volatility"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "std_dev": 2.0})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Bollinger Bands period", 20, 5, 100),
            float_param!(
                "std_dev",
                "Std Dev",
                "Standard deviation multiplier",
                2.0,
                1.0,
                4.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let std_dev = get_f64(params, "std_dev").ok_or("std_dev is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 100 {
            return Err("period must be <= 100".into());
        }
        if std_dev < 1.0 {
            return Err("std_dev must be >= 1.0".into());
        }
        if std_dev > 4.0 {
            return Err("std_dev must be <= 4.0".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let period = get_i64(params, "period").unwrap_or(20) as usize;
        let mult = get_f64(params, "std_dev").unwrap_or(2.0);
        if current_idx < period {
            return Signal::Hold;
        }
        let close = closes(klines);
        let ma = sma(&close, period);
        let sd = stddev(&close, period, &ma);
        let upper = ma[current_idx] + mult * sd[current_idx];
        let lower = ma[current_idx] - mult * sd[current_idx];
        let price = close[current_idx];
        let prev_price = close[current_idx - 1];
        // Price crosses below lower band → buy (oversold)
        if prev_price >= lower && price < lower {
            Signal::Buy { quantity_pct: 1.0 }
        // Price crosses above upper band → sell (overbought)
        } else if prev_price <= upper && price > upper {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 5. RSI ==========

impl StrategyTemplate for RsiTemplate {
    fn id(&self) -> &str {
        "rsi"
    }
    fn name(&self) -> &str {
        "RSI"
    }
    fn description(&self) -> &str {
        "Relative Strength Index mean reversion strategy"
    }
    fn category(&self) -> &str {
        "mean_reversion"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 14, "overbought": 75, "oversold": 25})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "RSI period", 14, 5, 50),
            float_param!(
                "overbought",
                "Overbought",
                "Overbought threshold",
                75.0,
                65.0,
                90.0
            ),
            float_param!(
                "oversold",
                "Oversold",
                "Oversold threshold",
                25.0,
                10.0,
                35.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let overbought = get_f64(params, "overbought").ok_or("overbought is required")?;
        let oversold = get_f64(params, "oversold").ok_or("oversold is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 50 {
            return Err("period must be <= 50".into());
        }
        if overbought < 65.0 {
            return Err("overbought must be >= 65".into());
        }
        if overbought > 90.0 {
            return Err("overbought must be <= 90".into());
        }
        if oversold < 10.0 {
            return Err("oversold must be >= 10".into());
        }
        if oversold > 35.0 {
            return Err("oversold must be <= 35".into());
        }
        if oversold >= overbought {
            return Err("oversold must be less than overbought".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let period = get_i64(params, "period").unwrap_or(14) as usize;
        let overbought = get_f64(params, "overbought").unwrap_or(75.0);
        let oversold = get_f64(params, "oversold").unwrap_or(25.0);
        if current_idx < period + 1 {
            return Signal::Hold;
        }
        let close = closes(klines);
        let rsi_vals = rsi(&close, period);
        let curr_rsi = rsi_vals[current_idx];
        let prev_rsi = rsi_vals[current_idx - 1];
        // RSI crosses below oversold → buy
        if prev_rsi > oversold && curr_rsi <= oversold {
            Signal::Buy { quantity_pct: 1.0 }
        // RSI crosses above overbought → sell
        } else if prev_rsi < overbought && curr_rsi >= overbought {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 6. Keltner ==========

impl StrategyTemplate for KeltnerTemplate {
    fn id(&self) -> &str {
        "keltner"
    }
    fn name(&self) -> &str {
        "Keltner Channels"
    }
    fn description(&self) -> &str {
        "Keltner Channels volatility strategy"
    }
    fn category(&self) -> &str {
        "volatility"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "atr_multiplier": 1.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Keltner period", 20, 5, 100),
            float_param!(
                "atr_multiplier",
                "ATR Multiplier",
                "ATR multiplier",
                1.5,
                1.0,
                3.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let atr = get_f64(params, "atr_multiplier").ok_or("atr_multiplier is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 100 {
            return Err("period must be <= 100".into());
        }
        if atr < 1.0 {
            return Err("atr_multiplier must be >= 1.0".into());
        }
        if atr > 3.0 {
            return Err("atr_multiplier must be <= 3.0".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let period = get_i64(params, "period").unwrap_or(20) as usize;
        let mult = get_f64(params, "atr_multiplier").unwrap_or(1.5);
        if current_idx < period {
            return Signal::Hold;
        }
        let close = closes(klines);
        let high = highs(klines);
        let low = lows(klines);
        let ma = sma(&close, period);
        let atr_vals = atr(&high, &low, &close, period);
        let upper = ma[current_idx] + mult * atr_vals[current_idx];
        let lower = ma[current_idx] - mult * atr_vals[current_idx];
        let price = close[current_idx];
        let prev_price = close[current_idx - 1];
        // Price breaks above upper → buy (momentum)
        if prev_price <= upper && price > upper {
            Signal::Buy { quantity_pct: 1.0 }
        // Price breaks below lower → sell
        } else if prev_price >= lower && price < lower {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 7. ATR Stop ==========

impl StrategyTemplate for AtrStopTemplate {
    fn id(&self) -> &str {
        "atr_stop"
    }
    fn name(&self) -> &str {
        "ATR Stop Loss"
    }
    fn description(&self) -> &str {
        "Average True Range based stop loss strategy"
    }
    fn category(&self) -> &str {
        "volatility"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 14, "multiplier": 3.0})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "ATR period", 14, 5, 50),
            float_param!(
                "multiplier",
                "Multiplier",
                "ATR multiplier for stop distance",
                3.0,
                1.0,
                5.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let mult = get_f64(params, "multiplier").ok_or("multiplier is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 50 {
            return Err("period must be <= 50".into());
        }
        if mult < 1.0 {
            return Err("multiplier must be >= 1.0".into());
        }
        if mult > 5.0 {
            return Err("multiplier must be <= 5.0".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        // ATR Stop Loss: long when price > MA + ATR*mult, short when price < MA - ATR*mult
        let period = get_i64(params, "period").unwrap_or(14) as usize;
        let mult = get_f64(params, "multiplier").unwrap_or(3.0);
        if current_idx < period {
            return Signal::Hold;
        }
        let close = closes(klines);
        let high = highs(klines);
        let low = lows(klines);
        let ma = sma(&close, period);
        let atr_vals = atr(&high, &low, &close, period);
        let upper = ma[current_idx] + mult * atr_vals[current_idx];
        let lower = ma[current_idx] - mult * atr_vals[current_idx];
        let price = close[current_idx];
        let prev_price = close[current_idx - 1];
        if prev_price <= lower && price > lower {
            Signal::Buy { quantity_pct: 1.0 }
        } else if prev_price >= upper && price < upper {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 8. Mean Reversion ==========

impl StrategyTemplate for MeanReversionTemplate {
    fn id(&self) -> &str {
        "mean_reversion"
    }
    fn name(&self) -> &str {
        "Mean Reversion"
    }
    fn description(&self) -> &str {
        "Statistical mean reversion strategy using standard deviation bands"
    }
    fn category(&self) -> &str {
        "mean_reversion"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "entry_std": 2.0, "exit_std": 0.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Lookback period", 20, 5, 100),
            float_param!(
                "entry_std",
                "Entry Std Dev",
                "Entry threshold in std devs",
                2.0,
                1.0,
                3.0
            ),
            float_param!(
                "exit_std",
                "Exit Std Dev",
                "Exit threshold in std devs",
                0.5,
                0.0,
                1.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let entry = get_f64(params, "entry_std").ok_or("entry_std is required")?;
        let exit = get_f64(params, "exit_std").ok_or("exit_std is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 100 {
            return Err("period must be <= 100".into());
        }
        if entry < 1.0 {
            return Err("entry_std must be >= 1.0".into());
        }
        if entry > 3.0 {
            return Err("entry_std must be <= 3.0".into());
        }
        if exit < 0.0 {
            return Err("exit_std must be >= 0.0".into());
        }
        if exit > 1.0 {
            return Err("exit_std must be <= 1.0".into());
        }
        if exit >= entry {
            return Err("exit_std must be less than entry_std".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let period = get_i64(params, "period").unwrap_or(20) as usize;
        let entry = get_f64(params, "entry_std").unwrap_or(2.0);
        let exit = get_f64(params, "exit_std").unwrap_or(0.5);
        if current_idx < period {
            return Signal::Hold;
        }
        let close = closes(klines);
        let ma = sma(&close, period);
        let sd = stddev(&close, period, &ma);
        let z_score = (close[current_idx] - ma[current_idx]) / sd[current_idx].max(1e-10);
        let prev_z =
            (close[current_idx - 1] - ma[current_idx - 1]) / sd[current_idx - 1].max(1e-10);
        // Entry: price deviates more than entry_std → bet on reversion
        // Exit: price reverts to within exit_std
        if prev_z >= -entry && z_score < -entry {
            Signal::Buy { quantity_pct: 1.0 }
        } else if prev_z <= entry && z_score > entry {
            Signal::Sell { quantity_pct: 1.0 }
        } else if z_score.abs() < exit {
            // Close position if held and price reverts
            Signal::CloseAll
        } else {
            Signal::Hold
        }
    }
}

// ========== 9. Ichimoku ==========

impl StrategyTemplate for IchimokuTemplate {
    fn id(&self) -> &str {
        "ichimoku"
    }
    fn name(&self) -> &str {
        "Ichimoku Cloud"
    }
    fn description(&self) -> &str {
        "Ichimoku Kinko Hyo cloud trading system"
    }
    fn category(&self) -> &str {
        "trend"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"conversion": 9, "base": 26, "span": 52, "displ": 26})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!(
                "conversion",
                "Conversion",
                "Conversion line period",
                9,
                5,
                20
            ),
            int_param!("base", "Base", "Base line period", 26, 10, 60),
            int_param!("span", "Span B", "Leading span B period", 52, 20, 120),
            int_param!("displ", "Displacement", "Displacement period", 26, 5, 60),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let conversion = get_i64(params, "conversion").ok_or("conversion is required")?;
        let base = get_i64(params, "base").ok_or("base is required")?;
        let span = get_i64(params, "span").ok_or("span is required")?;
        let displ = get_i64(params, "displ").ok_or("displ is required")?;
        if conversion < 5 {
            return Err("conversion must be >= 5".into());
        }
        if conversion > 20 {
            return Err("conversion must be <= 20".into());
        }
        if base < 10 {
            return Err("base must be >= 10".into());
        }
        if base > 60 {
            return Err("base must be <= 60".into());
        }
        if span < 20 {
            return Err("span must be >= 20".into());
        }
        if span > 120 {
            return Err("span must be <= 120".into());
        }
        if displ < 5 {
            return Err("displ must be >= 5".into());
        }
        if displ > 60 {
            return Err("displ must be <= 60".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        // Simplified Ichimoku: Tenkan-sen / Kijun-sen cross
        let conv = get_i64(params, "conversion").unwrap_or(9) as usize;
        let base = get_i64(params, "base").unwrap_or(26) as usize;
        if current_idx < base {
            return Signal::Hold;
        }
        let high = highs(klines);
        let low = lows(klines);

        // Tenkan-sen (Conversion): (highest high + lowest low) / 2 over period
        let tenkan = |idx: usize, p: usize| -> f64 {
            let start = idx + 1 - p;
            let h = high[start..=idx]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let l = low[start..=idx]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            (h + l) / 2.0
        };
        let curr_tenkan = tenkan(current_idx, conv);
        let curr_kijun = tenkan(current_idx, base);
        let prev_tenkan = tenkan(current_idx - 1, conv);
        let prev_kijun = tenkan(current_idx - 1, base);
        // TK cross: Tenkan crosses above Kijun → buy
        if prev_tenkan <= prev_kijun && curr_tenkan > curr_kijun {
            Signal::Buy { quantity_pct: 1.0 }
        } else if prev_tenkan >= prev_kijun && curr_tenkan < curr_kijun {
            Signal::Sell { quantity_pct: 1.0 }
        } else {
            Signal::Hold
        }
    }
}

// ========== 10. Double Bollinger ==========

impl StrategyTemplate for DoubleBollingerTemplate {
    fn id(&self) -> &str {
        "double_bollinger"
    }
    fn name(&self) -> &str {
        "Double Bollinger Bands"
    }
    fn description(&self) -> &str {
        "Double Bollinger Bands strategy with inner and outer bands"
    }
    fn category(&self) -> &str {
        "composite"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({"period": 20, "inner_std": 1.5, "outer_std": 2.5})
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("period", "Period", "Bollinger Bands period", 20, 5, 100),
            float_param!(
                "inner_std",
                "Inner Std Dev",
                "Inner band std dev",
                1.5,
                1.0,
                2.5
            ),
            float_param!(
                "outer_std",
                "Outer Std Dev",
                "Outer band std dev",
                2.5,
                2.0,
                4.0
            ),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let period = get_i64(params, "period").ok_or("period is required")?;
        let inner = get_f64(params, "inner_std").ok_or("inner_std is required")?;
        let outer = get_f64(params, "outer_std").ok_or("outer_std is required")?;
        if period < 5 {
            return Err("period must be >= 5".into());
        }
        if period > 100 {
            return Err("period must be <= 100".into());
        }
        if inner < 1.0 {
            return Err("inner_std must be >= 1.0".into());
        }
        if inner > 2.5 {
            return Err("inner_std must be <= 2.5".into());
        }
        if outer < 2.0 {
            return Err("outer_std must be >= 2.0".into());
        }
        if outer > 4.0 {
            return Err("outer_std must be <= 4.0".into());
        }
        if inner >= outer {
            return Err("inner_std must be less than outer_std".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        // Double Bollinger: buy when price inside inner band, sell when outside outer band
        let period = get_i64(params, "period").unwrap_or(20) as usize;
        let inner_m = get_f64(params, "inner_std").unwrap_or(1.5);
        let outer_m = get_f64(params, "outer_std").unwrap_or(2.5);
        if current_idx < period {
            return Signal::Hold;
        }
        let close = closes(klines);
        let ma = sma(&close, period);
        let sd = stddev(&close, period, &ma);
        let inner_upper = ma[current_idx] + inner_m * sd[current_idx];
        let inner_lower = ma[current_idx] - inner_m * sd[current_idx];
        let outer_upper = ma[current_idx] + outer_m * sd[current_idx];
        let outer_lower = ma[current_idx] - outer_m * sd[current_idx];
        let price = close[current_idx];
        // Entry: price outside outer band → mean reversion
        if price > outer_upper {
            Signal::Sell { quantity_pct: 1.0 }
        } else if price < outer_lower {
            Signal::Buy { quantity_pct: 1.0 }
        // Exit: price returns to inner band
        } else if price >= inner_lower && price <= inner_upper {
            Signal::CloseAll
        } else {
            Signal::Hold
        }
    }
}

// ============ Template Registry ============

pub fn get_all_templates() -> Vec<Box<dyn StrategyTemplate>> {
    vec![
        Box::new(MaCrossoverTemplate),
        Box::new(TripleMaTemplate),
        Box::new(MacdTemplate),
        Box::new(BollingerTemplate),
        Box::new(RsiTemplate),
        Box::new(KeltnerTemplate),
        Box::new(AtrStopTemplate),
        Box::new(MeanReversionTemplate),
        Box::new(IchimokuTemplate),
        Box::new(DoubleBollingerTemplate),
    ]
}

pub fn get_template(id: &str) -> Option<Box<dyn StrategyTemplate>> {
    get_all_templates().into_iter().find(|t| t.id() == id)
}

fn template_to_info(t: Box<dyn StrategyTemplate>) -> TemplateInfo {
    TemplateInfo {
        id: t.id().to_string(),
        template_id: template_type_to_uuid(t.id()),
        name: t.name().to_string(),
        description: t.description().to_string(),
        category: t.category().to_string(),
        default_parameters: t.default_parameters(),
        parameter_schema: t.parameter_schema(),
    }
}

// ============ Status Validation ============

fn validate_status_transition(current: &str, next: &str) -> Result<(), AppError> {
    let valid = matches!(
        (current, next),
        ("draft", "active") | ("active", "paused") | ("paused", "active") | ("paused", "stopped")
    );
    if !valid {
        return Err(AppError::Validation(format!(
            "Invalid status transition: {} -> {}",
            current, next
        )));
    }
    Ok(())
}

// Mapping from template_type string to a consistent UUID for ADR D6 compliance.
// In production this could come from a database templates table.
fn template_type_to_uuid(template_type: &str) -> Uuid {
    // Use a simple FNV-style hash to derive octets, then build v4-style UUID
    let bytes = template_type.as_bytes();
    let mut hash: u64 = 0xcbf29ce84222374fu64;
    for &b in bytes {
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= b as u64;
    }
    let (d1, d2) = ((hash >> 48) as u32, (hash >> 32) as u16);
    let d3 = (hash >> 16) as u16;
    let d4: [u8; 8] = [
        (hash >> 8) as u8,
        hash as u8,
        ((hash >> 56) ^ 0x40) as u8, // set version = 4
        ((hash >> 48) ^ 0x80) as u8, // set variant
        ((hash >> 40) & 0xff) as u8,
        ((hash >> 32) & 0xff) as u8,
        ((hash >> 24) & 0xff) as u8,
        ((hash >> 16) & 0xff) as u8,
    ];
    Uuid::from_fields(d1, d2, d3, &d4)
}

fn model_to_response(m: strategy::Model) -> StrategyResponse {
    StrategyResponse {
        id: m.id,
        user_id: m.user_id,
        name: m.name,
        description: m.description,
        symbol: m.symbol,
        timeframe: m.timeframe,
        strategy_type: m.strategy_type,
        template_id: template_type_to_uuid(&m.template_type),
        template_type: m.template_type,
        parameters: m.parameters,
        status: m.status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

// ============ CRUD Service Functions ============

pub async fn list_templates() -> Result<Vec<TemplateInfo>, AppError> {
    Ok(get_all_templates()
        .into_iter()
        .map(template_to_info)
        .collect())
}

// Resolve template_type from template_id UUID.
// Since templates are currently in-memory with string IDs, we use a deterministic
// UUID mapping. In production, this would query a templates database table.
fn resolve_template_type_from_id(template_id: &Uuid) -> Option<String> {
    // Check if template_id matches any known template's UUID
    let all_templates = get_all_templates();
    for t in all_templates {
        if template_id == &template_type_to_uuid(t.id()) {
            return Some(t.id().to_string());
        }
    }
    None
}

pub async fn create_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: CreateStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    // Resolve template_type from template_id
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Log the template_id being used
    tracing::info!(
        "Creating strategy with template_id: {}, resolved template_type: {}",
        req.template_id,
        template_type
    );

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol (non-empty, alphanumeric+USDT suffix)
    if req.symbol.trim().is_empty() {
        return Err(AppError::Validation("Symbol cannot be empty".into()));
    }
    if req.symbol.len() > 20 {
        return Err(AppError::Validation(
            "Symbol must be <= 20 characters".into(),
        ));
    }

    // Validate timeframe (allowed values)
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}. Allowed: trend_following/mean_reversion/grid_trading/arbitrage/custom",
            req.strategy_type
        )));
    }

    let now = chrono::Utc::now();
    let description = req.description.unwrap_or_default();
    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(req.name),
        description: Set(description),
        symbol: Set(req.symbol),
        timeframe: Set(req.timeframe),
        strategy_type: Set(req.strategy_type),
        template_type: Set(template_type),
        parameters: Set(req.parameters),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = model.insert(db).await?;
    Ok(model_to_response(saved))
}

pub async fn list_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: PaginationParams,
) -> Result<PaginatedResponse<StrategyResponse>, AppError> {
    let page = params.page();
    let size = params.size();
    let offset = params.offset();

    let total = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .count(db)
        .await? as u64;

    let items = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .order_by_desc(strategy::Column::UpdatedAt)
        .offset(offset)
        .limit(size)
        .all(db)
        .await?
        .into_iter()
        .map(model_to_response)
        .collect();

    Ok(PaginatedResponse {
        items,
        total,
        page,
        size,
    })
}

pub async fn export_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    status_filter: Option<&str>,
) -> Result<Vec<StrategyResponse>, AppError> {
    let mut query = strategy::Entity::find().filter(strategy::Column::UserId.eq(user_id));

    if let Some(status) = status_filter {
        query = query.filter(strategy::Column::Status.eq(status));
    }

    let items: Vec<StrategyResponse> = query
        .order_by_desc(strategy::Column::UpdatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(model_to_response)
        .collect();

    Ok(items)
}

pub async fn get_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    Ok(model_to_response(m))
}

pub async fn update_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
    req: UpdateStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    let mut active: strategy::ActiveModel = m.clone().into();
    active.updated_at = Set(chrono::Utc::now());

    if let Some(name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::Validation("Strategy name cannot be empty".into()));
        }
        if name.len() > 100 {
            return Err(AppError::Validation(
                "Strategy name must be <= 100 characters".into(),
            ));
        }
        active.name = Set(name);
    }

    if let Some(description) = req.description {
        active.description = Set(description);
    }

    if let Some(params) = req.parameters {
        let template = get_template(&m.template_type)
            .ok_or_else(|| AppError::Internal("Template not found for existing strategy".into()))?;
        template
            .validate(&params)
            .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;
        active.parameters = Set(params);
    }

    let saved = active.update(db).await?;
    Ok(model_to_response(saved))
}

pub async fn delete_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
) -> Result<(), AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    strategy::Entity::delete_by_id(strategy_id).exec(db).await?;

    Ok(())
}

pub async fn update_strategy_status(
    db: &DatabaseConnection,
    user_id: Uuid,
    strategy_id: Uuid,
    new_status: String,
) -> Result<StrategyResponse, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if m.user_id != user_id {
        return Err(AppError::NotFound("Strategy not found".into()));
    }

    validate_status_transition(&m.status, &new_status)?;

    let mut active: strategy::ActiveModel = m.into();
    active.status = Set(new_status);
    active.updated_at = Set(chrono::Utc::now());

    let saved = active.update(db).await?;
    Ok(model_to_response(saved))
}

pub async fn bulk_update_status(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: BulkUpdateStatusRequest,
) -> Result<Vec<StrategyResponse>, AppError> {
    if req.ids.is_empty() {
        return Err(AppError::Validation("ids cannot be empty".into()));
    }

    let valid_statuses = ["active", "paused", "draft"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid status: {}. Must be one of: active, paused, draft",
            req.status
        )));
    }

    let mut results = Vec::new();
    for strategy_id in req.ids {
        let m = strategy::Entity::find_by_id(strategy_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

        if m.user_id != user_id {
            return Err(AppError::NotFound("Strategy not found".into()));
        }

        validate_status_transition(&m.status, &req.status)?;

        let mut active: strategy::ActiveModel = m.into();
        active.status = Set(req.status.clone());
        active.updated_at = Set(chrono::Utc::now());

        let saved = active.update(db).await?;
        results.push(model_to_response(saved));
    }
    Ok(results)
}

// ============ Import/Export ============

pub async fn import_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: crate::models::schemas::ImportStrategyRequest,
) -> Result<StrategyResponse, AppError> {
    // Resolve template_type from template_id or fallback to provided template_type
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Log the template_id being used
    tracing::info!(
        "Importing strategy with template_id: {}, resolved template_type: {}",
        req.template_id,
        template_type
    );

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol
    if req.symbol.trim().is_empty() || req.symbol.len() > 20 {
        return Err(AppError::Validation("Invalid symbol".into()));
    }

    // Validate timeframe
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}",
            req.strategy_type
        )));
    }

    // Check for duplicate name within user's strategies
    let existing = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .filter(strategy::Column::Name.eq(&req.name))
        .one(db)
        .await?;
    let final_name = if existing.is_some() {
        // Append import suffix with counter
        let mut counter = 2;
        loop {
            let candidate = format!("{} (import-{})", req.name, counter);
            let exists = strategy::Entity::find()
                .filter(strategy::Column::UserId.eq(user_id))
                .filter(strategy::Column::Name.eq(&candidate))
                .one(db)
                .await?;
            if exists.is_none() {
                break candidate;
            }
            counter += 1;
        }
    } else {
        req.name.clone()
    };

    let now = chrono::Utc::now();
    let description = req.description.unwrap_or_default();
    let status = req.status.unwrap_or_else(|| "draft".to_string());

    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(final_name),
        description: Set(description),
        symbol: Set(req.symbol),
        timeframe: Set(req.timeframe),
        strategy_type: Set(req.strategy_type),
        template_type: Set(template_type),
        parameters: Set(req.parameters),
        status: Set(status),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = model.insert(db).await?;
    Ok(model_to_response(saved))
}

/// Import a batch of strategies from JSON export (ADR D2)
pub async fn import_batch(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: ImportBatchRequest,
) -> Result<ImportBatchResponse, AppError> {
    let mut imported = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, strategy_req) in req.strategies.iter().enumerate() {
        match import_single_strategy(db, user_id, strategy_req).await {
            Ok(_) => imported += 1,
            Err(e) => {
                errors.push(format!(
                    "策略[{}]「{}」导入失败: {}",
                    i + 1,
                    &strategy_req.name,
                    e
                ));
            }
        }
    }

    Ok(ImportBatchResponse { imported, errors })
}

/// Shared logic for importing a single strategy (validates, deduplicates name, inserts)
async fn import_single_strategy(
    db: &DatabaseConnection,
    user_id: Uuid,
    req: &ImportStrategyRequest,
) -> Result<(), AppError> {
    // Resolve template_type from template_id or fallback to provided template_type
    let template_type = if let Some(ref tt) = req.template_type {
        // Backward compatibility: if template_type is provided, use it directly
        tt.clone()
    } else {
        // Otherwise resolve from template_id
        resolve_template_type_from_id(&req.template_id).ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid template_id: {}. Could not resolve to a valid template_type",
                req.template_id
            ))
        })?
    };

    // Validate template exists
    let template = get_template(&template_type)
        .ok_or_else(|| AppError::Validation(format!("Unknown template type: {}", template_type)))?;

    // Validate parameters
    template
        .validate(&req.parameters)
        .map_err(|e| AppError::Validation(format!("Parameter validation failed: {}", e)))?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Strategy name cannot be empty".into()));
    }
    if req.name.len() > 100 {
        return Err(AppError::Validation(
            "Strategy name must be <= 100 characters".into(),
        ));
    }

    // Validate symbol
    if req.symbol.trim().is_empty() || req.symbol.len() > 20 {
        return Err(AppError::Validation("Invalid symbol".into()));
    }

    // Validate timeframe
    let allowed_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
    if !allowed_timeframes.contains(&req.timeframe.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid timeframe: {}. Allowed: 1m/5m/15m/30m/1h/4h/1d/1w",
            req.timeframe
        )));
    }

    // Validate strategy_type
    let allowed_types = [
        "trend_following",
        "mean_reversion",
        "grid_trading",
        "arbitrage",
        "custom",
    ];
    if !allowed_types.contains(&req.strategy_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid strategy_type: {}",
            req.strategy_type
        )));
    }

    // Check for duplicate name within user's strategies
    let existing = strategy::Entity::find()
        .filter(strategy::Column::UserId.eq(user_id))
        .filter(strategy::Column::Name.eq(&req.name))
        .one(db)
        .await?;
    let final_name = if existing.is_some() {
        // Append import suffix with counter
        let mut counter = 2;
        loop {
            let candidate = format!("{} (import-{})", req.name, counter);
            let exists = strategy::Entity::find()
                .filter(strategy::Column::UserId.eq(user_id))
                .filter(strategy::Column::Name.eq(&candidate))
                .one(db)
                .await?;
            if exists.is_none() {
                break candidate;
            }
            counter += 1;
        }
    } else {
        req.name.clone()
    };

    let now = chrono::Utc::now();
    let description = req.description.clone().unwrap_or_default();
    let status = req.status.clone().unwrap_or_else(|| "draft".to_string());

    let model = strategy::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        name: Set(final_name),
        description: Set(description),
        symbol: Set(req.symbol.clone()),
        timeframe: Set(req.timeframe.clone()),
        strategy_type: Set(req.strategy_type.clone()),
        template_type: Set(template_type.clone()),
        parameters: Set(req.parameters.clone()),
        status: Set(status),
        created_at: Set(now),
        updated_at: Set(now),
    };

    model.insert(db).await?;
    Ok(())
}

// ============ Bulk Delete ============

/// Delete multiple strategies by IDs for a user
pub async fn bulk_delete_strategies(
    db: &DatabaseConnection,
    user_id: Uuid,
    ids: &[Uuid],
) -> Result<BulkDeleteResponse, AppError> {
    if ids.is_empty() {
        return Ok(BulkDeleteResponse { deleted: 0 });
    }

    let mut deleted = 0_i64;
    for id in ids {
        let strategy = strategy::Entity::find_by_id(*id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

        if strategy.user_id != user_id {
            return Err(AppError::NotFound("Strategy not found".into()));
        }

        strategy::Entity::delete_by_id(*id).exec(db).await?;
        deleted += 1;
    }

    Ok(BulkDeleteResponse { deleted })
}

// ============ Strategy Code Storage ============

/// Store strategy code content (simple file-based storage)
pub async fn store_strategy_code(
    _db: &DatabaseConnection,
    user_id: Uuid,
    file_name: &str,
    content: &str,
) -> Result<String, AppError> {
    use std::fs;

    // Determine storage directory
    let storage_dir =
        std::env::var("STRATEGY_CODE_DIR").unwrap_or_else(|_| "/tmp/strategy_codes".to_string());

    let user_dir = format!("{}/{}", storage_dir, user_id);
    fs::create_dir_all(&user_dir)
        .map_err(|e| AppError::Internal(format!("Failed to create strategy directory: {}", e)))?;

    let safe_name = file_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect::<String>();

    let path = format!("{}/{}", user_dir, safe_name);
    fs::write(&path, content)
        .map_err(|e| AppError::Internal(format!("Failed to write strategy code file: {}", e)))?;

    Ok(path)
}

#[derive(Debug, serde::Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create klines for testing
    fn make_klines(prices: &[f64]) -> Vec<Kline> {
        prices
            .iter()
            .enumerate()
            .map(|(i, p)| Kline {
                open_time: i as i64 * 3600000,
                open: *p,
                high: *p * 1.01,
                low: *p * 0.99,
                close: *p,
                volume: 1000.0,
            })
            .collect()
    }

    // ===== MaCrossover Tests =====
    #[test]
    fn test_ma_crossover_default_valid() {
        let t = MaCrossoverTemplate;
        let params = t.default_parameters();
        assert!(t.validate(&params).is_ok());
    }

    #[test]
    fn test_ma_crossover_boundary() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 5, "slow_period": 10}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 50, "slow_period": 200}))
                .is_ok()
        );
    }

    #[test]
    fn test_ma_crossover_fast_gte_slow() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 30, "slow_period": 10}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 10}))
                .is_err()
        );
    }

    #[test]
    fn test_ma_crossover_invalid_values() {
        let t = MaCrossoverTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast_period": 0, "slow_period": 30}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 10, "slow_period": 5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast_period": 100, "slow_period": 30}))
                .is_err()
        );
    }

    #[test]
    fn test_ma_crossover_missing_fields() {
        let t = MaCrossoverTemplate;
        assert!(t.validate(&serde_json::json!({})).is_err());
        assert!(t.validate(&serde_json::json!({"fast_period": 10})).is_err());
    }

    // ===== MaCrossover Signal Tests =====
    #[test]
    fn test_ma_crossover_signal_buy() {
        let t = MaCrossoverTemplate;
        let params = serde_json::json!({"fast_period": 3, "slow_period": 5});
        // Uptrend: price goes 100→110→120→130→140, fast MA should cross above slow
        let klines = make_klines(&[
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0,
        ]);
        let signal = t.generate_signal(&klines, klines.len() - 1, &params);
        // In a strong uptrend, fast MA > slow MA, but we need a crossover event
        // Previous: fast > slow already, so this should be Hold
        assert_eq!(signal, Signal::Hold);
    }

    #[test]
    fn test_ma_crossover_signal_immature() {
        let t = MaCrossoverTemplate;
        let params = serde_json::json!({"fast_period": 3, "slow_period": 5});
        let klines = make_klines(&[100.0, 101.0, 102.0]); // not enough bars
        let signal = t.generate_signal(&klines, 2, &params);
        assert_eq!(signal, Signal::Hold);
    }

    // ===== TripleMA Tests =====
    #[test]
    fn test_triple_ma_default_valid() {
        let t = TripleMaTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_triple_ma_ordering() {
        let t = TripleMaTemplate;
        assert!(
            t.validate(&serde_json::json!({"short": 10, "medium": 5, "long": 50}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 5}))
                .is_err()
        );
    }

    #[test]
    fn test_triple_ma_boundary() {
        let t = TripleMaTemplate;
        assert!(
            t.validate(&serde_json::json!({"short": 5, "medium": 10, "long": 20}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"short": 20, "medium": 50, "long": 200}))
                .is_ok()
        );
    }

    // ===== MACD Tests =====
    #[test]
    fn test_macd_default_valid() {
        let t = MacdTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_macd_fast_lt_slow() {
        let t = MacdTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast": 26, "slow": 12, "signal": 9}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 12, "signal": 9}))
                .is_err()
        );
    }

    #[test]
    fn test_macd_signal_range() {
        let t = MacdTemplate;
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 1}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"fast": 12, "slow": 26, "signal": 50}))
                .is_err()
        );
    }

    // ===== Bollinger Tests =====
    #[test]
    fn test_bollinger_default_valid() {
        let t = BollingerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_bollinger_std_dev_range() {
        let t = BollingerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 5.0}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "std_dev": 1.5}))
                .is_ok()
        );
    }

    // ===== RSI Tests =====
    #[test]
    fn test_rsi_default_valid() {
        let t = RsiTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_rsi_overbought_oversold() {
        let t = RsiTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 30}))
                .is_ok()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 14, "overbought": 70, "oversold": 75}))
                .is_err()
        );
    }

    // ===== Keltner Tests =====
    #[test]
    fn test_keltner_default_valid() {
        let t = KeltnerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_keltner_atr_range() {
        let t = KeltnerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "atr_multiplier": 4.0}))
                .is_err()
        );
    }

    // ===== ATR Stop Tests =====
    #[test]
    fn test_atr_stop_default_valid() {
        let t = AtrStopTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_atr_stop_multiplier_range() {
        let t = AtrStopTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 14, "multiplier": 0.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 14, "multiplier": 6.0}))
                .is_err()
        );
    }

    // ===== Mean Reversion Tests =====
    #[test]
    fn test_mean_reversion_default_valid() {
        let t = MeanReversionTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_mean_reversion_std_ordering() {
        let t = MeanReversionTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "entry_std": 1.0, "exit_std": 1.0}))
                .is_err()
        );
    }

    // ===== Ichimoku Tests =====
    #[test]
    fn test_ichimoku_default_valid() {
        let t = IchimokuTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_ichimoku_boundary() {
        let t = IchimokuTemplate;
        assert!(
            t.validate(&serde_json::json!({"conversion": 5, "base": 10, "span": 20, "displ": 5}))
                .is_ok()
        );
        assert!(
            t.validate(
                &serde_json::json!({"conversion": 20, "base": 60, "span": 120, "displ": 60})
            )
            .is_ok()
        );
    }

    // ===== Double Bollinger Tests =====
    #[test]
    fn test_double_bollinger_default_valid() {
        let t = DoubleBollingerTemplate;
        assert!(t.validate(&t.default_parameters()).is_ok());
    }

    #[test]
    fn test_double_bollinger_inner_lt_outer() {
        let t = DoubleBollingerTemplate;
        assert!(
            t.validate(&serde_json::json!({"period": 20, "inner_std": 2.5, "outer_std": 1.5}))
                .is_err()
        );
        assert!(
            t.validate(&serde_json::json!({"period": 20, "inner_std": 2.0, "outer_std": 2.0}))
                .is_err()
        );
    }

    // ===== All Templates Tests =====
    #[test]
    fn test_all_templates_have_unique_ids() {
        let templates = get_all_templates();
        let mut ids: Vec<String> = templates.iter().map(|t| t.id().to_string()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn test_get_template_by_id() {
        let t = get_template("ma_crossover").unwrap();
        assert_eq!(t.id(), "ma_crossover");
        assert!(get_template("nonexistent").is_none());
    }

    #[test]
    fn test_all_templates_categories_valid() {
        let valid = ["trend", "mean_reversion", "volatility", "composite"];
        for t in get_all_templates() {
            assert!(
                valid.contains(&t.category()),
                "Invalid category: {}",
                t.category()
            );
        }
    }

    // ===== Status Transition Tests =====
    #[test]
    fn test_valid_status_transitions() {
        assert!(validate_status_transition("draft", "active").is_ok());
        assert!(validate_status_transition("active", "paused").is_ok());
        assert!(validate_status_transition("paused", "active").is_ok());
        assert!(validate_status_transition("paused", "stopped").is_ok());
    }

    #[test]
    fn test_invalid_status_transitions() {
        assert!(validate_status_transition("draft", "paused").is_err());
        assert!(validate_status_transition("draft", "stopped").is_err());
        assert!(validate_status_transition("active", "draft").is_err());
        assert!(validate_status_transition("active", "stopped").is_err());
        assert!(validate_status_transition("stopped", "active").is_err());
        assert!(validate_status_transition("stopped", "draft").is_err());
    }

    // ===== ParameterDef Tests =====
    #[test]
    fn test_int_param_values() {
        let p = int_param!("period", "Period", "The period", 20, 5, 100);
        assert_eq!(p.param_type, "integer");
        assert_eq!(p.default.as_i64(), Some(20));
        assert_eq!(p.min.as_ref().and_then(|v| v.as_i64()), Some(5));
    }

    #[test]
    fn test_float_param_values() {
        let p = float_param!("std_dev", "Std Dev", "Standard deviation", 2.5, 1.0, 5.0);
        assert_eq!(p.param_type, "float");
        assert!((p.default.as_f64().unwrap() - 2.5).abs() < 1e-10);
    }

    // ===== Technical Indicator Tests =====
    #[test]
    fn test_sma_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3);
        assert!((result[2] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2
        assert!((result[3] - 3.0).abs() < 1e-10); // (2+3+4)/3 = 3
        assert!((result[4] - 4.0).abs() < 1e-10); // (3+4+5)/3 = 4
    }

    #[test]
    fn test_sma_empty() {
        let result = sma(&[], 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rsi_all_rising() {
        let prices: Vec<f64> = (100..=130).map(|i| i as f64).collect();
        let rsi_vals = rsi(&prices, 14);
        // In a continuously rising market, RSI should be high (near 100)
        let last = rsi_vals[rsi_vals.len() - 1];
        assert!(last > 50.0, "RSI should be > 50 in uptrend, got {}", last);
    }

    #[test]
    fn test_rsi_all_falling() {
        let prices: Vec<f64> = (0..=30).rev().map(|i| 100.0 + i as f64).collect();
        let rsi_vals = rsi(&prices, 14);
        let last = rsi_vals[rsi_vals.len() - 1];
        assert!(last < 50.0, "RSI should be < 50 in downtrend, got {}", last);
    }

    #[test]
    fn test_ema_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = ema(&data, 3);
        // First value (idx 2) = SMA = 2.0
        assert!((result[2] - 2.0).abs() < 1e-10);
        // EMA[3] = (4.0 - 2.0) * 0.5 + 2.0 = 3.0
        assert!((result[3] - 3.0).abs() < 1e-10);
        // EMA[4] = (5.0 - 3.0) * 0.5 + 3.0 = 4.0
        assert!((result[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_stddev_constant() {
        let data = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let ma = sma(&data, 3);
        let sd = stddev(&data, 3, &ma);
        assert!(sd[4].abs() < 1e-10); // no variance
    }

    #[test]
    fn test_stddev_non_constant() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = sma(&data, 3);
        let sd = stddev(&data, 3, &ma);
        assert!(sd[2] > 0.0); // should have variance
    }

    #[test]
    fn test_macd_structure() {
        let data: Vec<f64> = (100..=200).map(|i| i as f64).collect();
        let (macd_line, signal_line, hist) = macd(&data, 12, 26, 9);
        assert_eq!(macd_line.len(), data.len());
        assert_eq!(signal_line.len(), data.len());
        assert_eq!(hist.len(), data.len());
    }

    #[test]
    fn test_atr_basic() {
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![11.0, 12.0, 13.0, 14.0, 15.0];
        let atr_vals = atr(&high, &low, &close, 3);
        assert!(atr_vals[atr_vals.len() - 1] > 0.0);
    }
}
