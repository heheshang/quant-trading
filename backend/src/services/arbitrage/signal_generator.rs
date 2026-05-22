//! 套利信号生成器
//! 基于价差偏离度和 Z-score 生成入场/出场/止损信号

use crate::services::arbitrage::types::*;
use crate::utils::error::AppError;
use rust_decimal::Decimal;

/// 信号生成器配置
pub struct SignalGeneratorConfig {
    /// Z-score 入场阈值（默认 2.0）
    pub z_score_entry: Decimal,
    /// Z-score 出场阈值（默认 0.5）
    pub z_score_exit: Decimal,
    /// 最小置信度（0-1）
    pub min_confidence: Decimal,
}

impl Default for SignalGeneratorConfig {
    fn default() -> Self {
        Self {
            z_score_entry: Decimal::from(2),
            z_score_exit: Decimal::from(50) / Decimal::from(100), // 0.5
            min_confidence: Decimal::from(70) / Decimal::from(100), // 0.7
        }
    }
}

/// 套利信号生成器
pub struct SignalGenerator {
    config: SignalGeneratorConfig,
}

impl SignalGenerator {
    pub fn new(config: SignalGeneratorConfig) -> Self {
        Self { config }
    }

    pub fn default_with_entry_threshold(entry: Decimal, exit: Decimal) -> Self {
        Self {
            config: SignalGeneratorConfig {
                z_score_entry: entry,
                z_score_exit: exit,
                ..Default::default()
            },
        }
    }

    /// 基于 LiveSpread 数据生成信号
    ///
    /// 信号逻辑：
    /// - EntryLong: z_score < -z_score_entry（价差被低估，做多价差）
    /// - EntryShort: z_score > +z_score_entry（价差被高估，做空价差）
    /// - Exit: |z_score| < z_score_exit（价差回归）
    /// - StopLoss: |z_score| > 2 * z_score_entry（价差继续偏离，止损）
    pub fn generate_signal(
        &self,
        spread_data: &LiveSpread,
        entry_threshold: Decimal,
        exit_threshold: Decimal,
        current_position: Option<&ArbitragePosition>,
    ) -> Result<Option<SignalType>, AppError> {
        // 如果没有 z_score，使用简单的价差偏离判断
        if let Some(z) = spread_data.z_score {
            return self.generate_from_zscore(z, entry_threshold, exit_threshold, current_position);
        }

        // 无 z_score 时的兜底逻辑（使用 spread_pct）
        Ok(self.generate_from_spread_pct(
            spread_data.spread_pct,
            entry_threshold,
            exit_threshold,
            current_position,
        ))
    }

    fn generate_from_zscore(
        &self,
        z_score: Decimal,
        entry_threshold: Decimal,
        exit_threshold: Decimal,
        current_position: Option<&ArbitragePosition>,
    ) -> Result<Option<SignalType>, AppError> {
        // 已持仓时判断是否需要平仓
        if let Some(pos) = current_position {
            if pos.status == "open" {
                let z_f = z_score.to_string().parse::<f64>().unwrap_or(0.0);
                let exit_f = exit_threshold.to_string().parse::<f64>().unwrap_or(0.0);
                let entry_f = entry_threshold.to_string().parse::<f64>().unwrap_or(0.0);

                // 止损：z_score 继续同向扩大
                if pos.direction == "long_spread" && z_score < -Decimal::from(2) * entry_threshold {
                    return Ok(Some(SignalType::StopLoss));
                }
                if pos.direction == "short_spread" && z_score > Decimal::from(2) * entry_threshold {
                    return Ok(Some(SignalType::StopLoss));
                }

                // 平仓：z_score 回归
                if (pos.direction == "long_spread" && z_score >= -exit_threshold)
                    || (pos.direction == "short_spread" && z_score <= exit_threshold)
                {
                    return Ok(Some(SignalType::Exit));
                }

                return Ok(None);
            }
        }

        // 无持仓时的入场判断
        let z_f = z_score.to_string().parse::<f64>().unwrap_or(0.0);
        let entry_f = entry_threshold.to_string().parse::<f64>().unwrap_or(2.0);

        if z_f < -entry_f {
            Ok(Some(SignalType::EntryLong))
        } else if z_f > entry_f {
            Ok(Some(SignalType::EntryShort))
        } else {
            Ok(None)
        }
    }

    fn generate_from_spread_pct(
        &self,
        spread_pct: Decimal,
        entry_threshold: Decimal,
        exit_threshold: Decimal,
        current_position: Option<&ArbitragePosition>,
    ) -> Option<SignalType> {
        let entry_f = entry_threshold.to_string().parse::<f64>().unwrap_or(2.0);
        let exit_f = exit_threshold.to_string().parse::<f64>().unwrap_or(0.5);
        let spread_f = spread_pct.to_string().parse::<f64>().unwrap_or(0.0);

        if let Some(pos) = current_position {
            if pos.status == "open" {
                // 持仓中，检测平仓
                if (pos.direction == "long_spread" && spread_f >= -exit_f)
                    || (pos.direction == "short_spread" && spread_f <= exit_f)
                {
                    return Some(SignalType::Exit);
                }
                return None;
            }
        }

        if spread_f < -entry_f {
            Some(SignalType::EntryLong)
        } else if spread_f > entry_f {
            Some(SignalType::EntryShort)
        } else {
            None
        }
    }

