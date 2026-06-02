//! models/trailing_stop_params.rs — 跟踪止损订单参数模型 (Trailing Stop Order Params, P1-2.3)
//! models/trailing_stop_params.rs — Trailing stop parameter schema (P1-2.3)
//!
//! 中文说明：
//!   本文件是 P1-2.3 跟踪止损订单的参数模型，作为 `orders.advanced_params` JSONB
//!   列的类型安全包装（type-safe wrapper）。跟踪止损 = 入场限价单 + 动态止损线，
//!   止损线只在价格朝"有利方向"移动时跟随调整，永不后退：
//!     - 做多 (Buy):  止损跟随价格上涨上移（peak_price 单调递增，SL = peak × (1 − distance)）
//!     - 做空 (Sell): 止损跟随价格下跌下移（peak_price 单调递减，SL = peak × (1 + distance)）
//!
//!   集成点（integration points）：
//!     1. `handlers/order.rs::parse_order_type` — 解析 `order_type=trailing_stop` 字符串
//!     2. `services/trailing_stop.rs::tick_one` — 轮询时读取本结构、更新 peak_price、判定触发
//!     3. `services/trailing_stop.rs::poll_once` — 2s 周期按 symbol 分组调价后批量 `tick_one`
//!     4. `db/order.rs::OrderType::TrailingStop` — SeaORM 枚举对应 `"trailing_stop"` 字符串
//!
//! English description:
//!   Type-safe wrapper around the `advanced_params` JSONB blob for trailing stop orders.
//!
//!   Trailing stop = entry limit order + dynamic stop-loss that follows price movement
//!   in the favourable direction only:
//!   - **Buy**:  SL follows price UP   (peak_price only increases, SL = peak × (1 − distance))
//!   - **Sell**: SL follows price DOWN (peak_price only decreases, SL = peak × (1 + distance))
//!
//!   Used by:
//!     1. `handlers/order.rs::parse_order_type` — parses `order_type=trailing_stop`
//!     2. `services/trailing_stop.rs::tick_one`  — reads/updates this struct, detects trigger
//!     3. `services/trailing_stop.rs::poll_once` — 2s polling loop that batches by symbol
//!     4. `db/order.rs::OrderType::TrailingStop` — SeaORM enum mapping to `"trailing_stop"`

use serde::{Deserialize, Serialize};

// 单元测试：文件末尾的 `#[cfg(test)] mod tests` 内含 16 个测试，覆盖
//   - validate_new 全部 7 个分支（buy/sell 合法、零价/负价/零距离/边界 0.1/超大/非法 side）
//   - update_peak 单调跟随（buy 升不降、sell 降不升、完整周期）
//   - trigger_price 公式（buy = peak×0.99，sell = peak×1.005）
//   - is_triggered 边界（高于/等于/低于 SL 时的触发判定）
//
// Unit tests: 16 tests at the bottom of this file covering:
//   - all 7 validate_new branches (valid buy/sell, zero/negative price, zero distance,
//     boundary 0.1, oversized distance, invalid side)
//   - update_peak monotonicity (buy: up-only, sell: down-only, full cycle)
//   - trigger_price formula (buy = peak×0.99, sell = peak×1.005)
//   - is_triggered edge cases (above/at/below SL)

