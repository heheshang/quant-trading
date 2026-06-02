//! services/trailing_stop.rs — 跟踪止损订单业务逻辑 (Trailing stop business logic, P1-2.3)
//!
//! 中文说明：
//!   本文件是 P1-2.3 跟踪止损订单的业务逻辑层，包含两大职责：
//!     1. `tick_one`       — 处理"单个"母单的当前价更新：朝有利方向移动时抬升/压低
//!                           `advanced_params.peak_price`，跌破/涨破 SL 时标记为
//!                           `filled` 并打 `triggered=true` 标志。
//!     2. `spawn_poll_loop`— 后台轮询 task：每 `poll_interval_secs` 秒（典型 2s）拉取
//!                           所有 active 母单所属 symbol 的最新价（默认 Binance
//!                           `GET /api/v3/ticker/price`），按 symbol 分组后逐个调
//!                           `tick_one`，避免 N+1 查询。
//!
//!   价格源策略：默认走 Binance REST（单 symbol、毫秒级、覆盖所有交易对），
//!   测试中可注入 mock `price_provider`。
//!
//! English description:
//!   Business-logic layer for P1-2.3 trailing stop orders. Two responsibilities:
//!     1. `tick_one`        — process a SINGLE trailing stop with one price update:
//!                            ratchet `advanced_params.peak_price` when price moves
//!                            favourably, and mark the parent as `filled` once the
//!                            stop is breached (the close order is created via the
//!                            regular `create_order` path; here we only persist
//!                            `triggered=true` into the params).
//!     2. `spawn_poll_loop` — background poller: every `poll_interval_secs` (typ. 2s)
//!                            fetch the latest price for every distinct symbol of
//!                            active trailing stops (default: Binance
//!                            `GET /api/v3/ticker/price`), group by symbol to avoid
//!                            the N+1 query problem, then call `tick_one` per order.
//!
//!   Price source: Binance REST by default (single-symbol, ms-level, covers all
//!   listed pairs). Tests inject a mock `price_provider`.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;
use tokio::time;
use uuid::Uuid;

use crate::db::order;
use crate::metrics;
use crate::models::trailing_stop_params::TrailingStopParams;
use crate::utils::error::AppError;

/// 母单 `orders.advanced_type` 列的判别符（discriminator）。
/// `advanced_type` discriminator for trailing stop parent orders.
///
/// 中文：
///   - 与 `db/order.rs::OrderType::TrailingStop` 的字符串映射保持一致（`"trailing_stop"`）
///   - 故意放在本文件顶层（不嵌 `mod advanced_type`）—— 本文件只有 1 个常量，
///     不需要 submodule 包装；Iceberg 那边是 2 个常量（`ICEBERG` + `ICEBERG_CHILD`）
///     才用 submodule。
///   - 用途：handler 写入、service 过滤、polling 时按 `advanced_type = TRAILING_STOP` 拉取
///
/// English:
///   - Mirrors the string mapping in `db/order.rs::OrderType::TrailingStop` (`"trailing_stop"`).
///   - Kept at the top level (no `mod advanced_type` submodule) because there is
///     only one constant here; Iceberg uses a submodule because it has TWO
///     constants (`ICEBERG` + `ICEBERG_CHILD`).
///   - Used by: handler writes, service-level filtering, and the polling loop's
///     `SELECT … WHERE advanced_type = TRAILING_STOP`.
pub const TRAILING_STOP: &str = "trailing_stop";

