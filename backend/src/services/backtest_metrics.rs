//! Backtest metrics — supplemental indicators beyond the base Sharpe/Sortino/Calmar
//! set. These are exposed to the API and persisted as part of the
//! `BacktestMetrics` JSON.
//!
//! P2-2 implementation: alpha, beta, information ratio, Kelly %, expectancy,
//! turnover, slippage estimation, and drawdown recovery days.
//!
//! All functions are pure and stateless — they take slices/values and return
//! `f64` (or `i64` for day counts). Edge cases (empty inputs, zero denominators)
//! return `0.0`/`0` instead of panicking, matching the rest of the engine.

use crate::models::backtest::{EquityPoint, TradeRecord};

/// Compute Jensen's alpha and beta versus a benchmark return series.
///
/// * `strategy_returns` — period-over-period returns of the strategy
///   (e.g. daily returns from the equity curve, in decimal: 0.01 = +1%).
/// * `benchmark_returns` — same-shape returns for the benchmark
///   (BTC 1d, in decimal).
///
/// Beta = `Cov(strategy, benchmark) / Var(benchmark)`.  
/// Alpha = `(mean(strategy) - beta * mean(benchmark)) * periods_per_year`,
///   annualized. `periods_per_year` is typically `365` for daily returns.
///
/// Returns `(alpha, beta)`. Both are `0.0` when fewer than 2 returns are
/// supplied or when the benchmark variance is zero.
pub fn alpha_beta(
    strategy_returns: &[f64],
    benchmark_returns: &[f64],
    periods_per_year: f64,
) -> (f64, f64) {
    let n = strategy_returns.len().min(benchmark_returns.len());
    if n < 2 {
        return (0.0, 0.0);
    }

    let n_f = n as f64;
    let mean_s: f64 = strategy_returns.iter().take(n).sum::<f64>() / n_f;
    let mean_b: f64 = benchmark_returns.iter().take(n).sum::<f64>() / n_f;

    let mut cov = 0.0;
    let mut var_b = 0.0;
    for i in 0..n {
        let ds = strategy_returns[i] - mean_s;
        let db = benchmark_returns[i] - mean_b;
        cov += ds * db;
        var_b += db * db;
    }

    if var_b <= 0.0 {
        return (0.0, 0.0);
    }

    let beta = cov / var_b;
    // Annualize the abnormal return: mean(active) * periods_per_year
    let active_daily = mean_s - beta * mean_b;
    let alpha = active_daily * periods_per_year;
    (alpha, beta)
}

/// Information ratio: average active return divided by tracking error (std-dev of
/// active returns), annualized.
///
/// * `strategy_returns` — strategy period returns (decimal).
/// * `benchmark_returns` — benchmark period returns (decimal).
///
/// `IR = (mean(strategy) - mean(benchmark)) / std(active_returns) * sqrt(periods_per_year)`.
///
/// Returns `0.0` if fewer than 2 returns, zero tracking error, or zero overlap.
pub fn information_ratio(
    strategy_returns: &[f64],
    benchmark_returns: &[f64],
    periods_per_year: f64,
) -> f64 {
    let n = strategy_returns.len().min(benchmark_returns.len());
    if n < 2 {
        return 0.0;
    }

    let n_f = n as f64;
    let mean_s: f64 = strategy_returns.iter().take(n).sum::<f64>() / n_f;
    let mean_b: f64 = benchmark_returns.iter().take(n).sum::<f64>() / n_f;
    let active_mean = mean_s - mean_b;

    let variance: f64 = (0..n)
        .map(|i| {
            let active_i = strategy_returns[i] - benchmark_returns[i];
            (active_i - active_mean).powi(2)
        })
        .sum::<f64>()
        / (n_f - 1.0);
    let tracking_error = variance.sqrt();

    // Guard against floating-point near-zero (constant active returns
    // produce tiny non-zero variance due to rounding). Use a relative epsilon.
    if tracking_error < 1e-12 || active_mean.abs() < 1e-12 {
        return 0.0;
    }

    active_mean / tracking_error * periods_per_year.sqrt()
}

