//! 技术指标 DTO（KDJ/MA/MACD/RSI/Bollinger）

use serde::{Deserialize, Serialize};

// ============ KDJ Indicator ============

/// KDJ signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum KdjSignal {
    GoldenCross,
    DeathCross,
    Overbought,
    Oversold,
    #[default]
    None,
}

/// Single KDJ bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdjBar {
    pub open_time: i64,
    pub k: f64,
    pub d: f64,
    pub j: f64,
    pub signal: KdjSignal,
}

/// KDJ calculation parameters
#[derive(Debug, Clone, Serialize)]
pub struct KdjParams {
    pub n: usize,
    pub m1: usize,
    pub m2: usize,
}

/// KDJ API response
#[derive(Debug, Clone, Serialize)]
pub struct KdjResponse {
    pub data: Vec<KdjBar>,
    pub params: KdjParams,
    pub symbol: String,
    pub interval: String,
}

/// KDJ query parameters (incoming from HTTP)
#[derive(Debug, Deserialize)]
pub struct KdjQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub n: Option<usize>,
    pub m1: Option<usize>,
    pub m2: Option<usize>,
}

// ─── MA (Moving Average) ────────────────────────────────────────────────────

/// Single MA line result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaBar {
    pub open_time: i64,
    pub ma: f64,
}

/// MA API response
#[derive(Debug, Clone, Serialize)]
pub struct MaResponse {
    pub data: Vec<MaBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// MA query parameters
#[derive(Debug, Deserialize)]
pub struct MaQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

// ─── MACD ─────────────────────────────────────────────────────────────────────

/// Single MACD bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacdBar {
    pub open_time: i64,
    pub macd: f64,      // MACD line value
    pub signal: f64,    // Signal line value
    pub histogram: f64, // MACD - Signal
}

/// MACD API response
#[derive(Debug, Clone, Serialize)]
pub struct MacdResponse {
    pub data: Vec<MacdBar>,
    pub params: MacdParams,
    pub symbol: String,
    pub interval: String,
}

/// MACD calculation parameters
#[derive(Debug, Clone, Serialize)]
pub struct MacdParams {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}

/// MACD query parameters
#[derive(Debug, Deserialize)]
pub struct MacdQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub fast_period: Option<usize>,
    pub slow_period: Option<usize>,
    pub signal_period: Option<usize>,
}

// ─── RSI ─────────────────────────────────────────────────────────────────────

/// Single RSI bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsiBar {
    pub open_time: i64,
    pub rsi: f64,
}

/// RSI API response
#[derive(Debug, Clone, Serialize)]
pub struct RsiResponse {
    pub data: Vec<RsiBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// RSI query parameters
#[derive(Debug, Deserialize)]
pub struct RsiQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

// ─── Bollinger Bands ────────────────────────────────────────────────────────

/// Single Bollinger Bands bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBar {
    pub open_time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Bollinger Bands API response
#[derive(Debug, Clone, Serialize)]
pub struct BollingerResponse {
    pub data: Vec<BollingerBar>,
    pub params: BollingerParams,
    pub symbol: String,
    pub interval: String,
}

/// Bollinger Bands parameters
#[derive(Debug, Clone, Serialize)]
pub struct BollingerParams {
    pub period: usize,
    pub std_dev: f64,
}

/// Bollinger Bands query parameters
#[derive(Debug, Deserialize)]
pub struct BollingerQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
    pub std_dev: Option<f64>,
}

/// EMA bar response
#[derive(Debug, Clone, Serialize)]
pub struct EmaResponse {
    pub data: Vec<MaBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// EMA query parameters
#[derive(Debug, Deserialize)]
pub struct EmaQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

/// ATR bar response
#[derive(Debug, Clone, Serialize)]
pub struct AtrResponse {
    pub data: Vec<RsiBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// ATR query parameters
#[derive(Debug, Deserialize)]
pub struct AtrQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

/// Stochastic bar
#[derive(Debug, Clone, Serialize)]
pub struct StochasticBar {
    pub open_time: i64,
    pub k: f64,
    pub d: f64,
}

/// Stochastic params
#[derive(Debug, Clone, Serialize)]
pub struct StochasticParams {
    pub k_period: usize,
    pub d_period: usize,
    pub smooth_k: usize,
}

/// Stochastic API response
#[derive(Debug, Clone, Serialize)]
pub struct StochasticResponse {
    pub data: Vec<StochasticBar>,
    pub params: StochasticParams,
    pub symbol: String,
    pub interval: String,
}

/// Stochastic query parameters
#[derive(Debug, Deserialize)]
pub struct StochasticQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub k_period: Option<usize>,
    pub d_period: Option<usize>,
    pub smooth_k: Option<usize>,
}
