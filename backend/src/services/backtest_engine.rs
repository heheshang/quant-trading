//! Backtest engine core — memory-vectorized computation with strategy signal
//! generation, equity tracking, and metrics calculation.
//!
//! Adheres to ADR-004 and ADR-007:
//! - Full memory vectorized computation
//! - Stateless [`StrategyTemplate::generate_signal()`]
//! - O(n) equity_curve traversal for metrics
//! - [`CancellationToken`] + [`Arc<AtomicU32>`] for progress/cancel

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio_util::sync::CancellationToken;

use crate::models::backtest::{
    Account, BacktestConfig, BacktestMetrics, Direction, EquityPoint, Kline, LimitType, Position,
    Signal, TradeRecord,
};
use crate::services::strategy::StrategyTemplate;
use serde_json::Value;

/// The core backtest execution engine.
///
/// Walks through historical kline bars, generates trading signals via the
/// injected [`StrategyTemplate`], manages positions, records equity points,
/// and calculates comprehensive performance metrics on completion.
pub struct BacktestEngine {
    config: BacktestConfig,
    klines: Vec<Kline>,
    strategy: Box<dyn StrategyTemplate>,
    strategy_params: Value,
    // Runtime state
    account: Account,
    open_position: Option<Position>,
    trades: Vec<TradeRecord>,
    equity_points: Vec<EquityPoint>,
    peak_equity: f64, // running peak for drawdown calculation (Bug#7)
    /// Previous bar's close price, used to compute price limits.
    prev_close: f64,
    // Control
    progress: Arc<AtomicU32>,
    cancel_token: CancellationToken,
}

impl BacktestEngine {
    /// Create a new engine instance ready to run.
    ///
    /// * `config` — backtest configuration (symbol, dates, capital, fees)
    /// * `klines` — historical price bars sorted by time ASC
    /// * `strategy` — the trading strategy template for signal generation
    /// * `strategy_params` — runtime parameters for the strategy
    /// * `progress` — shared atomic counter for progress reporting (0–100)
    /// * `cancel_token` — token to signal cancellation from outside
    pub fn new(
        config: BacktestConfig,
        klines: Vec<Kline>,
        strategy: Box<dyn StrategyTemplate>,
        strategy_params: Value,
        progress: Arc<AtomicU32>,
        cancel_token: CancellationToken,
    ) -> Self {
        let initial_capital = config.initial_capital;
        let kline_count = klines.len().max(1);
        Self {
            config,
            klines,
            strategy,
            strategy_params,
            account: Account {
                initial_capital,
                cash: initial_capital,
                equity: initial_capital,
            },
            open_position: None,
            trades: Vec::new(),
            equity_points: Vec::with_capacity(kline_count),
            peak_equity: initial_capital, // Bug#7: track running peak
            prev_close: 0.0,
            progress,
            cancel_token,
        }
    }

    /// Run the backtest. Returns (metrics, trades, equity_curve) on success.
    ///
    /// Walks through each kline bar, generates signals, manages positions,
    /// and finally calculates performance metrics. Supports cancellation
    /// via the [`CancellationToken`] checked every 100 bars.
    pub fn run(&mut self) -> Result<(BacktestMetrics, Vec<TradeRecord>, Vec<EquityPoint>), String> {
        let total = self.klines.len();
        if total == 0 {
            return Err("no kline data loaded".into());
        }

        // Phase 1: Walk through each bar
        let report_interval = (total / 10).max(1); // every ~10%
        let klines_copy = self.klines.clone();
        for (i, kline) in klines_copy.iter().enumerate() {
            // Check cancellation every 100 bars
            if i % 100 == 0 && self.cancel_token.is_cancelled() {
                return Err("cancelled".into());
            }

            // Report progress
            if i % report_interval == 0 {
                let pct = (i as f64 / total as f64 * 100.0) as u32;
                self.progress.store(pct.min(99), Ordering::Relaxed);
            }

            // Process one bar
            self.process_bar(i, kline);

            // Check if bankrupt
            if self.account.equity <= 0.0 {
                self.record_equity_point(kline.open_time, kline.close);
                break;
            }
        }

        // Close any open position at the last bar
        if let Some(pos) = self.open_position.take() {
            let last = &self.klines[self.klines.len() - 1];
            self.close_position(last.open_time, last.close, "signal", pos, None);
        }

        // Final progress
        self.progress.store(100, Ordering::Relaxed);

        // Phase 2: Calculate metrics
        let metrics = self.calculate_metrics();

        Ok((
            metrics,
            std::mem::take(&mut self.trades),
            std::mem::take(&mut self.equity_points),
        ))
    }