/// 跟踪止损订单参数 — 持久化到 `orders.advanced_params` JSONB 列。
/// Trailing stop order parameters — persisted to `orders.advanced_params` JSONB column.
///
/// 中文：该结构同时承担"配置"与"运行时状态"两个角色：
///   - 配置：创建时由 handler 序列化写入（含 `trailing_distance`、`side`）
///   - 状态：`peak_price` 在轮询循环中被持续更新（详见 `services/trailing_stop.rs`）
///
/// English: This struct plays two roles — static configuration + mutable runtime state:
///   - Configuration: serialized at creation time by the handler (contains `trailing_distance`, `side`)
///   - State: `peak_price` is mutated by the polling loop (see `services/trailing_stop.rs`)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrailingStopParams {
    /// 当前追踪到的极值价格（peak / trough price）。
    /// Initial peak price (== entry price at creation time).
    ///
    ///    中文：
    ///   - 初始值 = 入场价（创建订单时由 `validate_new` 写入）
    ///   - Buy 母单：单调递增（每次 `update_peak` 只在 `current_price > peak` 时更新）
    ///   - Sell 母单：单调递减（每次 `update_peak` 只在 `current_price < peak` 时更新）
    ///   - 持久化：每次更新后重新序列化写回 `advanced_params.peak_price`
    ///
    /// English:
    ///   - Initial value = entry price (set by `validate_new` at creation time)
    ///   - Buy:  monotonically increases (`update_peak` only writes when `current_price > peak`)
    ///   - Sell: monotonically decreases (`update_peak` only writes when `current_price < peak`)
    ///   - Persistence: re-serialized to `advanced_params.peak_price` after every update
    pub peak_price: f64,

    /// 跟踪距离，以 peak 价格的分数表示（0.01 = 1%）。
    /// Distance as a fraction of peak price (0.01 = 1%).
    ///
    ///    中文：
    ///   - 合法范围：(0.0, 0.1) — 由 `validate_new` 强制校验，下界 0 避免零距离无意义，
    ///     上界 10% 防止用户把止损设得过宽
    ///   - 触发价公式：
    ///       * Buy  →  trigger = peak × (1 − distance)
    ///       * Sell →  trigger = peak × (1 + distance)
    ///   - 创建后不可变（immutable after creation）
    ///
    /// English:
    ///   - Validated range: (0.0, 0.1) — see `validate_new`. Lower bound 0 avoids zero-distance
    ///     no-op, upper bound 10% prevents overly-wide stops.
    ///   - Trigger formula:
    ///       * Buy  →  trigger = peak × (1 − distance)
    ///       * Sell →  trigger = peak × (1 + distance)
    ///   - Immutable after creation
    pub trailing_distance: f64,

    /// 订单方向：`"buy"` 或 `"sell"`，与母单 `orders.side` 镜像冗余存储便于解析。
    /// Order side: `"buy"` or `"sell"` — mirrors the parent order side for parsing clarity.
    ///
    ///    中文：
    ///   - 冗余存储的原因：避免在 service 层反序列化后再去关联查询母单 side，
    ///     减少 JOIN，提升轮询吞吐
    ///   - 取值集合：`"buy" | "sell"`（其他值在 `update_peak`/`trigger_price` 中被静默忽略）
    ///
    /// English:
    ///   - Stored redundantly to avoid joining the parent order in the hot polling path.
    ///   - Allowed values: `"buy" | "sell"` (any other value is silently ignored by
    ///     `update_peak` / `trigger_price`).
    pub side: String,
}

impl TrailingStopParams {
    /// 构造并校验新的 `TrailingStopParams`。Validate input + construct.
    ///
    ///    中文：
    ///   与 `bracket_params::validate_new` 保持一致的工厂函数风格：
    ///   - 校验失败返回 `Err(String)`，错误信息由 caller 透传给客户端
    ///   - 校验通过返回已初始化的 `Self`，`peak_price` 默认等于入场价
    ///
    ///   校验规则（Rules）：
    ///   1. `entry_price` > 0 — 防御性检查（handler 层通常已校验一次）
    ///   2. `trailing_distance` ∈ (0.0, 0.1) — 严格开区间（0 与 0.1 都被拒绝）
    ///   3. `side` ∈ {"buy", "sell"} — 严格枚举
    ///
    /// English: Factory function mirroring the style of `bracket_params::validate_new`.
    ///   - Returns `Err(String)` on validation failure; the caller forwards the message to the client.
    ///   - Returns an initialized `Self` on success; `peak_price` is set to `entry_price` by default.
    ///
    ///   Rules:
    ///   1. `entry_price` > 0 — defensive check (handler also validates)
    ///   2. `trailing_distance` ∈ (0.0, 0.1) — strict open interval (0 and 0.1 both rejected)
    ///   3. `side` ∈ {"buy", "sell"} — strict enum
    pub fn validate_new(
        entry_price: f64,
        trailing_distance: f64,
        side: &str,
    ) -> Result<Self, String> {
        // 规则 1：入场价必须为正。Rule 1: entry price must be positive.
        if entry_price <= 0.0 {
            return Err(format!("entry_price must be > 0, got {}", entry_price));
        }
        // 规则 2：跟踪距离 ∈ (0, 0.1) — 严格开区间。
        // Rule 2: trailing distance in (0, 0.1) — strict open interval.
        //   - `<= 0.0` 拒绝：距离为 0 等同于无止损（立即触发或永不触发，行为未定义）
        //   - `>= 0.1` 拒绝：超过 10% 的止损在大多数交易场景下没有实际意义
        if trailing_distance <= 0.0 || trailing_distance >= 0.1 {
            return Err(format!(
                "trailing_distance must be in (0.0, 0.1), got {}",
                trailing_distance
            ));
        }
        // 规则 3：方向枚举校验。Rule 3: side enum check.
        if side != "buy" && side != "sell" {
            return Err(format!("side must be 'buy' or 'sell', got '{}'", side));
        }
        // 构造成功：用入场价初始化 peak_price。
        // Construct: initialize peak_price with entry price.
        Ok(Self {
            peak_price: entry_price,
            trailing_distance,
            side: side.to_string(),
        })
    }