/// 处理单个母单 + 一次当前价 tick（核心：peak 跟踪 + 触发判定）。
/// Process one trailing stop order with a single current-price update.
///
/// 中文：
///   这是 polling loop 的核心单元 —— 对单个母单应用一次最新价：
///     1. 防御性检查：`advanced_type == "trailing_stop"` 且 `status == Pending`
///        （防止外部脏数据：例如普通限价单被误塞进 polling 队列）
///     2. 反序列化 `advanced_params` → `TrailingStopParams`
///     3. 调 `params.update_peak(current_price)`（朝有利方向单调跟随）
///     4. 调 `params.is_triggered(current_price)`（价格穿越 SL 则触发）
///     5. 三种分支：
///        - 触发：标记 `status = Filled`，写入 `triggered_at`/`triggered_by_price`，
///                指标 +1，返回 `Triggered`
///        - 仅 peak 更新：写回 `peak_price`，返回 `PeakUpdated`
///        - 无变化：返回 `NoChange`
///
///   # 参数
///   * `db`            — SeaORM 数据库连接
///   * `order_id`      — 母单 UUID
///   * `current_price` — 当前市场价（exchange 或测试 mock）
///
///   # 返回
///   `TickOutcome::{Triggered | PeakUpdated | NoChange | Skipped}`，
///   DB 错误透传为 `AppError::Database`。
///
/// English: Core per-order unit of the polling loop.
///     1. Defensive checks: `advanced_type == "trailing_stop"` AND `status == Pending`
///        (guards against dirty data — e.g. a plain limit order accidentally
///        ending up in the polling queue).
///     2. Deserialize `advanced_params` → `TrailingStopParams`.
///     3. Call `params.update_peak(current_price)` (monotonic ratchet).
///     4. Call `params.is_triggered(current_price)` (SL breach?).
///     5. Three branches:
///        - Triggered:  set `status = Filled`, persist `triggered_at`/
///          `triggered_by_price`, increment metric, return `Triggered`.
///        - Peak only:  persist new `peak_price`, return `PeakUpdated`.
///        - No change:  return `NoChange`.
///
///   # Arguments
///   * `db`            — SeaORM `DatabaseConnection`.
///   * `order_id`      — UUID of the parent order.
///   * `current_price` — Latest market price (exchange or test mock).
///
///   # Returns
///   `TickOutcome::{Triggered | PeakUpdated | NoChange | Skipped}`. DB errors
///   bubble up as `AppError::Database`.
pub async fn tick_one(
    db: &Arc<DatabaseConnection>,
    order_id: Uuid,
    current_price: f64,
) -> Result<TickOutcome, AppError> {
    let parent = order::Entity::find_by_id(order_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("find trailing stop {}: {}", order_id, e)))?
        .ok_or_else(|| AppError::NotFound(format!("trailing stop order {} not found", order_id)))?;

    // 防御性检查 1：只处理 `advanced_type = "trailing_stop"` 的母单。
    // 防止外部脏数据（例如普通限价单被误塞进 polling 队列）。
    // Defensive check 1: only handle trailing_stop parents.
    // Guards against dirty data (e.g. a plain limit order accidentally ending
    // up in the polling queue).
    if parent.advanced_type.as_deref() != Some(TRAILING_STOP) {
        return Ok(TickOutcome::Skipped);
    }

    // 防御性检查 2：跳过已 `Filled` / `Cancelled` / `PartiallyFilled` 的母单。
    // 即使 polling 周期内反复拉取到已触发订单，也不会重复触发。
    // Defensive check 2: skip orders that are no longer Pending.
    // Even if a triggered order is re-fetched in a subsequent poll cycle, this
    // prevents double-trigger.
    if parent.status != order::OrderStatus::Pending {
        return Ok(TickOutcome::Skipped);
    }

    // 反序列化 `orders.advanced_params` JSONB → `TrailingStopParams`。
    // 缺失/格式错误按 `AppError::Internal` 抛出（说明入库数据有 bug，需人工修复）。
    // Deserialize JSONB → `TrailingStopParams`.
    // Missing / malformed params bubble up as `AppError::Internal` — this
    // indicates a data-integrity bug at write time, not a user-facing error.
    let mut params: TrailingStopParams = match parent.advanced_params.as_ref() {
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
            AppError::Internal(format!(
                "trailing stop {} invalid advanced_params: {}",
                order_id, e
            ))
        })?,
        None => {
            return Err(AppError::Internal(format!(
                "trailing stop {} missing advanced_params",
                order_id
            )));
        }
    };

    // 记录旧 peak 以便后续判断"是否真发生变化"（避免无意义的 DB 写）。
    // Record the old peak so we can decide whether a DB write is actually needed
    // (avoids no-op UPDATE statements).
    let old_peak = params.peak_price;
    // 朝有利方向单调跟随（详见 `TrailingStopParams::update_peak`）。
    // Monotonic ratchet in the favourable direction (see `TrailingStopParams::update_peak`).
    params.update_peak(current_price);
    // 浮点比较用 EPSILON：f64 在序列化/反序列化后可能有 1 ULP 的抖动。
    // Float comparison with EPSILON: f64 may shift by 1 ULP after
    // serialize / deserialize round-trip — we only want a true write when
    // the value actually moved.
    let peak_changed = (params.peak_price - old_peak).abs() > f64::EPSILON;
    // 触发判定：Buy → current_price ≤ SL；Sell → current_price ≥ SL。
    // Trigger check: Buy → price ≤ SL; Sell → price ≥ SL.
    let triggered = params.is_triggered(current_price);

    if triggered {
        // ─── 触发分支：标记母单为 `Filled`，把触发信息持久化 ───
        // Triggered branch: mark parent as Filled, persist trigger metadata.
        // v1 由下游"已填的母单"事件触发 close order 的创建（详见
        // `docs/p1-2-3-trailing-stop/architecture/T2_Design.md` §3.4 D5）。
        // v1: the close order is created downstream by the regular
        // `create_order` path (see T2_Design.md §3.4 D5).
        let mut active: order::ActiveModel = parent.into();
        active.status = Set(order::OrderStatus::Filled);
        active.filled_at = Set(Some(chrono::Utc::now()));
        let new_params = json!({
            "peak_price": params.peak_price,
            "trailing_distance": params.trailing_distance,
            "side": params.side,
            "triggered_at": chrono::Utc::now(),
            "triggered_by_price": current_price,
        });
        active.advanced_params = Set(Some(new_params));
        active.updated_at = Set(chrono::Utc::now());

        active
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::Database(format!("update trailing stop {}: {}", order_id, e)))?;

        // Prometheus 指标：按 side 维度 +1（`buy` / `sell`）。
        // Prometheus metric: +1, labelled by side (`buy` / `sell`).
        metrics::TRAILING_STOP_TRIGGERED_TOTAL
            .with_label_values(&[params.side.as_str()])
            .inc();

        tracing::info!(
            "Trailing stop {} triggered at price={} (peak={}, distance={})",
            order_id,
            current_price,
            params.peak_price,
            params.trailing_distance
        );

        Ok(TickOutcome::Triggered)
    } else if peak_changed {
        // ─── peak 更新分支：仅写回 `peak_price`（未触发 SL）───
        // Peak-only branch: just persist the new `peak_price` (SL not breached).
        // 用 `parent.clone().into()`：与 `if triggered` 分支独立持有 ActiveModel，
        // 互不影响（因为 `parent` 已被 `into()` 消费）。
        // Use `parent.clone().into()` because the `if triggered` branch already
        // consumed `parent` via `into()` — we need a fresh ActiveModel here.
        let mut active: order::ActiveModel = parent.clone().into();
        let new_params = json!({
            "peak_price": params.peak_price,
            "trailing_distance": params.trailing_distance,
            "side": params.side,
        });
        active.advanced_params = Set(Some(new_params));
        active.updated_at = Set(chrono::Utc::now());

        active
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::Database(format!("update peak_price {}: {}", order_id, e)))?;

        tracing::debug!(
            "Trailing stop {} peak moved {} → {}",
            order_id,
            old_peak,
            params.peak_price
        );

        Ok(TickOutcome::PeakUpdated)
    } else {
        // ─── 无变化分支：价格未触发 SL 且 peak 未变 ───
        // No-change branch: price did not breach SL AND peak did not move.
        // 完全跳过 DB 写 —— 这是高频轮询的吞吐关键（一次 2s 周期中 99% 的 tick
        // 都会落到这里）。
        // No DB write at all — this is critical for the 2s poll throughput
        // (~99% of ticks fall into this branch).
        Ok(TickOutcome::NoChange)
    }
}