/// Kelly criterion (full-Kelly) fraction, expressed as a percent.
///
/// `Kelly% = W - (1 - W) / R`, where `W = win_rate` (decimal) and
/// `R = avg_win / avg_loss` (both as positive magnitudes in percent).
///
/// Returns `0.0` when there are no wins AND no losses, or when `avg_loss == 0`.
/// Result is clamped to `[-100.0, 100.0]` to avoid extreme / nonsensical
/// fractions from very small win/loss samples.
///
/// Inputs are percentages (e.g. `60.0` for 60%, `5.0` for 5%) — matching
/// `BacktestMetrics::win_rate` and `avg_win_pct`/`avg_loss_pct`.
pub fn kelly_pct(win_rate_pct: f64, avg_win_pct: f64, avg_loss_pct: f64) -> f64 {
    if win_rate_pct <= 0.0 && avg_win_pct <= 0.0 && avg_loss_pct <= 0.0 {
        return 0.0;
    }
    if avg_loss_pct.abs() < 1e-9 {
        return 0.0;
    }
    let w = (win_rate_pct / 100.0).clamp(0.0, 1.0);
    let r = avg_win_pct.abs() / avg_loss_pct.abs();
    if r <= 0.0 {
        return 0.0;
    }
    let kelly = w - (1.0 - w) / r;
    // Convert to percent and clamp
    (kelly * 100.0).clamp(-100.0, 100.0)
}

/// Expectancy per trade in percent:
/// `expectancy = (win_rate/100) * avg_win_pct - ((1 - win_rate/100) * |avg_loss_pct|)`.
///
/// Returns `0.0` when no trades (caller should guard). Inputs are percentages
/// to match the rest of the metric set.
pub fn expectancy(win_rate_pct: f64, avg_win_pct: f64, avg_loss_pct: f64) -> f64 {
    let w = (win_rate_pct / 100.0).clamp(0.0, 1.0);
    w * avg_win_pct - (1.0 - w) * avg_loss_pct.abs()
}

/// Portfolio turnover (decimal, NOT percent) for a series of trades.
///
/// `turnover = sum(|trade_value|) / average_equity / period_days`,
/// where `trade_value = |pnl_usdt|` (USD notional approximation per trade
/// for backtests that don't track explicit position notional in the trade
/// record), and `period_days` is the span of the equity curve.
///
/// Returns `0.0` when there are no trades, the equity curve is empty, the
/// span is zero, or the average equity is non-positive.
pub fn turnover(trades: &[TradeRecord], equity: &[EquityPoint]) -> f64 {
    if trades.is_empty() || equity.len() < 2 {
        return 0.0;
    }
    let sum_abs: f64 = trades.iter().map(|t| t.pnl_usdt.abs()).sum();
    let avg_equity: f64 = equity.iter().map(|e| e.equity).sum::<f64>() / equity.len() as f64;
    if avg_equity <= 0.0 {
        return 0.0;
    }
    let first = equity.first().map(|e| e.time).unwrap_or(0);
    let last = equity.last().map(|e| e.time).unwrap_or(0);
    let days = (last - first) as f64 / 86_400_000.0;
    if days <= 0.0 {
        return 0.0;
    }
    sum_abs / avg_equity / days
}

/// Average slippage per trade in USDT, exposed as a separate diagnostic.
///
/// Returns `0.0` when there are no trades.
pub fn slippage_estimation(trades: &[TradeRecord]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }
    let total: f64 = trades.iter().map(|t| t.slippage).sum();
    total / trades.len() as f64
}