    /// 原地更新 `peak_price`（仅在价格朝有利方向移动时）。
    /// Update `peak_price` in-place if `current_price` moves in the favourable direction.
    ///
    ///    中文：
    ///   这是跟踪止损算法的核心 — "peak" 的单调性保证了止损线只朝有利方向移动：
    ///     - Buy  → 仅当 `current_price > peak_price` 时更新（抬升 SL）
    ///     - Sell → 仅当 `current_price < peak_price` 时更新（压低 SL）
    ///     - 其他情况保持不变（这是"trailing"与"普通 stop"的根本区别）
    ///
    ///   静默忽略非法 side（兜底，正常数据流中不会触发）。
    ///
    /// English: This is the heart of the trailing-stop algorithm — `peak_price` is
    ///   monotonically non-decreasing for buy, non-increasing for sell, which is
    ///   what makes the stop "trail" rather than "snap back":
    ///     - Buy  → only update when `current_price > peak_price` (raise SL)
    ///     - Sell → only update when `current_price < peak_price` (lower SL)
    ///     - Otherwise unchanged (this is the core difference vs. a plain stop)
    ///
    ///   Silently no-ops on invalid side (defensive; normal data flow never hits this).
    pub fn update_peak(&mut self, current_price: f64) {
        match self.side.as_str() {
            // 做多：价格创新高时抬升 peak（止损线跟随上移）。
            // Buy: when price makes a new high, raise peak (SL trails upward).
            // 使用 match 守卫（match guard）替代嵌套 if，以满足 clippy::collapsible_match。
            // Use a match guard instead of a nested `if` to satisfy clippy::collapsible_match.
            "buy" if current_price > self.peak_price => {
                self.peak_price = current_price;
            }
            // 做空：价格创新低时压低 peak（止损线跟随下移）。
            // Sell: when price makes a new low, lower peak (SL trails downward).
            "sell" if current_price < self.peak_price => {
                self.peak_price = current_price;
            }
            // 非法 side 或价格未朝有利方向移动 — 保持 peak 不变
            // （防御性兜底 + buy/sell 的"反向"或"持平"路径）。
            // Invalid side OR price did not move favourably — leave peak unchanged
            // (defensive fallback + the "reversed" / "flat" path of buy/sell).
            _ => {}
        }
    }

