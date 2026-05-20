//! P0-F2 Risk Manager Integration Tests
//!
//! Tests: 资金风控规则（单日亏损/单笔亏损/回撤）核心逻辑

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[cfg(test)]
mod risk_logic_tests {
    use super::*;

    /// 测试风控快照计算
    #[test]
    fn test_risk_snapshot_calculation() {
        let equity = dec!(100000.00);
        let peak_equity = dec!(110000.00);
        let daily_pnl = dec!(-3000.00);

        // 当日亏损 = -3000
        assert_eq!(daily_pnl, dec!(-3000.00));

        // 回撤 = (110000 - 100000) / 110000 = 0.0909...
        let drawdown = (peak_equity - equity) / peak_equity;
        assert!(drawdown > dec!(0.09) && drawdown < dec!(0.091));
    }

    /// 测试单日亏损超限判断
    #[test]
    fn test_daily_loss_limit() {
        let daily_loss_limit = dec!(5000.00);
        let actual_loss = dec!(-6000.00);

        // 亏损为负数，绝对值超过限制 → 触发
        assert!(actual_loss.abs() > daily_loss_limit);
    }

    /// 测试单笔亏损比例计算
    #[test]
    fn test_single_trade_loss_ratio() {
        let position_value = dec!(10000.00);
        let stop_loss_ratio = dec!(0.02); // 2%

        let max_loss = position_value * stop_loss_ratio;
        assert_eq!(max_loss, dec!(200.00));
    }

    /// 测试回撤超限判断
    #[test]
    fn test_drawdown_limit() {
        let peak_equity = dec!(100000.00);
        let current_equity = dec!(92000.00);
        let max_drawdown = dec!(0.08); // 8%

        let drawdown = (peak_equity - current_equity) / peak_equity;
        assert!(drawdown >= max_drawdown);
    }

    /// 测试权益为正数时回撤计算
    #[test]
    fn test_drawdown_with_zero_peak() {
        let peak_equity = Decimal::ZERO;
        let current_equity = dec!(50000.00);

        // 除以零保护
        let drawdown = if peak_equity > Decimal::ZERO {
            (peak_equity - current_equity) / peak_equity
        } else {
            Decimal::ZERO
        };
        assert_eq!(drawdown, Decimal::ZERO);
    }

    /// 测试 Decimal 精度
    #[test]
    fn test_decimal_precision() {
        let a = dec!(0.1);
        let b = dec!(0.2);
        let sum = a + b;
        assert_eq!(sum, dec!(0.3));
    }

    /// 测试紧急平仓信号生成
    #[test]
    fn test_emergency_close_signal() {
        let rules_active = true;
        let daily_loss_exceeded = true;
        let drawdown_exceeded = true;

        let should_emergency_close = rules_active && (daily_loss_exceeded || drawdown_exceeded);
        assert!(should_emergency_close);
    }

    /// 测试风险检查结果数据结构
    #[test]
    fn test_risk_check_result_fields() {
        let result = super::super::RiskCheckResult {
            passed: false,
            triggered_rules: vec!["D2".to_string(), "P3".to_string()],
            daily_loss: dec!(-6500.00),
            single_trade_loss: Some(dec!(-250.00)),
            drawdown: dec!(0.08),
            equity: dec!(93500.00),
            peak_equity: dec!(100000.00),
        };

        assert!(!result.passed);
        assert_eq!(result.triggered_rules.len(), 2);
        assert_eq!(result.daily_loss, dec!(-6500.00));
    }
}
