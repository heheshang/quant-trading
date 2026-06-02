//! services/matching_engine.rs — 模拟撮合引擎
//!
//! PRD: US-TE-07 (模拟撮合引擎)
//! ADR: ADR-TRADING-EXECUTION D1 (内存撮合+异步DB写入), D4 (深度变更事件驱动), D7 (加权平均均价)

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::db::order::{self as order_model, OrderSide, OrderStatus, OrderType};
use crate::utils::error::AppError;

// ─── Lock Helpers ───────────────────────────────────────────────

/// Acquire a `Mutex` lock and convert a `PoisonError` into `AppError::Internal`.
///
/// In a long-running process a panic inside the critical section would otherwise
/// poison the lock and cause every subsequent `lock().unwrap()` to panic again,
/// turning one transient error into a service-wide outage.
fn lock_or_internal<'a, T>(mutex: &'a Mutex<T>, ctx: &str) -> Result<MutexGuard<'a, T>, AppError> {
    mutex
        .lock()
        .map_err(|e| AppError::Internal(format!("Mutex poisoned ({}): {}", ctx, e)))
}

/// Acquire an `RwLock` read guard and convert a `PoisonError` into `AppError::Internal`.
///
/// Reserved for future `RwLock` migrations in `MatchingEngine` (e.g. read-heavy
/// order book snapshot paths). Not yet used; current locks are `Mutex` only.
#[allow(dead_code)]
fn read_lock_or_internal<'a, T>(
    lock: &'a RwLock<T>,
    ctx: &str,
) -> Result<RwLockReadGuard<'a, T>, AppError> {
    lock.read()
        .map_err(|e| AppError::Internal(format!("RwLock poisoned ({}): {}", ctx, e)))
}

/// Acquire an `RwLock` write guard and convert a `PoisonError` into `AppError::Internal`.
///
/// Reserved for future `RwLock` migrations in `MatchingEngine`.
#[allow(dead_code)]
fn write_lock_or_internal<'a, T>(
    lock: &'a RwLock<T>,
    ctx: &str,
) -> Result<RwLockWriteGuard<'a, T>, AppError> {
    lock.write()
        .map_err(|e| AppError::Internal(format!("RwLock poisoned ({}): {}", ctx, e)))
}

// ─── Types ──────────────────────────────────────────────────────

/// 订单簿条目（内存中的订单引用）
#[derive(Debug, Clone)]
pub struct OrderEntry {
    pub order_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub price: Option<f64>,
    pub remaining_quantity: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 单个交易对的订单簿
#[derive(Debug, Default)]
pub struct OrderBook {
    /// 买盘: 价格降序 (Reverse 实现降序)
    pub bids: BTreeMap<std::cmp::Reverse<OrderedFloat>, VecDeque<OrderEntry>>,
    /// 卖盘: 价格升序
    pub asks: BTreeMap<OrderedFloat, VecDeque<OrderEntry>>,
}

/// 包装 f64 使其实现 Ord (BTreeMap 要求)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// 单笔成交记录
#[derive(Debug, Clone)]
pub struct MatchTrade {
    pub order_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub is_maker: bool,
}

/// 撮合结果
#[derive(Debug)]
pub struct MatchResult {
    pub order_id: uuid::Uuid,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub total_fee: f64,
    pub trades: Vec<MatchTrade>,
    pub is_fully_filled: bool,
}

/// 成交记录（异步写入DB）
#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub order_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub is_maker: bool,
}

/// 深度数据（从行情服务读取）
#[derive(Debug, Clone)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct DepthData {
    pub bids: Vec<DepthLevel>, // 价格降序
    pub asks: Vec<DepthLevel>, // 价格升序
    pub timestamp: i64,
}

// ─── MatchingEngine ─────────────────────────────────────────────

pub struct MatchingEngine {
    /// 内存订单簿: symbol → OrderBook
    order_books: std::sync::Mutex<HashMap<String, OrderBook>>,

    /// 深度数据缓存 (从行情服务读取)
    depth_cache: std::sync::Mutex<HashMap<String, DepthData>>,

    /// 成交记录异步写入通道
    trade_sink: mpsc::Sender<TradeRecord>,

    /// 数据库连接
    db: Arc<DatabaseConnection>,