    /// 计算当前止损触发价（stop-loss trigger price）。
    /// Compute the current stop-loss trigger price.
    ///
    ///    中文：
    ///   - Buy  → `peak_price × (1 − trailing_distance)`
    ///     例：peak=51000, distance=0.01 → trigger=50490
    ///   - Sell → `peak_price × (1 + trailing_distance)`
    ///     例：peak=49000, distance=0.005 → trigger=49245
    ///   - 非法 side 返回 `peak_price` 本身（无意义的兜底值，调用方应已校验）
    ///
    ///   该方法为纯函数（pure function），便于在测试中直接断言。
    ///
    /// English: Pure function — safe to call repeatedly and to assert on in tests.
    ///   - Buy  → `peak_price × (1 − trailing_distance)`
    ///     e.g. peak=51000, distance=0.01 → trigger=50490
    ///   - Sell → `peak_price × (1 + trailing_distance)`
    ///     e.g. peak=49000, distance=0.005 → trigger=49245
    ///   - Invalid side returns `peak_price` itself (defensive; caller should have validated).
    pub fn trigger_price(&self) -> f64 {
        match self.side.as_str() {
            // 做多止损线在 peak 下方。
            // Buy stop sits below the peak.
            "buy" => self.peak_price * (1.0 - self.trailing_distance),
            // 做空止损线在 peak 上方。
            // Sell stop sits above the peak.
            "sell" => self.peak_price * (1.0 + self.trailing_distance),
            // 非法 side — 返回 peak 本身（理论无效；兜底）。
            // Invalid side — return peak (theoretical no-op; fallback).
            _ => self.peak_price,
        }
    }

    /// 判断当前价是否会触发止损。
    /// Check whether `current_price` would trigger the stop.
    ///
    ///    中文：
    ///   - Buy  → `current_price <= trigger_price()` 时触发（价格下穿 SL）
    ///   - Sell → `current_price >= trigger_price()` 时触发（价格上穿 SL）
    ///   - 包含等号：价格"恰好"等于 SL 视为已触发（避免边界抖动）
    ///   - 非法 side 永远不触发（防御性）
    ///
    /// English:
    ///   - Buy  → triggers when `current_price <= trigger_price()` (price crosses below SL)
    ///   - Sell → triggers when `current_price >= trigger_price()` (price crosses above SL)
    ///   - Equality included: exact-match counts as triggered (avoids boundary jitter)
    ///   - Invalid side never triggers (defensive)
    pub fn is_triggered(&self, current_price: f64) -> bool {
        match self.side.as_str() {
            // 做多：价格 ≤ SL 触发（跌破止损线）。
            // Buy: triggers when price ≤ SL (down through the stop line).
            "buy" => current_price <= self.trigger_price(),
            // 做空：价格 ≥ SL 触发（涨破止损线）。
            // Sell: triggers when price ≥ SL (up through the stop line).
            "sell" => current_price >= self.trigger_price(),
            // 非法 side — 永不应触发。
            // Invalid side — should never trigger.
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────
    // validate_new — 合法路径（happy path）
    // validate_new — happy paths
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：验证 buy 母单构造后三个字段的初始值。
    /// English: verify the three fields are correctly initialized for a buy parent.
    #[test]
    fn validate_new_buy_ok() {
        let p = TrailingStopParams::validate_new(50000.0, 0.01, "buy").unwrap();
        // 入场价 50000 → peak 初始值也应为 50000
        // entry price 50000 → initial peak should also be 50000
        assert_eq!(p.peak_price, 50000.0);
        assert_eq!(p.trailing_distance, 0.01);
        assert_eq!(p.side, "buy");
    }

    /// 中文：验证 sell 母单构造（distance 0.5% 也合法）。
    /// English: verify sell parent construction (distance 0.5% is also valid).
    #[test]
    fn validate_new_sell_ok() {
        let p = TrailingStopParams::validate_new(50000.0, 0.005, "sell").unwrap();
        assert_eq!(p.peak_price, 50000.0);
        assert_eq!(p.trailing_distance, 0.005);
    }

    // ─────────────────────────────────────────────────────────────────────
    // validate_new — 入场价校验
    // validate_new — entry price validation
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：入场价 = 0 应当被拒绝。
    /// English: entry price = 0 must be rejected.
    #[test]
    fn validate_new_rejects_zero_price() {
        let err = TrailingStopParams::validate_new(0.0, 0.01, "buy").unwrap_err();
        assert!(err.contains("entry_price must be > 0"));
    }

    /// 中文：入场价为负应当被拒绝。
    /// English: negative entry price must be rejected.
    #[test]
    fn validate_new_rejects_negative_price() {
        let err = TrailingStopParams::validate_new(-100.0, 0.01, "buy").unwrap_err();
        assert!(err.contains("entry_price must be > 0"));
    }

    // ─────────────────────────────────────────────────────────────────────
    // validate_new — 距离校验（边界 + 非法值）
    // validate_new — distance validation (boundary + invalid)
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：distance = 0 应当被拒绝（开区间下界）。
    /// English: distance = 0 must be rejected (open-interval lower bound).
    #[test]
    fn validate_new_rejects_zero_distance() {
        let err = TrailingStopParams::validate_new(50000.0, 0.0, "buy").unwrap_err();
        assert!(err.contains("trailing_distance must be in"));
    }

    /// 中文：distance = 0.1 应当被拒绝（开区间上界 10%）。
    /// English: distance = 0.1 must be rejected (open-interval upper bound 10%).
    #[test]
    fn validate_new_rejects_ten_percent_distance() {
        let err = TrailingStopParams::validate_new(50000.0, 0.1, "buy").unwrap_err();
        assert!(err.contains("trailing_distance must be in"));
    }

    /// 中文：distance = 0.5（远超 10%）应当被拒绝。
    /// English: distance = 0.5 (well above 10%) must be rejected.
    #[test]
    fn validate_new_rejects_over_ten_percent() {
        let err = TrailingStopParams::validate_new(50000.0, 0.5, "buy").unwrap_err();
        assert!(err.contains("trailing_distance must be in"));
    }

    // ─────────────────────────────────────────────────────────────────────
    // validate_new — 方向校验
    // validate_new — side validation
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：非法 side 字符串（"hold"）应当被拒绝。
    /// English: invalid side string ("hold") must be rejected.
    #[test]
    fn validate_new_rejects_invalid_side() {
        let err = TrailingStopParams::validate_new(50000.0, 0.01, "hold").unwrap_err();
        assert!(err.contains("side must be"));
    }

    // ─────────────────────────────────────────────────────────────────────
    // update_peak / trigger_price 联动 — peak 单调跟随逻辑
    // update_peak / trigger_price — peak monotonicity
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：Buy 母单在价格上涨时应抬升 peak，并重算 trigger 价。
    /// English: buy parent must raise peak on price upticks and recompute trigger.
    #[test]
    fn buy_peak_raises_on_price_increase() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.01, "buy").unwrap();
        // 价格从 50000 → 51000：peak 抬升到 51000
        // price 50000 → 51000: peak raises to 51000
        p.update_peak(51000.0);
        assert_eq!(p.peak_price, 51000.0);
        // 同步验证 trigger_price 公式：51000 × (1 − 0.01) = 50490
        // verify trigger_price formula: 51000 × (1 − 0.01) = 50490
        assert!((p.trigger_price() - 50490.0).abs() < 0.01);
    }