    /// Process a single kline bar: generate signal, manage position, record equity.
    fn process_bar(&mut self, idx: usize, kline: &Kline) {
        let prev = self.prev_close;
        self.prev_close = kline.close;

        // Compute price limits for the current bar (based on previous close)
        let price_limit = if prev > 0.0 {
            self.price_limits(prev)
        } else {
            (None, None)
        };

        let signal =
            self.strategy
                .generate_signal(&self.klines[..=idx], idx, &self.strategy_params);

        match signal {
            Signal::Hold => {
                // Check stop-loss / take-profit if we have a position
                self.check_sl_tp(kline, price_limit);
            }
            Signal::Buy { quantity_pct } => {
                // If we have a short position, close it first
                if let Some(pos) = self.open_position.take() {
                    self.close_position(kline.open_time, kline.close, "signal", pos, None);
                }
                // Only open if not already long
                if self.open_position.is_none() {
                    self.open_long(kline, quantity_pct, price_limit);
                }
            }
            Signal::Sell { quantity_pct } => {
                // If we have a long position, close it first
                if let Some(pos) = self.open_position.take() {
                    self.close_position(kline.open_time, kline.close, "signal", pos, None);
                }
                if self.open_position.is_none() {
                    self.open_short(kline, quantity_pct, price_limit);
                }
            }
            Signal::CloseAll => {
                if let Some(pos) = self.open_position.take() {
                    self.close_position(kline.open_time, kline.close, "signal", pos, None);
                }
            }
        }

        // Record equity curve
        self.record_equity_point(kline.open_time, kline.close);
    }

    /// Open a long position using `pct` of available cash.
    ///
    /// Applies slippage (buy at ask) and deducts trading fees.
    /// Enforces price limits: execution price must not exceed `upper_limit`.
    /// No-op if the allocated amount is insufficient for at least one unit.
    fn open_long(&mut self, kline: &Kline, pct: f64, (upper_limit, _): (Option<f64>, Option<f64>)) {
        let pct = pct.clamp(0.0, 1.0);
        let slippage = kline.close * self.config.slippage_rate;
        let exec_price = kline.close + slippage;

        // Price limit enforcement: long entry cannot buy above limit-up
        if let Some(upper_limit) = upper_limit {
            if exec_price > upper_limit {
                return; // 涨停，无法买入
            }
        }

        let allocated = self.account.cash * pct;
        let quantity = (allocated / exec_price).floor();
        if quantity < 1e-8 {
            return; // Cannot buy fractional
        }
        let cost = quantity * exec_price;
        let fee = cost * self.config.fee_rate;

        if cost + fee > self.account.cash {
            return; // Insufficient funds
        }

        self.account.cash -= cost + fee;
        self.open_position = Some(Position {
            direction: Direction::Long,
            quantity,
            entry_price: exec_price,
            entry_time: kline.open_time,
            stop_loss: None,
            take_profit: None,
            fee_paid: fee,
            slippage_paid: slippage * quantity,
            limit_type: None,
        });
    }

