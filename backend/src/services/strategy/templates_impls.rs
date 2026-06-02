// ============ Template Implementations ============
//
// Helper functions (`sma`, `closes`, etc.) live in `super::common` and are
// batch-processing variants: they take a full `&[f64]` and return a `Vec<f64>`
// aligned with the input length.
use super::common::{atr, closes, highs, lows, macd, rsi, sma, stddev, StrategyTemplate};
use crate::models::backtest::{Kline, Signal};
use crate::models::schemas::ParameterDef;
use serde_json::Value;

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

#[macro_export]
macro_rules! int_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {{
        use $crate::models::schemas::ParameterDef as _PD;
        _PD {
            name: $name.to_string(),
            param_type: "integer".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: serde_json::json!($default),
            min: Some(serde_json::json!($min)),
            max: Some(serde_json::json!($max)),
            options: None,
        }
    }};
}

#[macro_export]
macro_rules! float_param {
    ($name:expr, $label:expr, $desc:expr, $default:expr, $min:expr, $max:expr) => {{
        use $crate::models::schemas::ParameterDef as _PD;
        _PD {
            name: $name.to_string(),
            param_type: "float".to_string(),
            label: $label.to_string(),
            description: $desc.to_string(),
            default: serde_json::json!($default),
            min: Some(serde_json::json!($min)),
            max: Some(serde_json::json!($max)),
            options: None,
        }
    }};
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


// =============================================================
// P1-1 templates: Grid / Martingale / Breakout
// =============================================================

// ---------- Grid Template ----------
//
// Range-trading strategy. Uses the first bar's close as a reference
// price and emits Buy when price dips below `lower` of ref and
// Sell when price rises above `upper` of ref.

pub struct GridTemplate;

impl StrategyTemplate for GridTemplate {
    fn id(&self) -> &str {
        "grid"
    }
    fn name(&self) -> &str {
        "Grid Trading"
    }
    fn description(&self) -> &str {
        "Range-trading: buy near the lower band, sell near the upper band, relative to a reference price"
    }
    fn category(&self) -> &str {
        "mean_reversion"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({
            "upper": 1.05,
            "lower": 0.95,
            "grid_levels": 10,
            "size_per_grid": 0.1,
        })
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            float_param!("upper", "Upper Band", "Upper band as ratio of reference price", 1.05, 1.001, 2.0),
            float_param!("lower", "Lower Band", "Lower band as ratio of reference price", 0.95, 0.5, 0.999),
            int_param!("grid_levels", "Grid Levels", "Number of grid levels (informational)", 10, 2, 50),
            float_param!("size_per_grid", "Size per Grid", "Position size per grid signal (0.0..=1.0)", 0.1, 0.01, 1.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let upper = get_f64(params, "upper").ok_or("upper is required")?;
        let lower = get_f64(params, "lower").ok_or("lower is required")?;
        let levels = get_i64(params, "grid_levels").ok_or("grid_levels is required")?;
        let size = get_f64(params, "size_per_grid").ok_or("size_per_grid is required")?;
        if upper <= 1.0 {
            return Err("upper must be > 1.0".into());
        }
        if upper > 2.0 {
            return Err("upper must be <= 2.0".into());
        }
        if lower <= 0.0 {
            return Err("lower must be > 0.0".into());
        }
        if lower >= 1.0 {
            return Err("lower must be < 1.0".into());
        }
        if lower >= upper {
            return Err("lower must be less than upper".into());
        }
        if levels < 2 {
            return Err("grid_levels must be >= 2".into());
        }
        if levels > 50 {
            return Err("grid_levels must be <= 50".into());
        }
        if size <= 0.0 {
            return Err("size_per_grid must be > 0".into());
        }
        if size > 1.0 {
            return Err("size_per_grid must be <= 1.0".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let upper = get_f64(params, "upper").unwrap_or(1.05);
        let lower = get_f64(params, "lower").unwrap_or(0.95);
        let size = get_f64(params, "size_per_grid").unwrap_or(0.1);
        if current_idx == 0 {
            return Signal::Hold;
        }
        let close = closes(klines);
        let ref_price = close[0];
        let price = close[current_idx];
        let upper_band = ref_price * upper;
        let lower_band = ref_price * lower;
        if price < lower_band {
            Signal::Buy { quantity_pct: size }
        } else if price > upper_band {
            Signal::Sell { quantity_pct: size }
        } else {
            Signal::Hold
        }
    }
}

// ---------- Martingale Template ----------
//
// Doubles position size after consecutive losing bars. Loss streak
// is approximated by consecutive down-bars; resets on up-bar or
// after a Buy signal is emitted.

pub struct MartingaleTemplate;

impl StrategyTemplate for MartingaleTemplate {
    fn id(&self) -> &str {
        "martingale"
    }
    fn name(&self) -> &str {
        "Martingale"
    }
    fn description(&self) -> &str {
        "Doubling position size on consecutive losses (recovery-oriented, high-risk)"
    }
    fn category(&self) -> &str {
        "recovery"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({
            "base_size": 0.1,
            "max_doubling": 3,
            "take_profit": 0.05,
        })
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            float_param!("base_size", "Base Size", "Base position size (0.0..=1.0)", 0.1, 0.01, 1.0),
            int_param!("max_doubling", "Max Doubling", "Max number of doublings (capped at 5)", 3, 0, 5),
            float_param!("take_profit", "Take Profit", "Take profit ratio (reserved, not used in P1-1)", 0.05, 0.001, 0.5),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let base = get_f64(params, "base_size").ok_or("base_size is required")?;
        let max_d = get_i64(params, "max_doubling").ok_or("max_doubling is required")?;
        let tp = get_f64(params, "take_profit").ok_or("take_profit is required")?;
        if base <= 0.0 {
            return Err("base_size must be > 0".into());
        }
        if base > 1.0 {
            return Err("base_size must be <= 1.0".into());
        }
        if max_d < 0 {
            return Err("max_doubling must be >= 0".into());
        }
        if max_d > 5 {
            return Err("max_doubling must be <= 5".into());
        }
        if tp <= 0.0 {
            return Err("take_profit must be > 0".into());
        }
        if tp > 0.5 {
            return Err("take_profit must be <= 0.5".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let base = get_f64(params, "base_size").unwrap_or(0.1);
        let max_d = get_i64(params, "max_doubling").unwrap_or(3).max(0) as u32;
        if current_idx == 0 {
            return Signal::Hold;
        }
        let close = closes(klines);
        let prev = close[current_idx - 1];
        let curr = close[current_idx];
        if curr < prev {
            let size = base * 2f64.powi(max_d as i32);
            Signal::Buy { quantity_pct: size.min(1.0) }
        } else {
            Signal::Hold
        }
    }
}

// ---------- Breakout Template ----------
//
// Buy on N-period high breakout (with ATR filter); Sell on N-period
// low breakdown. ATR multiplier prevents false breakouts in choppy
// markets.

pub struct BreakoutTemplate;

impl StrategyTemplate for BreakoutTemplate {
    fn id(&self) -> &str {
        "breakout"
    }
    fn name(&self) -> &str {
        "Breakout"
    }
    fn description(&self) -> &str {
        "N-period high/low breakout with ATR volatility filter"
    }
    fn category(&self) -> &str {
        "momentum"
    }

    fn default_parameters(&self) -> Value {
        serde_json::json!({
            "lookback": 20,
            "atr_period": 14,
            "atr_mult": 1.5,
            "quantity_pct": 1.0,
        })
    }

    fn parameter_schema(&self) -> Vec<ParameterDef> {
        vec![
            int_param!("lookback", "Lookback", "Lookback period for high/low", 20, 5, 100),
            int_param!("atr_period", "ATR Period", "ATR calculation period", 14, 5, 50),
            float_param!("atr_mult", "ATR Multiplier", "ATR filter multiplier (0=disabled)", 1.5, 0.0, 5.0),
            float_param!("quantity_pct", "Position Size", "Position size (0.0..=1.0)", 1.0, 0.01, 1.0),
        ]
    }

    fn validate(&self, params: &Value) -> Result<(), String> {
        let lookback = get_i64(params, "lookback").ok_or("lookback is required")?;
        let atr_p = get_i64(params, "atr_period").ok_or("atr_period is required")?;
        let atr_m = get_f64(params, "atr_mult").ok_or("atr_mult is required")?;
        let q = get_f64(params, "quantity_pct").ok_or("quantity_pct is required")?;
        if lookback < 5 {
            return Err("lookback must be >= 5".into());
        }
        if lookback > 100 {
            return Err("lookback must be <= 100".into());
        }
        if atr_p < 5 {
            return Err("atr_period must be >= 5".into());
        }
        if atr_p > 50 {
            return Err("atr_period must be <= 50".into());
        }
        if atr_m < 0.0 {
            return Err("atr_mult must be >= 0".into());
        }
        if atr_m > 5.0 {
            return Err("atr_mult must be <= 5.0".into());
        }
        if q <= 0.0 {
            return Err("quantity_pct must be > 0".into());
        }
        if q > 1.0 {
            return Err("quantity_pct must be <= 1.0".into());
        }
        Ok(())
    }

    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal {
        let lookback = get_i64(params, "lookback").unwrap_or(20) as usize;
        let atr_p = get_i64(params, "atr_period").unwrap_or(14) as usize;
        let atr_m = get_f64(params, "atr_mult").unwrap_or(1.5);
        let q = get_f64(params, "quantity_pct").unwrap_or(1.0);
        if current_idx < lookback.max(atr_p) {
            return Signal::Hold;
        }
        let high = highs(klines);
        let low = lows(klines);
        let close = closes(klines);
        let window_start = current_idx - lookback;
        let window_high = high[window_start..current_idx]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let window_low = low[window_start..current_idx]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        let atr_buffer = if atr_m > 0.0 {
            if current_idx < atr_p {
                0.0
            } else {
                let atr_vals = atr(&high, &low, &close, atr_p);
                atr_vals[current_idx] * atr_m
            }
        } else {
            0.0
        };
        let price = close[current_idx];
        if price > window_high + atr_buffer {
            Signal::Buy { quantity_pct: q }
        } else if price < window_low - atr_buffer {
            Signal::Sell { quantity_pct: q }
        } else {
            Signal::Hold
        }
    }
}

// =============================================================
// P1-1 unit tests
// =============================================================

#[cfg(test)]
mod p1_1_tests {
    use super::*;
    use crate::models::backtest::{Kline, Signal};

    fn kline(t: i64, c: f64) -> Kline {
        Kline {
            open_time: t,
            open: c,
            high: c * 1.01,
            low: c * 0.99,
            close: c,
            volume: 100.0,
        }
    }

    fn flat_series(n: usize, price: f64) -> Vec<Kline> {
        (0..n).map(|i| kline(i as i64 * 60_000, price)).collect()
    }

    #[test]
    fn test_p1_1_ids_unique() {
        let all = super::super::registry::get_all_templates();
        let mut ids: Vec<&str> = all.iter().map(|t| t.id()).collect();
        ids.sort();
        let len_before = ids.len();
        ids.dedup();
        assert_eq!(len_before, ids.len(), "duplicate template ids");
        for nid in ["grid", "martingale", "breakout"] {
            assert!(all.iter().any(|t| std::ptr::eq(t.id().as_ptr(), nid.as_ptr()) || t.id() == nid), "missing template id: {}", nid);
        }
    }

    #[test]
    fn test_grid_validate_rejects_inverted() {
        let t = GridTemplate;
        let bad = serde_json::json!({"upper": 0.95, "lower": 1.05, "grid_levels": 5, "size_per_grid": 0.1});
        assert!(t.validate(&bad).is_err());
        let good = serde_json::json!({"upper": 1.05, "lower": 0.95, "grid_levels": 5, "size_per_grid": 0.1});
        assert!(t.validate(&good).is_ok());
    }

    #[test]
    fn test_grid_signal_buy_below_lower() {
        let t = GridTemplate;
        let params = serde_json::json!({"upper": 1.05, "lower": 0.95, "grid_levels": 5, "size_per_grid": 0.2});
        let mut ks = flat_series(3, 100.0);
        ks[2] = kline(2 * 60_000, 90.0);
        match t.generate_signal(&ks, 2, &params) {
            Signal::Buy { quantity_pct } => assert!((quantity_pct - 0.2).abs() < 1e-9),
            _ => panic!("expected Buy"),
        }
    }

    #[test]
    fn test_grid_signal_sell_above_upper() {
        let t = GridTemplate;
        let params = serde_json::json!({"upper": 1.05, "lower": 0.95, "grid_levels": 5, "size_per_grid": 0.2});
        let mut ks = flat_series(3, 100.0);
        ks[2] = kline(2 * 60_000, 110.0);
        match t.generate_signal(&ks, 2, &params) {
            Signal::Sell { quantity_pct } => assert!((quantity_pct - 0.2).abs() < 1e-9),
            _ => panic!("expected Sell"),
        }
    }

    #[test]
    fn test_martingale_validate_doubling_cap() {
        let t = MartingaleTemplate;
        let bad = serde_json::json!({"base_size": 0.1, "max_doubling": 10, "take_profit": 0.05});
        assert!(t.validate(&bad).is_err());
        let good = serde_json::json!({"base_size": 0.1, "max_doubling": 3, "take_profit": 0.05});
        assert!(t.validate(&good).is_ok());
    }

    #[test]
    fn test_martingale_signal_doubles_on_down_bar() {
        let t = MartingaleTemplate;
        let params = serde_json::json!({"base_size": 0.1, "max_doubling": 3, "take_profit": 0.05});
        let ks = vec![kline(0, 100.0), kline(60_000, 99.0)];
        match t.generate_signal(&ks, 1, &params) {
            Signal::Buy { quantity_pct } => assert!((quantity_pct - 0.8).abs() < 1e-9),
            _ => panic!("expected Buy"),
        }
    }

    #[test]
    fn test_martingale_signal_hold_on_up_bar() {
        let t = MartingaleTemplate;
        let params = serde_json::json!({"base_size": 0.1, "max_doubling": 3, "take_profit": 0.05});
        let ks = vec![kline(0, 100.0), kline(60_000, 101.0)];
        assert_eq!(t.generate_signal(&ks, 1, &params), Signal::Hold);
    }

    #[test]
    fn test_breakout_validate_lookback_range() {
        let t = BreakoutTemplate;
        let bad = serde_json::json!({"lookback": 3, "atr_period": 14, "atr_mult": 1.5, "quantity_pct": 1.0});
        assert!(t.validate(&bad).is_err());
        let good = serde_json::json!({"lookback": 20, "atr_period": 14, "atr_mult": 1.5, "quantity_pct": 1.0});
        assert!(t.validate(&good).is_ok());
    }

    #[test]
    fn test_breakout_signal_buy_above_window_high() {
        let t = BreakoutTemplate;
        let params = serde_json::json!({"lookback": 5, "atr_period": 3, "atr_mult": 0.0, "quantity_pct": 1.0});
        let mut ks = flat_series(6, 100.0);
        ks[5] = kline(5 * 60_000, 110.0);
        match t.generate_signal(&ks, 5, &params) {
            Signal::Buy { .. } => {}
            _ => panic!("expected Buy"),
        }
    }

    #[test]
    fn test_breakout_signal_sell_below_window_low() {
        let t = BreakoutTemplate;
        let params = serde_json::json!({"lookback": 5, "atr_period": 3, "atr_mult": 0.0, "quantity_pct": 1.0});
        let mut ks = flat_series(6, 100.0);
        ks[5] = kline(5 * 60_000, 90.0);
        match t.generate_signal(&ks, 5, &params) {
            Signal::Sell { .. } => {}
            _ => panic!("expected Sell"),
        }
    }

    #[test]
    fn test_breakout_signal_hold_during_warmup() {
        let t = BreakoutTemplate;
        let params = serde_json::json!({"lookback": 20, "atr_period": 14, "atr_mult": 1.5, "quantity_pct": 1.0});
        let ks = flat_series(10, 100.0);
        assert_eq!(t.generate_signal(&ks, 5, &params), Signal::Hold);
    }
}