/// `tick_one` 的返回枚举 —— 4 种互斥结局。
/// `tick_one` return enum — four mutually exclusive outcomes.
///
/// 中文：
///   调用方根据返回值可以统计/打点/告警：
///     - `Skipped`      → 防御性跳过（数据脏或已 non-Pending）
///     - `PeakUpdated`  → 写了一次 DB（仅 peak）
///     - `Triggered`    → 写了一次 DB 且订单已 `Filled`
///     - `NoChange`     → 完全没写 DB（最高频分支）
///
/// English: Returned by `tick_one`; the caller can aggregate / alert on it:
///   - `Skipped`     — defensive skip (dirty data or non-Pending).
///   - `PeakUpdated` — one DB write (peak only).
///   - `Triggered`   — one DB write AND the order is now `Filled`.
///   - `NoChange`    — no DB write at all (most common branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// 跳过（不是 trailing_stop / 已 Filled / Cancelled 等）。Defensive skip.
    Skipped,
    /// `peak_price` 已更新。`peak_price` was updated.
    PeakUpdated,
    /// 触发：母单已标记为 `Filled`。Stop triggered; parent marked as `Filled`.
    Triggered,
    /// 无变化（价格未触发 SL，peak 也未变）。No change — no DB write.
    NoChange,
}