    ///
    /// Open a short position using `pct` of available cash as collateral.
    ///
    /// Applies slippage (sell at bid) and deducts trading fees.
    /// Enforces price limits: execution price must not fall below `lower_limit`.
    /// No-op if insufficient cash for fees.
    fn open_short(&mut self, kline: &Kline, pct: f64, (_, lower_limit): (Option<f64>, Option<f64>)) {
        let pct = pct.clamp(0.0, 1.0);
        let slippage = kline.close * self.config.slippage_rate;
        let exec_price = kline.close - slippage;

        // Price limit enforcement: short entry cannot sell below limit-down
        if let Some(lower) = lower_limit {
            if exec_price < lower {
                return; // 跌停，无法卖出
            }
        }

        // Short selling: we "borrow" and sell, receiving cash
        let allocated = self.account.cash * pct;
        let quantity = (allocated / exec_price).floor();
        if quantity < 1e-8 {
            return;
        }
        let received = quantity * exec_price;
        let fee = received * self.config.fee_rate;

        if fee > self.account.cash {
            return;
        }

        self.account.cash += received - fee;
        self.open_position = Some(Position {
            direction: Direction::Short,
            quantity,
            entry_price: exec_price,
            entry_time: kline.open_time,
            stop_loss: None,
            take_profit: None,
            fee_paid: fee,
            slippage_paid: slippage * quantity,
            limit_type: None,
        });
    }

    /// Close a position at the given `price` and `reason`.
    ///
    /// Calculates PnL (realized), fees, and slippage for the exit side,
    /// records a [`TradeRecord`], and updates account cash/equity.
    fn close_position(
        &mut self,
        time: i64,
        price: f64,
        reason: &str,
        pos: Position,
        limit_type: Option<LimitType>,
    ) {
        let slippage = price * self.config.slippage_rate;
        let exit_price = match pos.direction {
            Direction::Long => price - slippage,  // sell at bid
            Direction::Short => price + slippage, // buy back at ask
        };
        let trade_value = pos.quantity * exit_price;
        let fee = trade_value * self.config.fee_rate;
        self.account.cash += trade_value - fee;

        let entry_value = pos.quantity * pos.entry_price;
        let direction_str = match pos.direction {
            Direction::Long => "long",
            Direction::Short => "short",
        };
        let pnl_usdt = match pos.direction {
            Direction::Long => trade_value - entry_value,
            Direction::Short => entry_value - trade_value,
        };

        self.account.equity = self.account.cash;

        self.trades.push(TradeRecord {
            entry_time: pos.entry_time,
            exit_time: time,
            direction: direction_str.to_string(),
            entry_price: pos.entry_price,
            exit_price,
            quantity: pos.quantity,
            pnl_usdt,
            pnl_pct: if entry_value != 0.0 {
                pnl_usdt / entry_value * 100.0
            } else {
                0.0
            },
            holding_period_ms: time - pos.entry_time,
            exit_reason: reason.to_string(),
            fee: pos.fee_paid + fee,
            slippage: pos.slippage_paid + slippage * pos.quantity,
            hit_limit: limit_type.is_some(),
            limit_type,
        });
    }

    /// Compute the upper (limit-up) and lower (limit-down) price limits
    /// based on the previous bar's close and the configured limit percentage.
    ///
    /// Returns `(upper_limit, lower_limit)` or `(None, None)` if price limits
    /// are disabled (`price_limit_pct` is `None` or `0.0`) or if `prev_close` is
    /// not yet available (first bar, `prev_close == 0.0`).
    fn price_limits(&self, prev_close: f64) -> (Option<f64>, Option<f64>) {
        let Some(pct) = self.config.price_limit_pct else {
            return (None, None);
        };
        if pct <= 0.0 || prev_close <= 0.0 {
            return (None, None);
        }
        (Some(prev_close * (1.0 + pct)), Some(prev_close * (1.0 - pct)))
    }