    /// 中文：Buy 母单在价格下跌时 peak 必须保持不变（核心不变量）。
    /// English: buy parent peak must stay unchanged on price drops (core invariant).
    #[test]
    fn buy_peak_does_not_lower_on_price_drop() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.01, "buy").unwrap();
        p.update_peak(49000.0);
        // 关键断言：peak 仍为 50000（未跟随下跌）
        // key assertion: peak remains 50000 (did NOT trail down)
        assert_eq!(p.peak_price, 50000.0);
    }

    /// 中文：Sell 母单在价格下跌时应压低 peak，并重算 trigger 价。
    /// English: sell parent must lower peak on price drops and recompute trigger.
    #[test]
    fn sell_peak_lowers_on_price_drop() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.005, "sell").unwrap();
        // 价格从 50000 → 49000：peak 压低到 49000
        // price 50000 → 49000: peak lowers to 49000
        p.update_peak(49000.0);
        assert_eq!(p.peak_price, 49000.0);
        // 同步验证 trigger_price 公式：49000 × (1 + 0.005) = 49245
        // verify trigger_price formula: 49000 × (1 + 0.005) = 49245
        assert!((p.trigger_price() - 49245.0).abs() < 0.01);
    }

    /// 中文：Sell 母单在价格上涨时 peak 必须保持不变（核心不变量）。
    /// English: sell parent peak must stay unchanged on price upticks (core invariant).
    #[test]
    fn sell_peak_does_not_raise_on_price_increase() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.005, "sell").unwrap();
        p.update_peak(51000.0);
        // 关键断言：peak 仍为 50000（未跟随上涨）
        // key assertion: peak remains 50000 (did NOT trail up)
        assert_eq!(p.peak_price, 50000.0);
    }

    // ─────────────────────────────────────────────────────────────────────
    // is_triggered — 触发判定的边界（上 / 等 / 下）
    // is_triggered — trigger boundary (above / at / below)
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：Buy 母单在 peak=51000、distance=0.01 时，trigger=50490。
    ///   - 50500 > 50490：不触发
    ///   - 50490 == 50490：触发（等号包含）
    ///   - 50400 < 50490：触发
    ///
    /// English: buy parent with peak=51000, distance=0.01 ⇒ trigger=50490.
    ///   - 50500 > 50490: not triggered
    ///   - 50490 == 50490: triggered (equality included)
    ///   - 50400 < 50490: triggered
    #[test]
    fn buy_trigger_at_or_below_sl() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.01, "buy").unwrap();
        p.update_peak(51000.0);
        // trigger = 51000 × 0.99 = 50490
        assert!(!p.is_triggered(50500.0)); // 在 SL 上方 / above
        assert!(p.is_triggered(50490.0));  // 恰好等于 SL / exactly at SL
        assert!(p.is_triggered(50400.0));  // 跌破 SL / below SL
    }

    /// 中文：Sell 母单在 peak=49000、distance=0.005 时，trigger=49245。
    ///   - 49200 < 49245：不触发
    ///   - 49245 == 49245：触发（等号包含）
    ///   - 49300 > 49245：触发
    ///
    /// English: sell parent with peak=49000, distance=0.005 ⇒ trigger=49245.
    ///   - 49200 < 49245: not triggered
    ///   - 49245 == 49245: triggered (equality included)
    ///   - 49300 > 49245: triggered
    #[test]
    fn sell_trigger_at_or_above_sl() {
        let mut p = TrailingStopParams::validate_new(50000.0, 0.005, "sell").unwrap();
        p.update_peak(49000.0);
        // trigger = 49000 × 1.005 = 49245
        assert!(!p.is_triggered(49200.0)); // 在 SL 下方 / below
        assert!(p.is_triggered(49245.0));  // 恰好等于 SL / exactly at SL
        assert!(p.is_triggered(49300.0));  // 涨破 SL / above
    }

    // ─────────────────────────────────────────────────────────────────────
    // 完整价格周期测试 — 验证 peak 的"水涨船高"行为
    // Full price cycle test — verifies "ratchet" behaviour of peak
    // ─────────────────────────────────────────────────────────────────────

    /// 中文：模拟价格序列 50000 → 51000 → 50500 → 51500 → 51000，
    ///   验证 buy 母单的 peak 永远取历史最高点（水涨船高，永不回落）。
    /// English: simulates price series 50000 → 51000 → 50500 → 51500 → 51000 and
    ///   verifies buy parent peak always tracks the historical maximum (ratchet only).
    #[test]
    fn buy_peak_monotonic_through_full_cycle() {
        // 价格序列：50000 → 51000 → 50500 → 51500 → 51000
        // 期望 peak：50000 → 51000 → 51000 → 51500 → 51500
        // Price series:    50000 → 51000 → 50500 → 51500 → 51000
        // Expected peak:   50000 → 51000 → 51000 → 51500 → 51500
        let mut p = TrailingStopParams::validate_new(50000.0, 0.01, "buy").unwrap();
        p.update_peak(51000.0);
        assert_eq!(p.peak_price, 51000.0);            // 创新高 → 上移 / new high → raise
        p.update_peak(50500.0);
        assert_eq!(p.peak_price, 51000.0);            // 回落 → 不变 / pullback → unchanged
        p.update_peak(51500.0);
        assert_eq!(p.peak_price, 51500.0);            // 再次创新高 → 上移 / new high → raise
        p.update_peak(51000.0);
        assert_eq!(p.peak_price, 51500.0);            // 再次回落 → 不变 / pullback → unchanged
    }
}