/// Number of days for the equity curve to recover from the maximum-drawdown
/// trough back to the pre-drawdown peak (or higher).
///
/// Algorithm:
/// 1. Walk the equity curve, find the largest drawdown by `peak - equity`.
/// 2. Record the trough index and the peak value at the time of the trough.
/// 3. Continue walking; return the index of the first bar where equity >= that
///    peak value.
/// 4. Recovery days = (recover_time - trough_time) / 86_400_000 ms.
///
/// Returns `0` when there is no drawdown, fewer than 2 equity points, or when
/// the curve never recovers (still in drawdown at the end).
pub fn drawdown_recovery_days(equity: &[EquityPoint]) -> i64 {
    if equity.len() < 2 {
        return 0;
    }
    let mut running_peak = f64::MIN;
    let mut max_dd = 0.0_f64;
    let mut peak_value_at_trough: f64 = 0.0;
    let mut trough_idx: usize = 0;
    for (i, ep) in equity.iter().enumerate() {
        // Update running peak BEFORE evaluating drawdown so peak_value_at_trough
        // captures the peak that was in effect at the trough.
        if ep.equity > running_peak {
            running_peak = ep.equity;
        }
        let dd = running_peak - ep.equity;
        if dd > max_dd {
            max_dd = dd;
            peak_value_at_trough = running_peak;
            trough_idx = i;
        }
    }
    if max_dd <= 0.0 {
        return 0;
    }
    // Walk forward from trough to find first bar where equity >= peak_value_at_trough
    for ep in equity.iter().skip(trough_idx + 1) {
        if ep.equity >= peak_value_at_trough {
            let trough_time = equity[trough_idx].time;
            return (ep.time - trough_time) / 86_400_000;
        }
    }
    // Never recovered
    0
}

/// Align strategy returns to a benchmark series by timestamp.
///
/// The strategy and benchmark may have slightly different time buckets (e.g.
/// a 1h ETH backtest aligned to 1d BTC closes). We pick the benchmark return
/// whose timestamp is the *latest* one ≤ the strategy return's timestamp —
/// the convention used by most quant backtest libraries (point-in-time
/// availability).
///
/// Returns two parallel `Vec<f64>` of equal length. Length is `min(strategy, benchmark)`
/// when at least one benchmark sample exists for each strategy sample; empty
/// when the alignment yields no overlap.
pub fn align_returns_by_time(
    strategy: &[(i64, f64)],
    benchmark: &[(i64, f64)],
) -> (Vec<f64>, Vec<f64>) {
    if strategy.is_empty() || benchmark.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut aligned_s = Vec::with_capacity(strategy.len());
    let mut aligned_b = Vec::with_capacity(strategy.len());
    let mut b_idx = 0usize;
    let mut last_valid_b: Option<f64> = None;

    for &(t, s) in strategy {
        while b_idx < benchmark.len() && benchmark[b_idx].0 <= t {
            last_valid_b = Some(benchmark[b_idx].1);
            b_idx += 1;
        }
        if let Some(b) = last_valid_b {
            aligned_s.push(s);
            aligned_b.push(b);
        }
        // If no benchmark yet, skip the strategy sample (warmup period).
    }
    (aligned_s, aligned_b)
}