/// 启动后台轮询 task，返回 `JoinHandle`（测试里可以 `.await` 等其完成）。
/// Spawn the background polling task. Returns a `JoinHandle` for tests.
///
/// 中文：
///   - 进程级长生命周期 task：在 `main.rs` 启动时 `spawn` 一次，跟随 backend 进程
///     同寿（`tokio` runtime 退出时一并结束）
///   - `MissedTickBehavior::Skip`：故意选 Skip 而非 Burst/Delay，目的是
///     **避免 tick 积压** —— 如果一次 `poll_once` 跑了 5s，下个 tick 就直接
///     跳过，不要连续追赶（追赶会导致 API 限流）
///   - 错误恢复：单次 `poll_once` 失败只 `tracing::error!` 记录，**不会 kill task**，
///     下个周期照常重试（详见 T2_Design.md §2.4）
///   - 价格源抽象为 `Fn(&str) -> BoxFuture<Result<f64, String>>`：默认实现走
///     `binance_rest::fetch_ticker_price`；测试中可注入 mock 闭包
///
///   # 参数
///   * `db`                — SeaORM `DatabaseConnection`（`Arc` 共享）
///   * `poll_interval_secs`— 轮询周期（典型 2s；可用 env var `TRAILING_POLL_INTERVAL_SECS` 覆盖）
///   * `price_provider`    — 价格源闭包（symbol → Result<price, String>）
///
/// English: Long-lived background task spawned once at startup.
///   - Process-lifetime: spawned in `main.rs`, lives as long as the backend.
///   - `MissedTickBehavior::Skip` (NOT `Burst` / `Delay`): avoids tick pile-up
///     — if one `poll_once` takes 5s, the next tick is dropped rather than
///     firing back-to-back (which would trigger exchange rate limits).
///   - Error recovery: a single failed `poll_once` only logs an error; the
///     task is NOT killed. The next cycle retries normally (see
///     T2_Design.md §2.4).
///   - Price source abstracted as `Fn(&str) -> BoxFuture<Result<f64, String>>`:
///     default impl wraps `binance_rest::fetch_ticker_price`; tests inject a mock.
///
///   # Arguments
///   * `db` — SeaORM `DatabaseConnection` (shared via `Arc`).
///   * `poll_interval_secs` — Seconds between polls (typ. 2s; overridable via
///     the `TRAILING_POLL_INTERVAL_SECS` env var).
///   * `price_provider` — Price source closure (symbol → Result<price, String>).
pub fn spawn_poll_loop<F>(
    db: Arc<DatabaseConnection>,
    poll_interval_secs: u64,
    price_provider: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn(&str) -> futures::future::BoxFuture<'static, Result<f64, String>> + Send + Sync + 'static,
{
    tokio::spawn(async move {
        // `tokio::time::interval` —— 周期定时器，首次 `tick()` 立即触发，之后按
        // `Duration` 周期触发。
        // `tokio::time::interval`: periodic ticker. The FIRST `tick()` fires
        // immediately; subsequent ticks are spaced by `Duration`.
        let mut ticker = time::interval(Duration::from_secs(poll_interval_secs));
        // Skip 而非 Burst/Delay —— 详见 T2_Design.md §2.4（避免积压 + 限流）。
        // Skip (not Burst / Delay) — see T2_Design.md §2.4 (avoids pile-up
        // + exchange rate limits).
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            // 等到下一个 tick。Wait for the next tick.
            ticker.tick().await;
            // 单次失败不 kill task：log 后继续循环（错误恢复策略）。
            // A single failure does NOT kill the task: log and continue
            // (error-recovery policy).
            if let Err(e) = poll_once(&db, &price_provider).await {
                tracing::error!("trailing stop poll loop error: {}", e);
            }
        }
    })
}