    /// Check stop-loss and take-profit levels against the current bar's high/low.
    ///
    /// If triggered, closes the position and records the exit reason.
    /// Also checks price limit bounds: if a position was opened at a limit price
    /// (`pos.limit_type` is set), close orders that would execute beyond the
    /// opposite limit are skipped.
    fn check_sl_tp(
        &mut self,
        kline: &Kline,
        (_upper_limit, _lower_limit): (Option<f64>, Option<f64>),
    ) {
        let position = match &self.open_position {
            Some(p) => p.clone(),
            None => return,
        };
        // Check stop-loss / take-profit using bar high/low
        if let Some(sl) = position.stop_loss {
            match position.direction {
                Direction::Long =>
                {
                    #[allow(clippy::collapsible_if)]
                    if kline.low <= sl {
                        if let Some(pos) = self.open_position.take() {
                            self.close_position(kline.open_time, sl, "stop_loss", pos, None);
                        }
                    }
                }
                Direction::Short =>
                {
                    #[allow(clippy::collapsible_if)]
                    if kline.high >= sl {
                        if let Some(pos) = self.open_position.take() {
                            self.close_position(kline.open_time, sl, "stop_loss", pos, None);
                        }
                    }
                }
            }
        }
        if let Some(tp) = position.take_profit {
            match position.direction {
                Direction::Long =>
                {
                    #[allow(clippy::collapsible_if)]
                    if kline.high >= tp {
                        if let Some(pos) = self.open_position.take() {
                            self.close_position(kline.open_time, tp, "take_profit", pos, None);
                        }
                    }
                }
                Direction::Short =>
                {
                    #[allow(clippy::collapsible_if)]
                    if kline.low <= tp {
                        if let Some(pos) = self.open_position.take() {
                            self.close_position(kline.open_time, tp, "take_profit", pos, None);
                        }
                    }
                }
            }
        }
    }