    /// 模拟延迟 (ms)
    delay_ms: u64,

    /// 手续费率
    fee_rate: f64,
}

impl MatchingEngine {
    /// P1-2.1: returns `Arc<Self>` so the spawn task can hold a clone while the
    /// caller can also wrap the engine in another Arc for state sharing.
    /// Breaking change vs P1-2 MVP: callers must now use `MatchingEngine::new(...)`
    /// directly (no `Arc::new(MatchingEngine::new(...))` wrapper needed).
    pub fn new(db: Arc<DatabaseConnection>, delay_ms: u64, fee_rate: f64) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<TradeRecord>(1024);
        let engine = Self {
            order_books: std::sync::Mutex::new(HashMap::new()),
            depth_cache: std::sync::Mutex::new(HashMap::new()),
            trade_sink: tx,
            db,
            delay_ms,
            fee_rate,
        };

        // P1-2.1: spawn_trade_writer takes &Arc<Self> and clones the Arc into
        // the spawned task. Returning the Arc lets the caller share ownership.
        let engine_arc = Arc::new(engine);
        engine_arc.spawn_trade_writer(rx);
        engine_arc
    }

    /// 启动时从 PG 重建订单簿 (pending / partial_filled 订单)
    ///
    /// ADR D1: 服务重启时需从 PG 加载恢复订单簿
    pub async fn rebuild_order_book(&self) -> Result<(), AppError> {
        info!("Rebuilding order book from PostgreSQL...");

        let orders = order_model::Entity::find()
            .filter(
                order_model::Column::Status
                    .is_in([OrderStatus::Pending, OrderStatus::PartialFilled]),
            )
            .filter(order_model::Column::OrderType.eq(OrderType::Limit))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut books = lock_or_internal(&self.order_books, "rebuild_order_book")?;
        for order in orders {
            let entry = OrderEntry {
                order_id: order.id,
                user_id: order.user_id,
                symbol: order.symbol.clone(),
                side: order.side.clone(),
                price: order.price,
                remaining_quantity: order.quantity - order.filled_quantity,
                created_at: order.created_at,
            };

            let book = books.entry(order.symbol.clone()).or_default();
            if let Some(price) = order.price {
                match order.side {
                    OrderSide::Buy => {
                        book.bids
                            .entry(std::cmp::Reverse(OrderedFloat(price)))
                            .or_default()
                            .push_back(entry);
                    }
                    OrderSide::Sell => {
                        book.asks
                            .entry(OrderedFloat(price))
                            .or_default()
                            .push_back(entry);
                    }
                }
            }
        }

        info!("Order book rebuilt successfully");
        Ok(())
    }

    /// 将限价单插入订单簿
    ///
    /// ADR D1: 限价单挂入订单簿等待深度触发
    pub fn insert_limit_order(&self, entry: OrderEntry) {
        let symbol = entry.symbol.clone();
        let price = match entry.price {
            Some(p) => p,
            None => {
                warn!(
                    "Market order should not be inserted into order book, order_id={}",
                    entry.order_id
                );
                return;
            }
        };

        let mut books = lock_or_internal(&self.order_books, "insert_limit_order").expect("order_books lock");
        let book = books.entry(symbol).or_default();
        match entry.side {
            OrderSide::Buy => {
                book.bids
                    .entry(std::cmp::Reverse(OrderedFloat(price)))
                    .or_default()
                    .push_back(entry);
            }
            OrderSide::Sell => {
                book.asks
                    .entry(OrderedFloat(price))
                    .or_default()
                    .push_back(entry);
            }
        }
    }

    /// 执行市价单撮合
    ///
    /// PRD: US-TE-02 (市价单)
    /// ADR D1: 市价单立即按深度数据逐档撮合，不进入订单簿
    pub async fn match_market(
        &self,
        order_id: uuid::Uuid,
        user_id: uuid::Uuid,
        symbol: &str,
        side: &OrderSide,
        quantity: f64,
    ) -> Result<MatchResult, AppError> {
        // 1. 获取深度数据
        let depth = {
            let cache = lock_or_internal(&self.depth_cache, "match_market")?;
            cache
                .get(symbol)
                .cloned()
                .ok_or_else(|| AppError::Internal("行情数据暂不可用，无法提交市价单".to_string()))?
        };

        // 2. 检查是否有对手盘
        let levels = match side {
            OrderSide::Buy => &depth.asks,
            OrderSide::Sell => &depth.bids,
        };
        if levels.is_empty() {
            return Err(AppError::Internal(
                "当前市场无对手盘，市价单无法成交".to_string(),
            ));
        }

        // 3. 逐档撮合
        let mut remaining = quantity;
        let mut filled = 0.0_f64;
        let mut total_cost = 0.0_f64;
        let mut total_fee = 0.0_f64;
        let mut trades = Vec::new();

        for level in levels {
            if remaining <= 1e-12 {
                break;
            }

            let match_qty = remaining.min(level.quantity);
            let trade_fee = match_qty * level.price * self.fee_rate;

            trades.push(MatchTrade {
                order_id,
                user_id,
                symbol: symbol.to_string(),
                side: side.clone(),
                price: level.price,
                quantity: match_qty,
                fee: trade_fee,
                is_maker: false, // 市价单始终为 taker
            });

            filled += match_qty;
            total_cost += match_qty * level.price;
            total_fee += trade_fee;
            remaining -= match_qty;
        }

        // P0-3: count filled quantity by side (label set bounded)
        if filled > 1e-12 {
            let side_label = match &side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            };
            crate::metrics::ORDERS_FILLED_TOTAL
                .with_label_values(&[side_label, "local"])
                .inc_by(filled as u64);
        }

        let avg_price = if filled > 1e-12 {
            Some(total_cost / filled)
        } else {
            None
        };

        // 4. 模拟延迟
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        // 5. 异步写入成交记录
        for trade in &trades {
            let _ = self
                .trade_sink
                .send(TradeRecord {
                    order_id,
                    user_id,
                    symbol: symbol.to_string(),
                    side: trade.side.clone(),
                    price: trade.price,
                    quantity: trade.quantity,
                    fee: trade.fee,
                    is_maker: trade.is_maker,
                })
                .await;
        }

        Ok(MatchResult {
            order_id,
            filled_quantity: filled,
            avg_fill_price: avg_price,
            total_fee,
            trades,
            is_fully_filled: remaining <= 1e-12,
        })
    }

    /// 限价单撮合（深度变更触发）
    ///
    /// PRD: US-TE-07 (模拟撮合引擎)
    /// ADR D4: 深度变更事件驱动
    pub fn on_depth_update(&self, symbol: &str, depth: &DepthData) -> Result<(), AppError> {
        // 1. 获取该交易对的订单簿（可写锁）
        let mut books = lock_or_internal(&self.order_books, "on_depth_update")?;
        let book = match books.get_mut(symbol) {
            Some(b) => b,
            None => return Ok(()),
        };

        // 深度数据为空直接返回
        if depth.asks.is_empty() || depth.bids.is_empty() {
            return Ok(());
        }

        // 2. 构建深度档位映射: price -> (quantity, level_index)
        //    用于多档深度撮合
        let ask_levels: Vec<(f64, f64)> = depth
            .asks
            .iter()
            .map(|lvl| (lvl.price, lvl.quantity))
            .collect();
        let bid_levels: Vec<(f64, f64)> = depth
            .bids
            .iter()
            .map(|lvl| (lvl.price, lvl.quantity))
            .collect();

        let mut trades_to_emit: Vec<TradeRecord> = Vec::new();

        // 3. 遍历买盘: price >= best_ask 的订单可撮合（买入吃卖单）
        for (price_key, queue) in book.bids.iter_mut() {
            let price = price_key.0.0;
            if price < ask_levels[0].0 {
                continue; // 价格不够好，跳过
            }

            // 找到深度档位: ask 价格 <= 订单价格 的最优档位
            let Some((_, ask_qty_at_price)) =
                ask_levels.iter().find(|(ask_price, _)| *ask_price <= price)
            else {
                continue;
            };

            // 遍历该价格队列中的所有订单
            let mut i = 0;
            while i < queue.len() {
                let entry = &mut queue[i];
                let fill_qty = entry.remaining_quantity.min(*ask_qty_at_price);
                if fill_qty <= 0.0 {
                    i += 1;
                    continue;
                }

                // 计算成交费用
                let trade_fee = fill_qty * price * self.fee_rate;
                trades_to_emit.push(TradeRecord {
                    order_id: entry.order_id,
                    user_id: entry.user_id,
                    symbol: entry.symbol.clone(),
                    side: entry.side.clone(),
                    price,
                    quantity: fill_qty,
                    fee: trade_fee,
                    is_maker: true,
                });

                // P0-3: count each limit-order fill
                let side_label = match &entry.side {
                    OrderSide::Buy => "buy",
                    OrderSide::Sell => "sell",
                };
                crate::metrics::ORDERS_FILLED_TOTAL
                    .with_label_values(&[side_label, "local"])
                    .inc_by(fill_qty as u64);

                // 更新剩余数量
                entry.remaining_quantity -= fill_qty;

                if entry.remaining_quantity <= 0.0 {
                    // 订单完全成交，从队列移除
                    queue.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        // 4. 遍历卖盘: price <= best_bid 的订单可撮合（卖出吃买单）
        for (price_key, queue) in book.asks.iter_mut() {
            let price = price_key.0;
            if price > bid_levels[0].0 {
                continue; // 价格不够好，跳过
            }

            let Some((_, bid_qty_at_price)) =
                bid_levels.iter().find(|(bid_price, _)| *bid_price >= price)
            else {
                continue;
            };

            let mut i = 0;
            while i < queue.len() {
                let entry = &mut queue[i];
                let fill_qty = entry.remaining_quantity.min(*bid_qty_at_price);
                if fill_qty <= 0.0 {
                    i += 1;
                    continue;
                }

                let trade_fee = fill_qty * price * self.fee_rate;
                trades_to_emit.push(TradeRecord {
                    order_id: entry.order_id,
                    user_id: entry.user_id,
                    symbol: entry.symbol.clone(),
                    side: entry.side.clone(),
                    price,
                    quantity: fill_qty,
                    fee: trade_fee,
                    is_maker: true,
                });

                // P0-3: count each limit-order fill
                let side_label = match &entry.side {
                    OrderSide::Buy => "buy",
                    OrderSide::Sell => "sell",
                };
                crate::metrics::ORDERS_FILLED_TOTAL
                    .with_label_values(&[side_label, "local"])
                    .inc_by(fill_qty as u64);

                entry.remaining_quantity -= fill_qty;

                if entry.remaining_quantity <= 0.0 {
                    queue.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        drop(books);

        // 5. 批量发送成交记录（原子化，避免部分发送）
        for record in trades_to_emit {
            let _ = self.trade_sink.try_send(record);
        }

        Ok(())
    }

    /// 从订单簿移除订单（撤单时调用）
    pub fn remove_from_book(&self, order_id: uuid::Uuid) {
        let mut books = lock_or_internal(&self.order_books, "remove_from_book")
            .expect("order_books lock");
        for (_symbol, book) in books.iter_mut() {
            // 检查买盘
            for (_, queue) in book.bids.iter_mut() {
                queue.retain(|e| e.order_id != order_id);
            }
            // 检查卖盘
            for (_, queue) in book.asks.iter_mut() {
                queue.retain(|e| e.order_id != order_id);
            }

            // 清理空队列
            book.bids.retain(|_, q| !q.is_empty());
            book.asks.retain(|_, q| !q.is_empty());
        }
    }

    /// 更新深度数据缓存
    pub fn update_depth(&self, symbol: &str, depth: DepthData) {
        let mut cache = lock_or_internal(&self.depth_cache, "update_depth")
            .expect("depth_cache lock");
        cache.insert(symbol.to_string(), depth);
    }

    /// 获取交易对的深度数据
    pub fn get_depth(&self, symbol: &str) -> Option<DepthData> {
        let cache = lock_or_internal(&self.depth_cache, "get_depth")
            .expect("depth_cache lock");
        cache.get(symbol).cloned()
    }

    /// 更新持仓 (ADR D7: 加权平均法计算开仓均价)
    ///
    /// 同向加仓: 新均价 = (原均价*原数量 + 成交价*成交数量) / (原数量+成交数量)
    /// 反向平仓: 减少数量，均价不变，盈亏计入 realized_pnl
    /// 反向翻转: 新方向，新均价
    pub fn calculate_new_avg_price(
        old_avg: f64,
        old_qty: f64,
        trade_price: f64,
        trade_qty: f64,
    ) -> f64 {
        let new_qty = old_qty + trade_qty;
        if new_qty.abs() < 1e-12 {
            return trade_price;
        }
        (old_avg * old_qty + trade_price * trade_qty) / new_qty
    }

    /// 计算平仓盈亏
    ///
    /// 买入平仓: pnl = (close_price - entry_price) * quantity
    /// 卖出平仓: pnl = (entry_price - close_price) * quantity
    pub fn calculate_realized_pnl(
        entry_price: f64,
        close_price: f64,
        quantity: f64,
        is_long: bool,
    ) -> f64 {
        if is_long {
            (close_price - entry_price) * quantity
        } else {
            (entry_price - close_price) * quantity
        }
    }

    /// 定时检查过期委托
    pub async fn check_expired_orders(&self) -> Result<u64, AppError> {
        use crate::db::order::Entity as OrderEntity;

        // 1. 查询 status='pending' AND expire_at IS NOT NULL AND expire_at < NOW() 的订单
        let expired_orders = OrderEntity::find()
            .filter(order_model::Column::Status.eq(OrderStatus::Pending))
            .filter(order_model::Column::ExpireAt.is_not_null())
            .filter(order_model::Column::ExpireAt.lt(chrono::Utc::now()))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if expired_orders.is_empty() {
            return Ok(0);
        }

        let count = expired_orders.len() as u64;
        let order_ids: Vec<uuid::Uuid> = expired_orders.iter().map(|o| o.id).collect();
        let now = chrono::Utc::now();

        // 批量更新所有过期订单状态
        for order in expired_orders {
            let mut active_model: order_model::ActiveModel = order.into();
            active_model.status = sea_orm::Set(OrderStatus::Expired);
            active_model.updated_at = sea_orm::Set(now);

            if let Err(e) = active_model.update(self.db.as_ref()).await {
                error!("Failed to expire order: {:?}", e);
            }
        }

        // 3. 批量从内存订单簿移除
        for order_id in order_ids {
            self.remove_from_book(order_id);
        }

        info!("Expired {} orders", count);
        Ok(count)
    }

    /// 后台批量写入成交记录
    pub fn spawn_trade_writer(self: &Arc<Self>, mut rx: mpsc::Receiver<TradeRecord>) {
        let db = self.db.clone();
        // P1-2.1: caller already holds `self: &Arc<MatchingEngine>`, clone the Arc
        // so the spawned task can call iceberg::append_next_child which expects Arc.
        let engine = self.clone();
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(64);
            loop {
                // 收集一批成交记录
                tokio::select! {
                    Some(record) = rx.recv() => {
                        buffer.push(record);
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // 超时，写入当前批次
                    }
                }

                if !buffer.is_empty() {
                    if let Err(e) = Self::flush_trades(&db, &engine, &buffer).await {
                        error!("Failed to flush trade records: {:?}", e);
                    }
                    buffer.clear();
                }
            }
        });
    }

    async fn flush_trades(
        db: &Arc<DatabaseConnection>,
        engine: &Arc<MatchingEngine>,
        records: &[TradeRecord],
    ) -> Result<(), sea_orm::DbErr> {
        use crate::db::order::trades::ActiveModel as TradeActiveModel;
        use crate::db::order::trades::Entity as TradeEntity;

        if records.is_empty() {
            return Ok(());
        }

        info!("Flushing {} trade records to DB", records.len());

        // 按 order_id 分组聚合成交
        let mut order_fills: std::collections::HashMap<uuid::Uuid, (f64, f64, f64)> =
            std::collections::HashMap::new();
        for r in records {
            let entry = order_fills.entry(r.order_id).or_insert((0.0, 0.0, 0.0));
            entry.0 += r.quantity;
            entry.1 += r.quantity * r.price;
            entry.2 += r.fee;
        }

        // 1. 批量 INSERT trades 表
        let trade_models: Vec<TradeActiveModel> = records
            .iter()
            .map(|r| TradeActiveModel {
                id: sea_orm::Set(uuid::Uuid::new_v4()),
                order_id: sea_orm::Set(r.order_id),
                user_id: sea_orm::Set(r.user_id),
                symbol: sea_orm::Set(r.symbol.clone()),
                side: sea_orm::Set(r.side.clone()),
                price: sea_orm::Set(r.price),
                quantity: sea_orm::Set(r.quantity),
                fee: sea_orm::Set(r.fee),
                is_maker: sea_orm::Set(r.is_maker),
                created_at: sea_orm::Set(chrono::Utc::now()),
            })
            .collect();

        TradeEntity::insert_many(trade_models)
            .exec(db.as_ref())
            .await?;

        // 2. 批量 UPDATE orders（聚合更新，避免 O(N) 查询）
        // 注意: is_in 对 Uuid 类型有兼容性问题，改用内存过滤
        let order_ids_set: std::collections::HashSet<uuid::Uuid> =
            order_fills.keys().copied().collect();
        let orders = order_model::Entity::find()
            .filter(order_model::Column::Status.eq(OrderStatus::Pending))
            .all(db.as_ref())
            .await?
            .into_iter()
            .filter(|o| order_ids_set.contains(&o.id))
            .collect::<Vec<_>>();

            for order in orders {
                let (filled_qty, total_cost, total_fee) =
                    match order_fills.get(&order.id) {
                        Some(v) => *v,
                        None => continue,
                    };

                let new_filled = order.filled_quantity + filled_qty;
                let is_fully_filled = new_filled >= order.quantity - 1e-12;
                let avg_price = total_cost / filled_qty;

                let mut active_model: order_model::ActiveModel = order.clone().into();
                active_model.filled_quantity = sea_orm::Set(new_filled);
                active_model.avg_fill_price = sea_orm::Set(Some(avg_price));
                active_model.fee = sea_orm::Set(order.fee + total_fee);
                active_model.status = sea_orm::Set(if is_fully_filled {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartialFilled
                });
                active_model.updated_at = sea_orm::Set(chrono::Utc::now());
                if is_fully_filled {
                    active_model.filled_at = sea_orm::Set(Some(chrono::Utc::now()));
                }

                active_model.update(db.as_ref()).await?;

                // P1-2.1: Iceberg child fill triggers next slice replenishment
                if order.advanced_type.as_deref()
                    == Some(crate::services::iceberg::advanced_type::ICEBERG_CHILD)
                {
                    let result = crate::services::iceberg::append_next_child(
                        db,
                        engine,
                        order.id,
                        filled_qty,
                    )
                    .await;
                    if let Err(e) = result {
                        error!("Iceberg replenish failed for child {}: {:?}", order.id, e);
                    }
                }

                // P1-2.2: Bracket parent fully filled → record bracket_link
                if is_fully_filled
                    && order.advanced_type.as_deref()
                        == Some(crate::services::bracket::advanced_type::BRACKET)
                {
                    // Extract SL/TP from parent's advanced_params
                    if let Some(params_val) = order.advanced_params.as_ref() {
                        if let Ok(params) = serde_json::from_value::<
                            crate::models::bracket_params::BracketParams,
                        >(params_val.clone())
                        {
                            let result = crate::services::bracket::record_parent_filled(
                                db,
                                crate::services::bracket::RecordParentFilledInput {
                                    parent_id: order.id,
                                    filled_quantity: filled_qty,
                                    sl_price: params.stop_loss_price,
                                    tp_price: params.take_profit_price,
                                    symbol: order.symbol.clone(),
                                    side: match order.side {
                                        crate::db::order::OrderSide::Buy => "buy".to_string(),
                                        crate::db::order::OrderSide::Sell => "sell".to_string(),
                                    },
                                    user_id: order.user_id,
                                },
                            )
                            .await;
                            if let Err(e) = result {
                                error!(
                                    "Bracket record_parent_filled failed for {}: {:?}",
                                    order.id, e
                                );
                            }
                        } else {
                            error!(
                                "Bracket parent {} has invalid advanced_params",
                                order.id
                            );
                        }
                    }
                }
            }

        Ok(())
    }
}

// ─── 定时任务 ───────────────────────────────────────────────────

/// 启动撮合引擎定时任务
pub async fn start_matching_tasks(engine: Arc<MatchingEngine>) {
    // 过期委托检查: 每 60s
    let expired_engine = engine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = expired_engine.check_expired_orders().await {
                error!("Expired order check failed: {:?}", e);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_new_avg_price_same_direction() {
        // 买入 0.1 BTC @ 49000 → avg = 49000
        let avg = MatchingEngine::calculate_new_avg_price(0.0, 0.0, 49000.0, 0.1);
        assert!((avg - 49000.0).abs() < 1e-6);

        // 买入 0.1 BTC @ 51000 → avg = (49000*0.1 + 51000*0.1) / 0.2 = 50000
        let avg = MatchingEngine::calculate_new_avg_price(49000.0, 0.1, 51000.0, 0.1);
        assert!((avg - 50000.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_realized_pnl_long() {
        // 买入持仓均价 50000, 卖出价 52000, 数量 0.1
        let pnl = MatchingEngine::calculate_realized_pnl(50000.0, 52000.0, 0.1, true);
        assert!((pnl - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_realized_pnl_short() {
        // 卖出持仓均价 50000, 买入平仓 48000, 数量 0.1
        let pnl = MatchingEngine::calculate_realized_pnl(50000.0, 48000.0, 0.1, false);
        assert!((pnl - 200.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_insert_and_remove_limit_order() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let engine = MatchingEngine::new(db, 0, 0.001);

        let order_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        let entry = OrderEntry {
            order_id,
            user_id,
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            price: Some(50000.0),
            remaining_quantity: 1.0,
            created_at: now,
        };

        engine.insert_limit_order(entry);

        // Verify it's in the book
        {
            let books = engine.order_books.lock().unwrap();
            let book = books.get("BTCUSDT").unwrap();
            assert_eq!(book.bids.len(), 1);
        }

        // Remove it
        engine.remove_from_book(order_id);

        // Verify it's gone
        {
            let books = engine.order_books.lock().unwrap();
            let book = books.get("BTCUSDT").unwrap();
            assert!(book.bids.is_empty());
        }
    }

    #[tokio::test]
    async fn test_order_book_bids_descending() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let engine = MatchingEngine::new(db, 0, 0.001);
        let user_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        // Insert bids at different prices
        for price in [50000.0, 49000.0, 51000.0] {
            engine.insert_limit_order(OrderEntry {
                order_id: uuid::Uuid::new_v4(),
                user_id,
                symbol: "BTCUSDT".to_string(),
                side: OrderSide::Buy,
                price: Some(price),
                remaining_quantity: 1.0,
                created_at: now,
            });
        }

        let books = engine.order_books.lock().unwrap();
        let book = books.get("BTCUSDT").unwrap();
        // BTreeMap with Reverse: highest price first
        let prices: Vec<f64> = book.bids.keys().map(|k| k.0.0).collect();
        assert_eq!(prices, vec![51000.0, 50000.0, 49000.0]);
    }

    #[tokio::test]
    async fn test_order_book_asks_ascending() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let engine = MatchingEngine::new(db, 0, 0.001);
        let user_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        for price in [50000.0, 49000.0, 51000.0] {
            engine.insert_limit_order(OrderEntry {
                order_id: uuid::Uuid::new_v4(),
                user_id,
                symbol: "BTCUSDT".to_string(),
                side: OrderSide::Sell,
                price: Some(price),
                remaining_quantity: 1.0,
                created_at: now,
            });
        }

        let books = engine.order_books.lock().unwrap();
        let book = books.get("BTCUSDT").unwrap();
        let prices: Vec<f64> = book.asks.keys().map(|k| k.0).collect();
        assert_eq!(prices, vec![49000.0, 50000.0, 51000.0]);
    }

    #[tokio::test]
    async fn test_update_and_get_depth() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let engine = MatchingEngine::new(db, 0, 0.001);

        let depth = DepthData {
            bids: vec![DepthLevel {
                price: 50000.0,
                quantity: 1.0,
            }],
            asks: vec![DepthLevel {
                price: 50100.0,
                quantity: 0.5,
            }],
            timestamp: 1234567890,
        };

        engine.update_depth("BTCUSDT", depth);
        let retrieved = engine.get_depth("BTCUSDT").unwrap();
        assert_eq!(retrieved.bids.len(), 1);
        assert_eq!(retrieved.asks.len(), 1);
    }
}