/// 单次轮询周期：拉 active 母单 → 按 symbol 分组 → 每 symbol 取价 → 每订单 tick。
/// One iteration of the poll loop: load active trailing stops, group by symbol,
/// fetch price per symbol, call `tick_one` per order.
///
/// 中文：
///   4 步流程（与 T2_Design.md §3.1 架构图完全对应）：
///     1. 加载所有 active 母单（`advanced_type = TRAILING_STOP` AND `status = Pending`）
///     2. 按 `symbol` 分组到 `HashSet<String>` —— **避免 N+1**：N 个不同 symbol
///        只调 N 次 exchange API，而不是 N 个订单调 N 次
///     3. 顺序对每个 symbol 调 `price_provider`（顺序而非并发 —— 尊重 exchange
///        限流；并发度 > N 会触发 429）
///     4. 把价格分发给该 symbol 下所有母单，逐个 `tick_one`
///
///   单价拉取失败：warn 后跳过该 symbol 下所有母单，**不让 1 个坏 symbol 阻断
///   整个 polling cycle**。
///   单订单 `tick_one` 失败：error 后继续处理剩余订单。
///
/// English: 4-step flow (mirrors T2_Design.md §3.1):
///   1. Load all active parents (`advanced_type = TRAILING_STOP` AND
///      `status = Pending`).
///   2. Group by `symbol` into a `HashSet<String>` — **avoids the N+1 problem**:
///      N distinct symbols ⇒ N exchange calls, NOT one per order.
///   3. For each symbol, call `price_provider` SEQUENTIALLY (NOT concurrently —
///      to respect exchange rate limits; >N concurrent calls would 429).
///   4. Fan the price out to all parents of that symbol, calling `tick_one`
///      per order.
///
///   Per-symbol fetch failure: `warn!` and skip that symbol's parents — one
///   bad symbol MUST NOT block the rest of the polling cycle.
///   Per-order `tick_one` failure: `error!` and continue with the next order.
pub async fn poll_once<F>(db: &Arc<DatabaseConnection>, price_provider: &F) -> Result<(), AppError>
where
    F: Fn(&str) -> futures::future::BoxFuture<'static, Result<f64, String>> + Send + Sync + 'static,
{
    // 步骤 1：拉取所有 active 母单（`advanced_type = trailing_stop` + `status = Pending`）。
    // 早返回：空集合时直接 `Ok(())` —— 没有母单时 0 API 调用、0 DB 写。
    // Step 1: load all active parents. Early-return `Ok(())` when empty —
    // 0 active orders ⇒ 0 API calls, 0 DB writes (the common idle case).
    let active_orders = order::Entity::find()
        .filter(order::Column::AdvancedType.eq(TRAILING_STOP))
        .filter(order::Column::Status.eq(order::OrderStatus::Pending))
        .all(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("load trailing stops: {}", e)))?;

    if active_orders.is_empty() {
        return Ok(());
    }

    // 步骤 2：按 `symbol` 分组到 `HashSet`，自动去重。
    // 例如 50 个 BTCUSDT 母单只对应 1 个 symbol → 1 次 API 调用。
    // Step 2: group by `symbol` into a `HashSet` (auto-deduplicates).
    // E.g. 50 BTCUSDT parents → 1 symbol → 1 API call.
    let mut symbols: HashSet<String> = HashSet::new();
    for o in &active_orders {
        symbols.insert(o.symbol.clone());
    }

    // 步骤 3+4：顺序对每个 symbol 取价，再 fan-out 给该 symbol 下所有母单。
    // Step 3+4: for each symbol, fetch the price, then fan out to all parents
    // of that symbol.
    for sym in &symbols {
        // 顺序而非并发：避免触发 exchange 限流（429）。
        // Sequential, NOT concurrent: avoids exchange rate limits (429).
        let price_result = price_provider(sym).await;
        match price_result {
            Ok(price) => {
                // 价格取成功 → 遍历该 symbol 下所有母单，逐个 `tick_one`。
                // Price fetched OK → iterate all parents of this symbol, tick each.
                for o in active_orders.iter().filter(|o| &o.symbol == sym) {
                    // 单订单 `tick_one` 失败不阻断其他订单（容错：详见 poll_once 文档）。
                    // A single `tick_one` failure does NOT block other orders
                    // (fault tolerance — see poll_once doc).
                    if let Err(e) = tick_one(db, o.id, price).await {
                        tracing::error!("tick_one failed for {}: {}", o.id, e);
                    }
                }
            }
            Err(e) => {
                // 单 symbol 取价失败 → warn 后跳过该 symbol 下所有母单（不影响其他 symbol）。
                // One symbol failed → warn and skip ALL parents of that symbol
                // (does NOT affect other symbols — fault isolation).
                tracing::warn!("price fetch failed for {}: {}", sym, e);
            }
        }
    }

    Ok(())
}