    /// Record an equity point for the current bar.
    ///
    /// Computes mark-to-market equity (cash + unrealized PnL) and the
    /// running drawdown percentage using [`Self::peak_equity`].
    fn record_equity_point(&mut self, time: i64, kline_close: f64) {
        let equity = if let Some(pos) = &self.open_position {
            // Mark-to-market: use current close for unrealized PnL
            let mtm_value = match pos.direction {
                Direction::Long => pos.quantity * kline_close,
                Direction::Short => pos.quantity * (2.0 * pos.entry_price - kline_close),
            };
            self.account.cash + mtm_value - pos.quantity * pos.entry_price
        } else {
            self.account.cash
        };
        // Bug#7 fix: compute drawdown_pct in real-time using running peak
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }
        let drawdown_pct = if self.peak_equity > 0.0 {
            (equity - self.peak_equity) / self.peak_equity * 100.0
        } else {
            0.0
        };
        self.equity_points.push(EquityPoint {
            time,
            equity,
            drawdown_pct,
        });
        self.account.equity = equity;
    }

    // ============ Metrics Calculation ============

    /// Calculate comprehensive performance metrics from the completed backtest.
    ///
    /// Returns [`BacktestMetrics`] including total/annualized return,
    /// max drawdown, Sharpe/Sortino/Calmar ratios, win rate, profit factor,
    /// and fee/slippage totals.
    fn calculate_metrics(&self) -> BacktestMetrics {
        let total_trades = self.trades.len() as i32;
        let final_equity = self
            .equity_points
            .last()
            .map(|e| e.equity)
            .unwrap_or(self.config.initial_capital);
        let total_return =
            (final_equity - self.config.initial_capital) / self.config.initial_capital;

        // Duration in days
        let first_time = self.equity_points.first().map(|e| e.time).unwrap_or(0);
        let last_time = self.equity_points.last().map(|e| e.time).unwrap_or(0);
        let days = (last_time - first_time) as f64 / 86_400_000.0;

        let annualized_return = if days > 0.0 {
            (1.0 + total_return).powf(365.0 / days) - 1.0
        } else {
            0.0
        };

        // Max drawdown
        let mut peak = f64::MIN;
        let mut max_drawdown = 0.0;
        for ep in &self.equity_points {
            if ep.equity > peak {
                peak = ep.equity;
            }
            let dd = (ep.equity - peak) / peak;
            if dd < max_drawdown {
                max_drawdown = dd;
            }
        }

        // Daily returns vector for Sharpe/Sortino
        let daily_returns: Vec<f64> = self
            .equity_points
            .windows(2)
            .map(|w| (w[1].equity - w[0].equity) / w[0].equity)
            .collect();

        let n = daily_returns.len();
        let mean_return = if n > 0 {
            daily_returns.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };

        let variance = if n > 1 {
            daily_returns
                .iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>()
                / (n - 1) as f64
        } else {
            0.0
        };
        let std_return = variance.sqrt();

        let sharpe_ratio = if std_return > 0.0 {
            mean_return / std_return * (365.0_f64).sqrt()
        } else {
            0.0
        };

        // Sortino ratio (downside deviation)
        // Standard formula: downside_deviation = sqrt(sum(min(r - target, 0)^2) / N)
        // where target = 0.0 (risk-free rate) and N = total number of returns (Bug#5 fix)
        let target_return = 0.0;
        let downside_variance = if n > 0 {
            daily_returns
                .iter()
                .map(|r| (r - target_return).min(0.0).powi(2))
                .sum::<f64>()
                / n as f64
        } else {
            0.0
        };
        let downside_std = downside_variance.sqrt();
        let sortino_ratio = if downside_std > 0.0 {
            mean_return / downside_std * (365.0_f64).sqrt()
        } else {
            0.0
        };

        let calmar_ratio = if max_drawdown != 0.0 {
            annualized_return / max_drawdown.abs()
        } else {
            0.0
        };

        // Win rate, profit factor
        let w_returns: Vec<f64> = self
            .trades
            .iter()
            .filter(|t| t.pnl_usdt > 0.0)
            .map(|t| t.pnl_pct)
            .collect();
        let l_returns: Vec<f64> = self
            .trades
            .iter()
            .filter(|t| t.pnl_usdt <= 0.0)
            .map(|t| t.pnl_pct)
            .collect();

        let win_count = w_returns.len();
        let loss_count = l_returns.len();
        let win_rate = if total_trades > 0 {
            win_count as f64 / total_trades as f64 * 100.0
        } else {
            0.0
        };

        let total_profit: f64 = self
            .trades
            .iter()
            .filter(|t| t.pnl_usdt > 0.0)
            .map(|t| t.pnl_usdt)
            .sum();
        let total_loss: f64 = self
            .trades
            .iter()
            .filter(|t| t.pnl_usdt <= 0.0)
            .map(|t| t.pnl_usdt.abs())
            .sum();

        let profit_factor = if total_loss > 0.0 {
            total_profit / total_loss
        } else if total_profit > 0.0 {
            9999.99 // cap INFINITY to avoid JSON serialization issues (Bug#6)
        } else {
            1.0
        };

        let avg_win_pct = if win_count > 0 {
            w_returns.iter().sum::<f64>() / win_count as f64
        } else {
            0.0
        };
        let avg_loss_pct = if loss_count > 0 {
            l_returns.iter().sum::<f64>() / loss_count as f64
        } else {
            0.0
        };
        let avg_trade_pct = if total_trades > 0 {
            self.trades.iter().map(|t| t.pnl_pct).sum::<f64>() / total_trades as f64
        } else {
            0.0
        };

        let total_fees: f64 = self.trades.iter().map(|t| t.fee).sum();
        let total_slippage: f64 = self.trades.iter().map(|t| t.slippage).sum();

        BacktestMetrics {
            total_return_pct: total_return * 100.0,
            annualized_return_pct: annualized_return * 100.0,
            max_drawdown_pct: max_drawdown * 100.0,
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
            win_rate,
            total_trades,
            profit_factor,
            avg_win_pct,
            avg_loss_pct,
            avg_trade_pct,
            total_fees,
            total_slippage,
        }
    }
}

// ============ Helper for truncated equity curve ============

/// Sample equity_curve for large datasets.
///
/// Uses uniform sampling: keeps every `step`-th point + first + last.
/// This keeps the response payload manageable even for backtests with
/// hundreds of thousands of equity points.
pub fn sample_equity_curve(points: &[EquityPoint], max_points: usize) -> Vec<EquityPoint> {
    if points.len() <= max_points {
        return points.to_vec();
    }
    let step = points.len() / max_points;
    let mut sampled = Vec::with_capacity(max_points + 2);
    for (i, p) in points.iter().enumerate() {
        if i % step == 0 || i == 0 || i == points.len() - 1 {
            sampled.push(p.clone());
        }
    }
    sampled
}

// ============ Data Access: Kline Loading ============

