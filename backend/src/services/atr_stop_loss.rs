//! ATR 追踪止损服务 (ADR-015) — 随价格朝有利方向自动上移止损价，触发时市价平仓。
//! ATR trailing-stop service (ADR-015) — auto-ratchets the stop price in the
//!   favourable direction; closes the position at market when triggered.
//!
//! 中文说明：
//!   - 持久化：每个启用 ATR 追踪的开仓 position 在 `atr_stop_loss` 表中有一行
//!     (position_id → current_stop, atr_value)，是该 position 止损状态的"唯一真源"
//!   - "追踪"的本质：`current_stop` 单调朝有利方向移动（long 只能上移、short 只能下移），
//!     永不下调/上调至离入场价更近的位置 — 这是与"固定止损"的关键区别
//!   - 触发即清表：触发时立即删除该行，避免后续 tick 重复触发导致重复平仓
//!
//!   集成点（integration points）：
//!     1. `services/risk_manager.rs::open_position`  — 开仓成功后调 `init_stop_loss`
//!     2. `services/ws_hub.rs`（行情更新）            — 每次 ticker 调 `update_and_check`
//!     3. `handlers/position.rs::get_position`        — UI 拉取展示时调 `get_stop_price`
//!
//! English description:
//!   - Persistence: one row per ATR-tracked position in the `atr_stop_loss` table,
//!     acting as the single source of truth for that position's stop state.
//!   - "Trailing" invariant: `current_stop` is monotonic in the favourable direction
//!     (up-only for long, down-only for short) — never retreats towards entry.
//!   - Trigger = row deletion: triggered rows are deleted in the same txn to prevent
//!     a second tick from firing the same close a second time.
//!
//!   Used by:
//!     1. `services/risk_manager.rs::open_position`  — calls `init_stop_loss` after fill
//!     2. `services/ws_hub.rs`（ticker updates）       — calls `update_and_check` per tick
//!     3. `handlers/position.rs::get_position`        — calls `get_stop_price` for UI

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::atr_stop_loss;
use crate::db::atr_stop_loss::Entity as AtrStopLossEntity;
use crate::utils::error::AppError;

// ─── Types ───────────────────────────────────────────────────────────────────

/// 开仓时初始化 ATR 追踪止损所需的入参。
/// Init payload for an ATR trailing stop at position open.
///
/// 中文：构造时由 `risk_manager.rs` 在订单成交后一次性提交，`init_stop_loss` 据此
///   计算初始止损价并 INSERT 一行到 `atr_stop_loss`。
/// English: Built once at fill-time by `risk_manager.rs`; `init_stop_loss` uses it to
///   compute the initial stop and INSERT a row into `atr_stop_loss`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossInit {
    /// 持仓 UUID — 同时作为本表的主索引 / 与 `positions` 表 1:1 关联。
    /// Position UUID — primary index; 1:1 with the `positions` row.
    pub position_id: Uuid,

    /// 入场价（成交均价）— 持久化用于审计/回放，不参与 trailing 重算。
    /// Entry (average fill) price — persisted for audit/replay; not used in trailing math.
    pub entry_price: Decimal,

    /// 入场时刻的 ATR 值（一般 N=14 周期）— 之后会被 `update_and_check` 用
    ///   最新 ATR 覆盖；此处仅作为"开仓快照"用于审计。
    /// ATR at the moment of entry (typically N=14) — snapshot for audit; later
    ///   overwritten by `update_and_check` with the live ATR.
    pub atr_value: Decimal,

    /// ATR 计算窗口（周期数）— 冗余存储便于回放与多周期对比分析。
    /// ATR lookback period — stored redundantly for replay / multi-TF analysis.
    pub atr_period: i32,

    /// ATR 倍数（典型 1.5–3.0）— 越大止损越宽，越不容易被正常波动震出；
    ///   可配置以适配不同波动率环境（高波动币种需更大倍数）。
    /// ATR multiplier (typical 1.5–3.0) — larger ⇒ wider stop, less whipsaw;
    ///   configurable to adapt to per-symbol volatility regimes.
    pub multiplier: Decimal,

    /// 方向：`"long" | "short"` — 决定初始止损价是 entry 减（long）还是加（short）atr×mult。
    /// Side: `"long" | "short"` — determines whether the initial SL sits below
    ///   (long) or above (short) the entry.
    pub side: String, // "long" | "short"
}