// `Set` re-export for the `active.status = Set(...)` lines above.
// `Set` 的 re-export —— 用于上面 `active.status = Set(...)` 这种调用。
// `Set` re-export — used in the `active.status = Set(...)` calls above.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::trailing_stop_params::TrailingStopParams;

    // 单元测试：只覆盖纯逻辑 + 枚举值 + 常量。
    // `tick_one` 涉及 DB，由集成测试 / E2E 覆盖。
    //
    // Unit tests: pure logic + enum values + constants only.
    // `tick_one` requires a DB connection — covered by integration / E2E tests.

    /// 中文：验证 `TickOutcome` 的 `PartialEq` 行为（`assert_eq!` 路径）。
    ///   - 同变体相等
    ///   - 不同变体不等
    ///
    /// English: verify `TickOutcome`'s `PartialEq` behaviour:
    ///   - same variant ⇒ equal
    ///   - different variants ⇒ not equal
    #[test]
    fn tick_outcome_eq() {
        assert_eq!(TickOutcome::Triggered, TickOutcome::Triggered);
        assert_ne!(TickOutcome::PeakUpdated, TickOutcome::Triggered);
    }

    /// 中文：验证 `TRAILING_STOP` 常量与 `db/order.rs::OrderType::TrailingStop` 的
    ///   字符串映射一致（防止误改字符串导致 round-trip 失败）。
    /// English: verify `TRAILING_STOP` matches the string mapping in
    ///   `db/order.rs::OrderType::TrailingStop` (guards against accidental
    ///   rename breaking the DB round-trip).
    #[test]
    fn trailing_stop_constant() {
        assert_eq!(TRAILING_STOP, "trailing_stop");
    }

    /// 中文：service 层与 model 层的 `TrailingStopParams` 一致性冒烟测试。
    ///   - service 用的 `validate_new` / `update_peak` / `is_triggered` 必须与
    ///     `models/trailing_stop_params.rs` 行为完全一致
    ///   - 关键场景：entry=100, distance=0.05 → peak=110 时 SL=104.5，
    ///     108 不触发，104 触发
    ///
    /// English: smoke test that the service layer uses the same
    ///   `TrailingStopParams` shape as the model layer.
    ///   - `validate_new` / `update_peak` / `is_triggered` must behave
    ///     identically to `models/trailing_stop_params.rs`.
    ///   - Key scenario: entry=100, distance=0.05 → peak=110 ⇒ SL=104.5,
    ///     so 108 does NOT trigger, 104 DOES trigger.
    #[test]
    fn integration_update_peak_via_service_layer_params() {
        // Smoke test: ensure the params struct used by service matches model
        // 冒烟测试：service 用到的 params struct 与 model 一致
        let p = TrailingStopParams::validate_new(100.0, 0.05, "buy").unwrap();
        let mut p2 = p.clone();
        p2.update_peak(110.0);
        assert_eq!(p2.peak_price, 110.0);
        assert!(!p2.is_triggered(108.0)); // trigger = 104.5
        assert!(p2.is_triggered(104.0));
    }
}