/// Compute period-over-period returns from a value series.
///
/// `returns[i] = (values[i+1] - values[i]) / values[i]`.
/// Returns a vector of length `values.len() - 1`.
pub fn period_returns(values: &[f64]) -> Vec<f64> {
    if values.len() < 2 {
        return Vec::new();
    }
    values
        .windows(2)
        .map(|w| if w[0] != 0.0 { (w[1] - w[0]) / w[0] } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::backtest::{Direction, EquityPoint, TradeRecord};

    fn trade(pnl: f64, slip: f64) -> TradeRecord {
        TradeRecord {
            entry_time: 0,
            exit_time: 0,
            direction: "long".to_string(),
            entry_price: 100.0,
            exit_price: 105.0,
            quantity: 1.0,
            pnl_usdt: pnl,
            pnl_pct: pnl,
            holding_period_ms: 0,
            exit_reason: "signal".to_string(),
            fee: 0.0,
            slippage: slip,
            hit_limit: false,
            limit_type: None,
        }
    }

    fn equity_pt(time_ms: i64, eq: f64) -> EquityPoint {
        EquityPoint {
            time: time_ms,
            equity: eq,
            drawdown_pct: 0.0,
        }
    }

    #[test]
    fn alpha_beta_basic() {
        // Strategy tracks benchmark exactly: beta = 1, alpha = 0
        let s = vec![0.01, 0.02, -0.005, 0.03, 0.015];
        let b = vec![0.01, 0.02, -0.005, 0.03, 0.015];
        let (alpha, beta) = alpha_beta(&s, &b, 365.0);
        assert!((beta - 1.0).abs() < 1e-9, "beta should be 1.0, got {}", beta);
        assert!(alpha.abs() < 1e-9, "alpha should be ~0, got {}", alpha);
    }

    #[test]
    fn alpha_beta_with_outperformance() {
        // Strategy outperforms by a constant 0.5% per day → alpha = 0.5% * 365 = 1.825 (decimal)
        let s = vec![0.015, 0.025, 0.000, 0.035, 0.020];
        let b = vec![0.010, 0.020, -0.005, 0.030, 0.015];
        let (alpha, beta) = alpha_beta(&s, &b, 365.0);
        assert!((beta - 1.0).abs() < 1e-9, "beta should be ~1.0, got {}", beta);
        // 0.005 * 365 = 1.825
        assert!(
            (alpha - 1.825).abs() < 1e-6,
            "alpha should be ~1.825, got {}",
            alpha
        );
    }

    #[test]
    fn alpha_beta_empty() {
        let (alpha, beta) = alpha_beta(&[], &[0.01, 0.02], 365.0);
        assert_eq!(alpha, 0.0);
        assert_eq!(beta, 0.0);
    }

    #[test]
    fn information_ratio_basic() {
        // If active returns are all zero, IR = 0
        let s = vec![0.01, 0.02, 0.03];
        let b = vec![0.01, 0.02, 0.03];
        let ir = information_ratio(&s, &b, 365.0);
        assert!(ir.abs() < 1e-9, "expected ~0, got {}", ir);
    }

    #[test]
    fn information_ratio_with_alpha() {
        // Strategy = benchmark + 0.01 each day
        let s = vec![0.02, 0.03, 0.04, 0.05];
        let b = vec![0.01, 0.02, 0.03, 0.04];
        let ir = information_ratio(&s, &b, 365.0);
        // Constant active return of 0.01 → std(active) = 0 → IR = 0
        // (this is by design: zero tracking error)
        assert!(ir.abs() < 1e-9, "expected 0 for zero tracking error, got {}", ir);
    }

    #[test]
    fn kelly_pct_basic() {
        // 60% win rate, 2:1 reward:risk → kelly = 0.6 - 0.4/2 = 0.4 = 40%
        let k = kelly_pct(60.0, 10.0, 5.0);
        assert!((k - 40.0).abs() < 1e-9, "expected 40.0, got {}", k);
    }

    #[test]
    fn kelly_pct_clamps() {
        // Win rate 100%, infinite reward → kelly should be clamped at 100
        let k = kelly_pct(100.0, 100.0, 1.0);
        assert!((k - 100.0).abs() < 1e-9, "expected 100.0, got {}", k);
    }

    #[test]
    fn kelly_pct_no_trades() {
        let k = kelly_pct(0.0, 0.0, 0.0);
        assert_eq!(k, 0.0);
    }

    #[test]
    fn expectancy_basic() {
        // 60% win, avg win 5%, avg loss -3%
        // expectancy = 0.6 * 5 - 0.4 * 3 = 3.0 - 1.2 = 1.8
        let e = expectancy(60.0, 5.0, -3.0);
        assert!((e - 1.8).abs() < 1e-9, "expected 1.8, got {}", e);
    }

    #[test]
    fn turnover_with_trades() {
        let trades = vec![trade(100.0, 1.0), trade(-50.0, 1.0), trade(75.0, 1.0)];
        // 5 days, avg equity 10000
        let equity: Vec<EquityPoint> = (0..6)
            .map(|i| equity_pt(i as i64 * 86_400_000, 10_000.0))
            .collect();
        let t = turnover(&trades, &equity);
        // sum_abs = 225, avg_equity = 10000, days = 5
        // 225 / 10000 / 5 = 0.0045
        assert!((t - 0.0045).abs() < 1e-9, "expected 0.0045, got {}", t);
    }

    #[test]
    fn turnover_no_trades() {
        let equity: Vec<EquityPoint> = (0..3)
            .map(|i| equity_pt(i as i64 * 86_400_000, 100.0))
            .collect();
        assert_eq!(turnover(&[], &equity), 0.0);
    }

    #[test]
    fn slippage_estimation_avg() {
        let trades = vec![trade(0.0, 2.0), trade(0.0, 4.0), trade(0.0, 6.0)];
        let s = slippage_estimation(&trades);
        assert!((s - 4.0).abs() < 1e-9, "expected 4.0, got {}", s);
    }

    #[test]
    fn slippage_estimation_empty() {
        assert_eq!(slippage_estimation(&[]), 0.0);
    }

    #[test]
    fn drawdown_recovery_days_basic() {
        // Peak at 100, trough at 80, recovery back to 100 after 3 days
        let equity = vec![
            equity_pt(0, 100.0),
            equity_pt(86_400_000, 90.0),
            equity_pt(2 * 86_400_000, 80.0), // trough
            equity_pt(3 * 86_400_000, 90.0),
            equity_pt(4 * 86_400_000, 95.0),
            equity_pt(5 * 86_400_000, 100.0), // recovered
        ];
        let d = drawdown_recovery_days(&equity);
        assert_eq!(d, 3, "expected 3 days, got {}", d);
    }

    #[test]
    fn drawdown_recovery_days_no_drawdown() {
        let equity = vec![
            equity_pt(0, 100.0),
            equity_pt(86_400_000, 110.0),
            equity_pt(2 * 86_400_000, 120.0),
        ];
        assert_eq!(drawdown_recovery_days(&equity), 0);
    }

    #[test]
    fn drawdown_recovery_days_unrecovered() {
        // Still in drawdown at the end → 0
        let equity = vec![
            equity_pt(0, 100.0),
            equity_pt(86_400_000, 90.0),
            equity_pt(2 * 86_400_000, 80.0),
        ];
        assert_eq!(drawdown_recovery_days(&equity), 0);
    }

    #[test]
    fn period_returns_basic() {
        let v = vec![100.0, 110.0, 99.0];
        let r = period_returns(&v);
        assert_eq!(r.len(), 2);
        assert!((r[0] - 0.10).abs() < 1e-9);
        assert!((r[1] - (-0.10)).abs() < 1e-9);
    }

    #[test]
    fn period_returns_handles_zero() {
        let v = vec![0.0, 100.0];
        let r = period_returns(&v);
        // Should be 0.0 not NaN/Inf
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], 0.0);
    }

    #[test]
    fn align_returns_by_time_basic() {
        // Strategy returns hourly, benchmark daily — align by latest benchmark ≤ strategy time
        let strategy = vec![(0_i64, 0.01_f64), (3_600_000, 0.02), (7_200_000, 0.015)];
        let benchmark = vec![(0_i64, 0.005_f64), (86_400_000, 0.01)];
        let (s, b) = align_returns_by_time(&strategy, &benchmark);
        // First two strategy samples have benchmark t=0 (0.005), the last has t=0 (0.005)
        assert_eq!(s.len(), 3);
        assert_eq!(b.len(), 3);
        assert!((b[0] - 0.005).abs() < 1e-9);
        // Last one still picks up the most recent benchmark ≤ 7.2M, which is 0 at 0
        assert!((b[2] - 0.005).abs() < 1e-9);
    }

    #[test]
    fn align_returns_skips_warmup() {
        // Strategy has samples before any benchmark exists → skipped
        let strategy = vec![(100_i64, 0.01_f64), (200, 0.02)];
        let benchmark = vec![(300_i64, 0.005_f64), (400, 0.01)];
        let (s, b) = align_returns_by_time(&strategy, &benchmark);
        // After strategy t=100, no benchmark ≤ 100. After t=200, no benchmark ≤ 200.
        // First benchmark available is at t=300, so no overlap → empty
        assert_eq!(s.len(), 0);
        assert_eq!(b.len(), 0);
    }

    // Compile-time check that Direction is used (keeps imports honest)
    #[allow(dead_code)]
    fn _dir_used() -> Direction {
        Direction::Long
    }
}