    /// 将 SignalType 转换为 SignalDirection
    pub fn signal_to_direction(signal: &SignalType) -> SignalDirection {
        match signal {
            SignalType::EntryLong => SignalDirection::LongSpread,
            SignalType::EntryShort => SignalDirection::ShortSpread,
            SignalType::Exit | SignalType::StopLoss => SignalDirection::Neutral,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_live_spread(z_score: Option<Decimal>) -> LiveSpread {
        LiveSpread {
            pair_id: 1,
            price_a: Decimal::from(10200),
            price_b: Decimal::from(10000),
            spread: Decimal::from(200),
            spread_pct: Decimal::from(2),
            z_score,
            timestamp: Utc::now(),
        }
    }

    fn make_open_position(direction: &str) -> ArbitragePosition {
        ArbitragePosition {
            id: 1,
            pair_id: 1,
            direction: direction.to_string(),
            size_a: Decimal::from(100),
            size_b: Decimal::from(100),
            entry_spread: Decimal::from(100),
            current_spread: None,
            unrealized_pnl: None,
            status: "open".to_string(),
            opened_at: Utc::now(),
            closed_at: None,
        }
    }

    #[test]
    fn test_entry_long_when_zscore_oversold() {
        let signal_gen = SignalGenerator::default_with_entry_threshold(
            Decimal::from(2),
            Decimal::from(50) / Decimal::from(100),
        );

        let spread = make_live_spread(Some(Decimal::from(-3))); // z = -3 < -2
        let signal = signal_gen
            .generate_signal(
                &spread,
                Decimal::from(2),
                Decimal::from(50) / Decimal::from(100),
                None,
            )
            .unwrap();
        assert_eq!(signal, Some(SignalType::EntryLong));
    }

    #[test]
    fn test_entry_short_when_zscore_overbought() {
        let signal_gen = SignalGenerator::default_with_entry_threshold(
            Decimal::from(2),
            Decimal::from(50) / Decimal::from(100),
        );

        let spread = make_live_spread(Some(Decimal::from(3))); // z = 3 > 2
        let signal = signal_gen
            .generate_signal(
                &spread,
                Decimal::from(2),
                Decimal::from(50) / Decimal::from(100),
                None,
            )
            .unwrap();
        assert_eq!(signal, Some(SignalType::EntryShort));
    }

    #[test]
    fn test_neutral_when_zscore_in_range() {
        let signal_gen = SignalGenerator::default_with_entry_threshold(
            Decimal::from(2),
            Decimal::from(50) / Decimal::from(100),
        );

        let spread = make_live_spread(Some(Decimal::from(1))); // 1 < 2, no signal
        let signal = signal_gen
            .generate_signal(
                &spread,
                Decimal::from(2),
                Decimal::from(50) / Decimal::from(100),
                None,
            )
            .unwrap();
        assert_eq!(signal, None);
    }

    #[test]
    fn test_exit_when_position_regression() {
        let signal_gen = SignalGenerator::default_with_entry_threshold(
            Decimal::from(2),
            Decimal::from(50) / Decimal::from(100),
        );

        // 持仓 long_spread，z_score 回到 -0.3（回归）
        let spread = make_live_spread(Some(Decimal::from(-30) / Decimal::from(100)));
        let pos = make_open_position("long_spread");
        let signal = signal_gen
            .generate_signal(
                &spread,
                Decimal::from(2),
                Decimal::from(50) / Decimal::from(100),
                Some(&pos),
            )
            .unwrap();
        assert_eq!(signal, Some(SignalType::Exit));
    }

    #[test]
    fn test_stop_loss_when_continuation() {
        let signal_gen = SignalGenerator::default_with_entry_threshold(
            Decimal::from(2),
            Decimal::from(50) / Decimal::from(100),
        );

        // 持仓 long_spread，z_score 继续下跌到 -5（超过 -4，止损）
        let spread = make_live_spread(Some(Decimal::from(-5)));
        let pos = make_open_position("long_spread");
        let signal = signal_gen
            .generate_signal(
                &spread,
                Decimal::from(2),
                Decimal::from(50) / Decimal::from(100),
                Some(&pos),
            )
            .unwrap();
        assert_eq!(signal, Some(SignalType::StopLoss));
    }

    #[test]
    fn test_signal_to_direction() {
        assert_eq!(
            SignalGenerator::signal_to_direction(&SignalType::EntryLong),
            SignalDirection::LongSpread
        );
        assert_eq!(
            SignalGenerator::signal_to_direction(&SignalType::EntryShort),
            SignalDirection::ShortSpread
        );
        assert_eq!(
            SignalGenerator::signal_to_direction(&SignalType::Exit),
            SignalDirection::Neutral
        );
    }
}