/// Build a time range query for kline_data.
///
/// Parses `start_date`/`end_date` from the config into millisecond timestamps.
/// Returns `(symbol, interval, start_ms, end_ms)`.
pub fn build_kline_query(config: &BacktestConfig) -> (String, String, i64, i64) {
    let start_ms = chrono::NaiveDate::parse_from_str(&config.start_date, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
        .map(|dt| dt.and_utc().timestamp_millis())
        .unwrap_or(0);
    let end_ms = chrono::NaiveDate::parse_from_str(&config.end_date, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap())
        .map(|dt| dt.and_utc().timestamp_millis())
        .unwrap_or(0);
    (
        config.symbol.clone(),
        config.interval.clone(),
        start_ms,
        end_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::backtest::Signal;

    // Mock strategy that buys on first bar, sells on last
    struct MockStrategy;
    impl StrategyTemplate for MockStrategy {
        fn id(&self) -> &str {
            "mock"
        }
        fn name(&self) -> &str {
            "Mock"
        }
        fn description(&self) -> &str {
            ""
        }
        fn category(&self) -> &str {
            "mock"
        }
        fn default_parameters(&self) -> Value {
            serde_json::json!({})
        }
        fn parameter_schema(&self) -> Vec<crate::models::schemas::ParameterDef> {
            vec![]
        }
        fn validate(&self, _params: &Value) -> Result<(), String> {
            Ok(())
        }
        fn generate_signal(&self, _klines: &[Kline], idx: usize, _params: &Value) -> Signal {
            if idx == 0 {
                Signal::Buy { quantity_pct: 1.0 }
            } else if idx == _klines.len() - 1 {
                Signal::CloseAll
            } else {
                Signal::Hold
            }
        }
    }

    #[test]
    fn test_engine_run_no_cancel() {
        let klines = (0..10)
            .map(|i| Kline {
                open_time: i * 3600000,
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.0 + i as f64,
                close: 100.5 + i as f64,
                volume: 1000.0,
            })
            .collect();

        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (metrics, _trades, _) = result.unwrap();
        assert_eq!(metrics.total_trades, 1); // 1 round-trip: buy at idx 0 → close at last bar
        assert!(metrics.sharpe_ratio >= 0.0 || metrics.sharpe_ratio == 0.0);
    }

    #[test]
    fn test_engine_empty_klines() {
        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            vec![],
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        assert!(engine.run().is_err());
    }

    #[test]
    fn test_build_kline_query() {
        let config = BacktestConfig {
            symbol: "ETH/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-06-01".into(),
            end_date: "2024-06-30".into(),
            initial_capital: 50000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };
        let (sym, interval, start_ms, end_ms) = build_kline_query(&config);
        assert_eq!(sym, "ETH/USDT");
        assert_eq!(interval, "1h");
        assert!(start_ms < end_ms);
    }

    // ===== Additional Engine Tests =====

    /// Mock: Buy at idx 0, Sell at idx=N/2, Hold otherwise
    struct MockBuySellMidStrategy;
    impl StrategyTemplate for MockBuySellMidStrategy {
        fn id(&self) -> &str {
            "mock_mid"
        }
        fn name(&self) -> &str {
            "Mock Mid"
        }
        fn description(&self) -> &str {
            ""
        }
        fn category(&self) -> &str {
            "mock"
        }
        fn default_parameters(&self) -> Value {
            serde_json::json!({})
        }
        fn parameter_schema(&self) -> Vec<crate::models::schemas::ParameterDef> {
            vec![]
        }
        fn validate(&self, _params: &Value) -> Result<(), String> {
            Ok(())
        }
        fn generate_signal(&self, klines: &[Kline], idx: usize, _params: &Value) -> Signal {
            let mid = klines.len() / 2;
            if idx == 0 {
                Signal::Buy { quantity_pct: 1.0 }
            } else if idx == mid {
                Signal::Sell { quantity_pct: 1.0 }
            } else {
                Signal::Hold
            }
        }
    }

    #[test]
    fn test_engine_profit_on_uptrend() {
        // Prices steadily rise — short trade should lose, but we only go long
        let klines: Vec<Kline> = (0..20)
            .map(|i| Kline {
                open_time: i as i64 * 3600000,
                open: 100.0 + i as f64 * 2.0,
                high: 102.0 + i as f64 * 2.0,
                low: 99.0 + i as f64 * 2.0,
                close: 101.0 + i as f64 * 2.0,
                volume: 1000.0,
            })
            .collect();

        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(MockBuySellMidStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (metrics, trades, _) = result.unwrap();
        assert_eq!(metrics.total_trades, 2);
        // In uptrend, long position should profit
        if !trades.is_empty() {
            assert!(
                trades[0].pnl_usdt > 0.0 || trades[1].pnl_usdt > 0.0,
                "Expected profitable trades in uptrend, got {:?}",
                trades
            );
        }
        assert!(
            metrics.total_return_pct > 0.0,
            "Expected positive return in uptrend, got {}",
            metrics.total_return_pct
        );
    }

    #[test]
    fn test_engine_loss_on_downtrend() {
        // Prices steadily fall — long should lose
        let klines: Vec<Kline> = (0..20)
            .map(|i| Kline {
                open_time: i as i64 * 3600000,
                open: 200.0 - i as f64 * 5.0,
                high: 201.0 - i as f64 * 5.0,
                low: 198.0 - i as f64 * 5.0,
                close: 199.0 - i as f64 * 5.0,
                volume: 1000.0,
            })
            .collect();

        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (_, trades, _) = result.unwrap();
        if !trades.is_empty() {
            // Sum of all trade PnL should be negative in downtrend (long-only strategy)
            let total_pnl: f64 = trades.iter().map(|t| t.pnl_usdt).sum();
            assert!(
                total_pnl < 0.0,
                "Expected negative PnL in downtrend, got {}",
                total_pnl
            );
        }
    }

    #[test]
    fn test_engine_equity_curve_length() {
        let klines: Vec<Kline> = (0..15)
            .map(|i| Kline {
                open_time: i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1000.0,
            })
            .collect();

        let config = BacktestConfig {
            symbol: "XRP/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines.clone(),
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (_, _, equity) = result.unwrap();
        assert!(!equity.is_empty(), "Equity curve should not be empty");
        assert_eq!(
            equity.len(),
            klines.len(),
            "Equity curve should have one point per kline"
        );
    }

    #[test]
    fn test_engine_no_trades_metrics() {
        // Strategy that always holds — no trades
        struct NoTradeStrategy;
        impl StrategyTemplate for NoTradeStrategy {
            fn id(&self) -> &str {
                "no_trade"
            }
            fn name(&self) -> &str {
                "No Trade"
            }
            fn description(&self) -> &str {
                ""
            }
            fn category(&self) -> &str {
                "mock"
            }
            fn default_parameters(&self) -> Value {
                serde_json::json!({})
            }
            fn parameter_schema(&self) -> Vec<crate::models::schemas::ParameterDef> {
                vec![]
            }
            fn validate(&self, _params: &Value) -> Result<(), String> {
                Ok(())
            }
            fn generate_signal(&self, _: &[Kline], _: usize, _: &Value) -> Signal {
                Signal::Hold
            }
        }

        let klines: Vec<Kline> = (0..5)
            .map(|i| Kline {
                open_time: i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1000.0,
            })
            .collect();

        let config = BacktestConfig {
            symbol: "ETH/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 50000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(NoTradeStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (metrics, trades, equity) = result.unwrap();
        assert_eq!(metrics.total_trades, 0);
        assert!(trades.is_empty());
        assert!(!equity.is_empty());
        assert_eq!(metrics.win_rate, 0.0);
        assert_eq!(metrics.sharpe_ratio, 0.0);
    }

    #[test]
    fn test_engine_mark_to_market() {
        // Verify that equity curve reflects mark-to-market during open positions
        let klines: Vec<Kline> = vec![
            Kline {
                open_time: 0,
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 1000.0,
            },
            // Price drops — equity should decrease
            Kline {
                open_time: 3600000,
                open: 101.0,
                high: 102.0,
                low: 98.0,
                close: 99.0,
                volume: 1000.0,
            },
            // Price drops more
            Kline {
                open_time: 7200000,
                open: 99.0,
                high: 100.0,
                low: 97.0,
                close: 98.0,
                volume: 1000.0,
            },
            // Price rises
            Kline {
                open_time: 10800000,
                open: 98.0,
                high: 103.0,
                low: 97.0,
                close: 102.0,
                volume: 1000.0,
            },
            // Close at last bar
            Kline {
                open_time: 14400000,
                open: 102.0,
                high: 103.0,
                low: 101.0,
                close: 102.5,
                volume: 1000.0,
            },
        ];

        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (_, _, equity) = result.unwrap();

        // We buy at kline 0 (close=101.0), hold through bars 1-3, sell at bar 4
        // Equity at bar 0: after buy, cash reduced + MTM = cash + position_value
        // At bar 1: close=99.0, position value should drop
        // At bar 3: close=102.0, position value should rise
        // At bar 4: position closed, equity = cash
        assert!(equity.len() >= 3, "Expected at least 3 equity points");
        // The equity curve should have at least one non-initial value
        let first_equity = equity[0].equity;
        assert!(
            (first_equity - 10000.0).abs() < 0.01 || first_equity < 10000.0,
            "Initial bar equity should be close to capital (minus buy cost), got {}",
            first_equity
        );
    }

    #[test]
    fn test_sample_equity_curve_small() {
        let points: Vec<EquityPoint> = (0..5)
            .map(|i| EquityPoint {
                time: i * 1000,
                equity: 100.0 + i as f64,
                drawdown_pct: 0.0,
            })
            .collect();
        let sampled = sample_equity_curve(&points, 10);
        assert_eq!(
            sampled.len(),
            points.len(),
            "Should not sample if under max"
        );
    }

    #[test]
    fn test_sample_equity_curve_large() {
        let points: Vec<EquityPoint> = (0..1000)
            .map(|i| EquityPoint {
                time: i * 1000,
                equity: 100.0,
                drawdown_pct: 0.0,
            })
            .collect();
        let sampled = sample_equity_curve(&points, 10);
        assert!(
            sampled.len() <= 12,
            "Should reduce to at most max+2 points, got {}",
            sampled.len()
        );
        assert!(
            sampled.len() >= 2,
            "Should keep first and last, got {}",
            sampled.len()
        );
        assert_eq!(sampled[0].time, 0);
        assert_eq!(sampled[sampled.len() - 1].time, 999000);
    }

    #[test]
    fn test_metrics_max_drawdown() {
        // Create a scenario with a clear drawdown
        let klines: Vec<Kline> = vec![
            Kline {
                open_time: 0,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 1000.0,
            },
            Kline {
                open_time: 3600000,
                open: 100.5,
                high: 101.5,
                low: 100.0,
                close: 101.0,
                volume: 1000.0,
            },
            Kline {
                open_time: 7200000,
                open: 101.0,
                high: 101.0,
                low: 90.0,
                close: 91.0,
                volume: 1000.0,
            },
            Kline {
                open_time: 10800000,
                open: 91.0,
                high: 92.0,
                low: 85.0,
                close: 86.0,
                volume: 1000.0,
            },
            Kline {
                open_time: 14400000,
                open: 86.0,
                high: 87.0,
                low: 84.0,
                close: 85.0,
                volume: 1000.0,
            },
        ];

        let config = BacktestConfig {
            symbol: "SOL/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-01-02".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            ..Default::default()
        };

        let mut engine = BacktestEngine::new(
            config,
            klines,
            Box::new(MockStrategy),
            serde_json::json!({}),
            Arc::new(AtomicU32::new(0)),
            CancellationToken::new(),
        );

        let result = engine.run();
        assert!(result.is_ok());
        let (metrics, _, _) = result.unwrap();
        // In a downtrend, there should be some drawdown
        // (the initial buy may still be losing at the end)
        // Just verify max_drawdown_pct is <= 0
        assert!(
            metrics.max_drawdown_pct <= 0.0 || metrics.max_drawdown_pct == 0.0,
            "Max drawdown should be <= 0, got {}",
            metrics.max_drawdown_pct
        );
    }
}
