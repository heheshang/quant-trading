use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 套利类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArbitrageType {
    CalendarSpread, // 跨期套利
    CrossPair,      // 跨品种套利
    SpotFutures,    // 期现套利
}

impl std::fmt::Display for ArbitrageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArbitrageType::CalendarSpread => write!(f, "calendar_spread"),
            ArbitrageType::CrossPair => write!(f, "cross_pair"),
            ArbitrageType::SpotFutures => write!(f, "spot_futures"),
        }
    }
}

/// 价差计算模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpreadCalculationMode {
    Ratio,      // 价格比率
    Percentage, // 百分比差
    ZScore,     // Z-score（统计套利）
}

/// 套利信号类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    EntryLong,  // 做多价差（A多B空）
    EntryShort, // 做空价差（A空B多）
    Exit,       // 平仓
    StopLoss,   // 止损
}

/// 信号方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    LongSpread,  // long_spread (A多B空)
    ShortSpread, // short_spread (A空B多)
    Neutral,     // 无信号
}

/// 持仓状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PositionStatus {
    Open,
    Closed,
    Liquidated,
}

/// 套利对配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitragePairConfig {
    pub spread_entry_threshold: Decimal,
    pub spread_exit_threshold: Decimal,
    pub max_position_size: Decimal,
    pub calculation_mode: SpreadCalculationMode,
    pub correlation_threshold: Option<Decimal>, // 跨品种用
    pub z_score_entry: Option<Decimal>,         // 跨品种用
    pub z_score_exit: Option<Decimal>,          // 跨品种用
}

/// 数据库记录的套利对
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitragePair {
    pub id: i32,
    pub pair_type: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub exchange: String,
    pub status: String,
    pub spread_entry_threshold: Decimal,
    pub spread_exit_threshold: Decimal,
    pub max_position_size: Decimal,
    pub calculation_mode: String,
    pub correlation_threshold: Option<Decimal>,
    pub z_score_entry: Option<Decimal>,
    pub z_score_exit: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建套利对请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArbitragePairRequest {
    pub pair_type: ArbitrageType,
    pub symbol_a: String,
    pub symbol_b: String,
    pub exchange: String,
    #[serde(flatten)]
    pub spread_config: ArbitragePairConfig,
}

/// 更新套利对请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArbitragePairRequest {
    pub pair_type: Option<ArbitrageType>,
    pub symbol_a: Option<String>,
    pub symbol_b: Option<String>,
    pub exchange: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub spread_config: Option<ArbitragePairConfig>,
}

/// 价差数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadData {
    pub pair_id: i32,
    pub spread: Decimal,
    pub spread_pct: Decimal,
    pub z_score: Option<Decimal>,
    pub historical_mean: Option<Decimal>,
    pub historical_std: Option<Decimal>,
    pub signal: SignalDirection,
    pub timestamp: DateTime<Utc>,
}

/// 套利持仓
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitragePosition {
    pub id: i32,
    pub pair_id: i32,
    pub direction: String,
    pub size_a: Decimal,
    pub size_b: Decimal,
    pub entry_spread: Decimal,
    pub current_spread: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// 套利信号记录
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitrageSignal {
    pub id: i32,
    pub pair_id: i32,
    pub signal_type: String,
    pub spread: Option<Decimal>,
    pub z_score: Option<Decimal>,
    pub confidence: Option<Decimal>,
    pub executed: bool,
    pub created_at: DateTime<Utc>,
}

/// 实时价差信息（内存中计算）
#[derive(Debug, Clone)]
pub struct LiveSpread {
    pub pair_id: i32,
    pub price_a: Decimal,
    pub price_b: Decimal,
    pub spread: Decimal,
    pub spread_pct: Decimal,
    pub z_score: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

/// Z-score 滑动窗口数据
#[derive(Debug, Clone)]
pub struct SpreadWindow {
    values: Vec<Decimal>,
    window_size: usize,
}

impl SpreadWindow {
    pub fn new(window_size: usize) -> Self {
        Self {
            values: Vec::new(),
            window_size,
        }
    }

    pub fn push(&mut self, value: Decimal) {
        if self.values.len() >= self.window_size {
            self.values.remove(0);
        }
        self.values.push(value);
    }

    pub fn mean(&self) -> Option<Decimal> {
        if self.values.is_empty() {
            return None;
        }
        let sum: Decimal = self.values.iter().sum();
        Some(sum / Decimal::from(self.values.len()))
    }

    pub fn std(&self) -> Option<Decimal> {
        if self.values.len() < 2 {
            return None;
        }
        let mean = self.mean()?;
        let variance: Decimal = self
            .values
            .iter()
            .map(|v| {
                let diff = *v - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(self.values.len() - 1);
        // 避免负数（浮点误差）
        if variance <= Decimal::ZERO {
            return Some(Decimal::ZERO);
        }
        // 标准差
        let approx = variance.to_string().parse::<f64>().ok()?;
        let std_f = approx.sqrt();
        Some(Decimal::from_f64_retain(std_f).unwrap_or(Decimal::ZERO))
    }

    pub fn z_score(&self, value: Decimal) -> Option<Decimal> {
        let mean = self.mean()?;
        let std = self.std()?;
        if std == Decimal::ZERO {
            return Some(Decimal::ZERO);
        }
        let mean_f = mean.to_string().parse::<f64>().ok()?;
        let std_f = std.to_string().parse::<f64>().ok()?;
        let val_f = value.to_string().parse::<f64>().ok()?;
        let z = (val_f - mean_f) / std_f;
        Decimal::from_f64_retain(z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spread_window() {
        let mut window = SpreadWindow::new(5);
        window.push(Decimal::from(100));
        window.push(Decimal::from(102));
        window.push(Decimal::from(101));
        window.push(Decimal::from(99));
        window.push(Decimal::from(98));

        assert_eq!(window.values.len(), 5);
        let mean = window.mean().unwrap();
        assert!(mean > Decimal::from(99) && mean < Decimal::from(101));

        let std = window.std().unwrap();
        assert!(std > Decimal::ZERO);

        // Z-score of last value
        let z = window.z_score(Decimal::from(98));
        assert!(z.is_some());
    }

    #[test]
    fn test_arbitrage_type_display() {
        assert_eq!(ArbitrageType::CalendarSpread.to_string(), "calendar_spread");
        assert_eq!(ArbitrageType::CrossPair.to_string(), "cross_pair");
        assert_eq!(ArbitrageType::SpotFutures.to_string(), "spot_futures");
    }

    #[test]
    fn test_signal_direction_serde() {
        let json = r#""long_spread""#;
        let dir: SignalDirection = serde_json::from_str(json).unwrap();
        assert_eq!(dir, SignalDirection::LongSpread);

        let back = serde_json::to_string(&SignalDirection::ShortSpread).unwrap();
        assert_eq!(back, r#""short_spread""#);
    }
}