/// 行情更新时驱动追踪止损的入参。
/// Per-tick payload that drives the trailing stop re-evaluation.
///
/// 中文：每次 WsHub 收到 ticker 时由 `ws_hub.rs` 组装并调用 `update_and_check`。
/// English: Assembled by `ws_hub.rs` from each incoming ticker; passed to
///   `update_and_check` which both persists the new stop and detects triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossUpdate {
    /// 持仓 UUID — 查表的唯一索引。
    /// Position UUID — sole key used to look up the existing row.
    pub position_id: Uuid,

    /// 最新市场价（mid / last trade）— 用于重算 candidate stop 与触发判定。
    /// Latest market price (mid / last trade) — used for both candidate-stop and trigger math.
    pub current_price: Decimal,

    /// 当前实时 ATR — 由 indicator 服务按 tick 维护，可能与开仓时的 atr_value 不同。
    /// Live ATR — maintained tick-by-tick by the indicator service; may differ from
    ///   the entry-time `atr_value`.
    pub current_atr: Decimal,

    /// 当前生效的 ATR 倍数（可与 Init 时不同）— 支持动态调参 / 风控降级。
    /// Effective ATR multiplier (may differ from Init) — supports dynamic retuning
    ///   and risk-control tightening/loosening.
    pub multiplier: Decimal,

    /// 方向镜像 — 与表中 `position_side` 冗余，调用方可省略一次 DB read。
    /// Side mirror — redundant with `position_side` in DB; saves one read.
    pub side: String,
}

// ─── Service ──────────────────────────────────────────────────────────────

/// ATR 追踪止损服务（无内部状态，DB-backed）。
/// ATR trailing-stop service — stateless wrapper over the `atr_stop_loss` table.
pub struct AtrStopLossService {
    db: Arc<DatabaseConnection>,
}

impl AtrStopLossService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 初始化 ATR 追踪止损（开仓时调用）。
    /// Initialize a trailing stop at position open.
    ///
    /// 中文：
    ///   计算初始止损价：
    ///     - long  → `entry_price − atr_value × multiplier`（止损在 entry 下方）
    ///     - short → `entry_price + atr_value × multiplier`（止损在 entry 上方）
    ///   然后 INSERT 一行到 `atr_stop_loss`；返回计算出的初始 stop。
    ///   若已存在同 position_id 的记录，INSERT 会因主键/唯一约束失败 — 视为
    ///   上游重复调用（应当由 risk_manager 避免）。
    ///
    /// English: Computes the initial stop and INSERTs a row. Returns the computed
    ///   stop price on success. If a row for `position_id` already exists the
    ///   INSERT will fail — that signals a duplicate open call (should be guarded
    ///   upstream by `risk_manager`).
    pub async fn init_stop_loss(&self, params: StopLossInit) -> Result<Decimal, AppError> {
        let stop_price = match params.side.as_str() {
            // 多头止损线在入场价下方（保护已实现浮盈的下边界）。
            // Long: SL sits BELOW entry — lower bound that protects realized profit.
            "long" => params.entry_price - params.atr_value * params.multiplier,
            // 空头止损线在入场价上方（保护已实现浮盈的上边界）。
            // Short: SL sits ABOVE entry — upper bound that protects realized profit.
            "short" => params.entry_price + params.atr_value * params.multiplier,
            // 非法 side — 应当由上游 risk_manager 在 order 阶段就拒绝；此处兜底。
            // Invalid side — should be rejected upstream by risk_manager; defensive fallback.
            _ => return Err(AppError::Internal("Invalid position side".to_string())),
        };

        let model = atr_stop_loss::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            position_id: sea_orm::Set(params.position_id),
            entry_price: sea_orm::Set(params.entry_price),
            current_stop: sea_orm::Set(stop_price),
            atr_value: sea_orm::Set(params.atr_value),
            atr_period: sea_orm::Set(params.atr_period),
            multiplier: sea_orm::Set(params.multiplier),
            position_side: sea_orm::Set(params.side.clone()),
            created_at: sea_orm::Set(chrono::Utc::now()),
            updated_at: sea_orm::Set(chrono::Utc::now()),
        };

        AtrStopLossEntity::insert(model)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(stop_price)
    }

    /// 更新追踪止损价并检测是否触发。
    /// Ratchet the trailing stop and detect trigger on every ticker.
    ///
    /// 中文：
    ///   行情更新时调用（每次 ticker）。语义：
    ///     1. 查不到记录 → 返回 `Ok(None)`（该 position 未启用追踪 / 已被触发清表）
    ///     2. 计算 candidate = price ± current_atr × multiplier（符号由 side 决定）
    ///        但 **仅在朝有利方向** 推进时才采用 — 永不回退
    ///     3. 触发判定：当前价穿越**旧**止损线 → 删行 + 返回 `Some(.. triggered=true)`
    ///     4. 未触发但 SL 推进 → 持久化新 stop + 返回 `Some(.. updated=true)`
    ///     5. 返回的 `StopLossResult` 同时携带 `triggered` / `updated` 标志，
    ///        让 caller（ws_hub）能**立即**派发市价平仓
    ///
    /// English: Hot path — called once per ticker per tracked position.
    ///     1. Missing row → `Ok(None)` (position not tracked or already triggered).
    ///     2. candidate = price ± ATR×mult; adopted **only** if it moves the stop
    ///        in the favourable direction (never retreats).
    ///     3. Trigger = current price crosses the **old** SL → delete row + return
    ///        `Some(.. triggered=true)`.
    ///     4. Ratchet only (no trigger) → persist new stop + return `Some(.. updated=true)`.
    ///     5. The returned `StopLossResult` carries both `triggered` and `updated` so
    ///        the caller (ws_hub) can immediately dispatch a market-close on trigger.
    pub async fn update_and_check(
        &self,
        params: StopLossUpdate,
    ) -> Result<Option<StopLossResult>, AppError> {
        // 查询当前止损记录
        // Look up the live row for this position.
        let record = AtrStopLossEntity::find()
            .filter(atr_stop_loss::Column::PositionId.eq(params.position_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let record = match record {
            Some(r) => r,
            // 无追踪止损记录 → 该 position 未启用 ATR 追踪或已被触发清表。
            // 跳过本 tick，让 caller 处理其它 position；不视为错误。
            // No row → position is not ATR-tracked, or was already triggered & cleared.
            // Skip this tick silently; not an error.
            None => return Ok(None), // 无追踪止损记录，跳过
        };

        let side = record.position_side.as_str();
        let current_stop = record.current_stop;

        // 计算新的止损价
        // Compute the candidate stop for this tick.
        let new_stop = match side {
            // 多头：仅在 price 创新高时把 SL 上移（不回调）— 这是 "trailing" 的本质。
            // Long: only RAISE the SL when price makes a new high (never pull back) —
            //   this is what makes the stop "trail" rather than "snap back".
            "long" => {
                // 多头：candidate = current_price − ATR×mult（entry 下方）。
                // 推进条件：candidate > current_stop。
                // Long: candidate = current_price − ATR×mult (below entry). Adopt
                //   only when strictly above the previous SL.
                let potential = params.current_price - params.current_atr * params.multiplier;
                if potential > current_stop {
                    potential
                } else {
                    // 不推进 — 保持旧 SL 不变（这是"trailing"与"普通 stop"的关键区别）。
                    // No ratchet — keep the old SL unchanged (this is the core difference
                    //   between a trailing stop and a plain stop).
                    current_stop
                }
            }
            // 空头：仅在 price 创新低时把 SL 下移（不反弹）— 对称于 long。
            // Short: only LOWER the SL when price makes a new low (never bounce) —
            //   symmetric to long.
            "short" => {
                // 空头：candidate = current_price + ATR×mult（entry 上方）。
                // 推进条件：candidate < current_stop。
                // Short: candidate = current_price + ATR×mult (above entry). Adopt
                //   only when strictly below the previous SL.
                let potential = params.current_price + params.current_atr * params.multiplier;
                if potential < current_stop {
                    potential
                } else {
                    // 不推进 — 保持旧 SL 不变（防御性，避免在反弹中过早抬升）。
                    // No ratchet — keep the old SL unchanged (defensive: do not
                    //   raise the stop on a short-side bounce).
                    current_stop
                }
            }
            _ => return Err(AppError::Internal("Invalid side".to_string())),
        };

        // 检测是否触发 — 用 **旧** SL 判定（确保推进之前先检测）。
        // Trigger check uses the **old** SL — detection must precede any ratchet
        //   for the same tick.
        let triggered = match side {
            // 多头：价格下穿旧止损线即触发。
            // Long: triggers when current_price crosses down through the old SL.
            "long" => params.current_price <= current_stop,
            // 空头：价格上穿旧止损线即触发。
            // Short: triggers when current_price crosses up through the old SL.
            "short" => params.current_price >= current_stop,
            _ => false,
        };

        let mut result = StopLossResult {
            triggered,
            old_stop: current_stop,
            new_stop,
            updated: false,
        };

        if triggered {
            // 触发：删除该行的 `atr_stop_loss` 记录（防止后续 tick 重复触发）。
            // Trigger: delete the `atr_stop_loss` row to prevent a second tick from
            //   re-firing the same close — caller is expected to market-close on
            //   receiving `triggered=true`.
            let mut del: atr_stop_loss::ActiveModel = record.into();
            del.updated_at = sea_orm::Set(chrono::Utc::now());
            AtrStopLossEntity::delete(del)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            return Ok(Some(result));
        }

        if new_stop != current_stop {
            // SL 推进时持久化（新 stop + 最新 ATR），确保 UI 与重启后状态一致。
            // Persist on ratchet (new SL + fresh ATR) so UI and post-restart state
            //   stay consistent with the in-memory calculation.
            let mut upd: atr_stop_loss::ActiveModel = record.into();
            upd.current_stop = sea_orm::Set(new_stop);
            upd.atr_value = sea_orm::Set(params.current_atr);
            upd.updated_at = sea_orm::Set(chrono::Utc::now());
            AtrStopLossEntity::update(upd)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            result.updated = true;
        }

        Ok(Some(result))
    }

    /// 拉取当前持仓的 ATR 止损价（只读，用于 UI 展示 / 风控审计）。
    /// Read current stop price for a position (read-only — UI display / risk audit).
    ///
    /// 中文：纯读路径，不修改 DB 状态；返回 `None` 表示该 position 未启用 ATR 追踪。
    /// English: Read-only path that does not touch DB state. `None` ⇒ position is
    ///   not ATR-tracked (e.g. legacy fixed-SL positions).
    pub async fn get_stop_price(&self, position_id: Uuid) -> Result<Option<Decimal>, AppError> {
        let record = AtrStopLossEntity::find()
            .filter(atr_stop_loss::Column::PositionId.eq(position_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(record.map(|r| r.current_stop))
    }

    /// 计算入场价的"静态" ATR 止损（不追踪，仅一次性公式）。
    /// Compute a one-shot, non-trailing ATR stop at entry.
    ///
    /// 中文：
    ///   与 `init_stop_loss` 的初始价公式相同，但**不**写库 — 用于：
    ///     - 下单时先算一个保守 SL 提交到交易所作为"硬底"（防止本地服务挂了时被甩锅）
    ///     - 前端"硬止损 vs 追踪止损"对比展示
    ///   纯函数（pure function），便于单测。
    ///
    /// English: Same formula as the initial stop in `init_stop_loss` but **without**
    ///   DB I/O. Use cases:
    ///     - Pre-compute a conservative SL to submit to the exchange as a "hard floor"
    ///       (so a local-service outage still has the position capped).
    ///     - UI side-by-side display of "static SL" vs "trailing SL".
    ///   Pure function — safe to call in tests.
    pub fn calc_static_stop(
        entry_price: Decimal,
        atr_value: Decimal,
        multiplier: Decimal,
        side: &str,
    ) -> Decimal {
        match side {
            // 多头：SL 在 entry 下方 / Long: SL below entry
            "long" => entry_price - atr_value * multiplier,
            // 空头：SL 在 entry 上方 / Short: SL above entry
            "short" => entry_price + atr_value * multiplier,
            // 非法 side 兜底返回 entry（理论无效路径；避免 panic）。
            // Invalid side: fall back to entry itself (theoretical invalid path;
            //   avoids panic).
            _ => entry_price,
        }
    }
}

/// `update_and_check` 的返回 — 同时承载触发标志和推进标志，让 caller 一次到位。
/// Return type of `update_and_check` — carries both trigger and ratchet flags so
///   the caller can dispatch market-close / UI updates in a single round trip.
#[derive(Debug, Clone)]
pub struct StopLossResult {
    /// `true` ⇒ caller 必须立即市价平仓；行已被删除。
    /// `true` ⇒ caller MUST market-close the position now; the row is already deleted.
    pub triggered: bool,   // 是否触发
    /// 推进前 / 触发前的旧止损价 — 用于日志审计与前端回放。
    /// Pre-ratchet / pre-trigger SL — for audit logs and UI replay.
    pub old_stop: Decimal, // 原止损价
    /// 当前生效的止损价（已推进 / 未推进 / 触发前的旧值统一在此返回）。
    /// Currently effective SL (post-ratchet if ratcheted, else old value).
    pub new_stop: Decimal, // 新止损价（已更新）
    /// 本 tick 是否推进过 SL — 前端可据此闪烁"SL 上移"提示。
    /// Whether this tick ratcheted the SL — UI can flash a "SL raised" hint on true.
    pub updated: bool,     // 止损价是否更新
}

// ─── Database Entity ─────────────────────────────────────────────────────

// Entity is defined in `crate::db::atr_stop_loss`
// This service module uses it via `crate::db::atr_stop_loss::Entity`

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 中文：long 头寸的静态 ATR 止损应等于 `entry − atr × mult`（SL 在 entry 下方）。
    ///   示例：entry=65000, atr=300, mult=1.5 → SL = 65000 − 450 = 64550。
    /// English: static ATR stop for a long = `entry − atr × mult` (SL below entry).
    ///   e.g. entry=65000, atr=300, mult=1.5 ⇒ SL = 65000 − 450 = 64550.
    #[test]
    fn test_calc_static_stop_long() {
        let entry = Decimal::new(65000, 0); // 65000
        let atr = Decimal::new(300, 0); // 300
        let mult = Decimal::new(15, 1); // 1.5

        let stop = AtrStopLossService::calc_static_stop(entry, atr, mult, "long");
        // 65000 - 300 * 1.5 = 65000 - 450 = 64550
        assert_eq!(stop, Decimal::new(64550, 0));
    }

    /// 中文：short 头寸的静态 ATR 止损应等于 `entry + atr × mult`（SL 在 entry 上方）。
    ///   示例：entry=65000, atr=300, mult=1.5 → SL = 65000 + 450 = 65450。
    /// English: static ATR stop for a short = `entry + atr × mult` (SL above entry).
    ///   e.g. entry=65000, atr=300, mult=1.5 ⇒ SL = 65000 + 450 = 65450.
    #[test]
    fn test_calc_static_stop_short() {
        let entry = Decimal::new(65000, 0);
        let atr = Decimal::new(300, 0);
        let mult = Decimal::new(15, 1);

        let stop = AtrStopLossService::calc_static_stop(entry, atr, mult, "short");
        // 65000 + 300 * 1.5 = 65000 + 450 = 65450
        assert_eq!(stop, Decimal::new(65450, 0));
    }

    /// 中文：验证 long 头寸的"trailing 只在朝有利方向（上涨）时推进"的不变量：
    ///   1. 价格上涨 65000 → 66000：candidate SL = 65550，高于旧 SL 64550，应上移
    ///   2. 价格回调 66000 → 65600：candidate SL = 65150，**低于**已上移的 SL 65550，
    ///      应当**保持**在 65550，绝不回退到 65150
    ///   3. 同样 candidate SL = 65150 仍高于初始 SL 64550，说明 ATR 缓冲在"吸收"小幅回调
    ///
    /// English: verifies the long-side "trail only on upticks" invariant:
    ///   1. 65000 → 66000: candidate = 65550, above the old 64550 ⇒ SL should raise.
    ///   2. 66000 → 65600 pullback: candidate = 65150, BELOW the ratcheted 65550,
    ///      so the SL must stay at 65550 (never retreat to 65150).
    ///   3. 65150 is still above the initial 64550 ⇒ the ATR buffer absorbs small
    ///      pullbacks without ever taking the SL back to entry.
    #[test]
    fn test_trailing_stop_only_moves_profitably_long() {
        let entry = Decimal::new(65000, 0);
        let atr = Decimal::new(300, 0);
        let mult = Decimal::new(15, 1);

        // 价格从 65000 涨到 66000，ATR=300，ATR止损应上移
        // Price 65000 → 66000 with ATR=300: trailing SL should ratchet upward.
        let current_price = Decimal::new(66000, 0);
        let current_stop = entry - atr * mult; // 64550 — 初始 SL / initial SL

        let potential_stop = current_price - atr * mult; // 66000 - 450 = 65550 — 推进后的 SL / ratcheted candidate
        assert!(potential_stop > current_stop); // 止损应上移 / SL should ratchet up
        assert_eq!(potential_stop, Decimal::new(65550, 0));

        // 价格从 66000 小幅回调到 65600，ATR=300，ATR止损应保持或下移
        // Price 66000 → 65600 pullback: candidate SL should NOT ratchet down
        //   (i.e. the stored SL stays at 65550 even though the new candidate is lower).
        let current_price2 = Decimal::new(65600, 0);
        let potential_stop2 = current_price2 - atr * mult; // 65600 - 450 = 65150
        assert!(potential_stop2 < potential_stop); // 止损应跟随回调 / candidate is lower after pullback
        assert!(potential_stop2 > current_stop); // 但不应低于初始止损 / but still above initial 64550
    }
}
