# ADR-015: ATR 追踪止损

**ID**: ADR-015
**日期**: 2026-05-21
**状态**: Proposed
**关联**: ADR-013, ADR-014, PRD-Phase4-Risk-Management

---

## 状态

Proposed — P1 功能，Phase 4 风控补充

---

## 背景

ADR-013 D1 中 `stop_loss_type` / `atr_period` / `atr_multiplier` 字段预留但未实现。P1-F4 要求 ATR 追踪止损：
- 用户开仓时指定 ATR 系数
- 系统随价格有利方向自动上移止损价
- 触发时市价平仓

---

## 技术方案

### 位置

`backend/src/services/atr_stop_loss.rs`（新建）

### 数据流

```
行情更新 (WsHub)
    ↓
TickerSnapshotWriter 写入 Redis（已有）
    ↓
AtrStopLossMonitor（新建 background task）
    ↓ 订阅 Redis key: kline:{symbol}:{interval}
    ↓ 读取最新 K 线计算 ATR
    ↓ 更新 atr_stop_loss 表 current_stop
    ↓ 检测触发 → 调用 trigger_order 平仓
```

### ATR 计算（复用 indicator.rs）

```rust
// indicator.rs 已实现 atr() 函数
pub fn atr(highs: &[Decimal], lows: &[Decimal], closes: &[Decimal], period: u32) -> Decimal
```

### ATR 追踪止损核心算法

```rust
pub struct AtrStopLossService {
    db: Arc<DatabaseConnection>,
    redis: Arc<RedisPool>,
}

impl AtrStopLossService {
    /// 初始化追踪止损（开仓时调用）
    pub async fn init_stop_loss(
        &self,
        position_id: Uuid,
        entry_price: Decimal,
        atr_value: Decimal,
        multiplier: Decimal, // 系数，如 1.5
    ) -> Result<Decimal, AppError> {
        // 初始止损 = 入场价 - ATR * 系数（多头）
        let stop_price = entry_price - atr_value * multiplier;
        // 写入 atr_stop_loss 表
        self.save_stop_loss(position_id, entry_price, stop_price, atr_value).await
    }

    /// 更新追踪止损（每次行情更新时调用）
    pub async fn update_stop_loss(
        &self,
        position_id: Uuid,
        current_price: Decimal,
        current_atr: Decimal,
        multiplier: Decimal,
        position_side: &str, // "long" | "short"
    ) -> Result<Option<Decimal>, AppError> {
        let stop = self.get_current_stop(position_id).await?;
        let new_stop = match position_side {
            "long" => {
                // 多头：价格创新高时，止损价随之上移
                let potential_stop = current_price - current_atr * multiplier;
                if potential_stop > stop { potential_stop } else { stop }
            }
            "short" => {
                // 空头：价格创新低时，止损价随之下移
                let potential_stop = current_price + current_atr * multiplier;
                if potential_stop < stop { potential_stop } else { stop }
            }
        };
        if new_stop != stop {
            self.update_stop_loss(position_id, new_stop).await?;
            return Ok(Some(new_stop));
        }
        Ok(None)
    }

    /// 检测是否触发
    pub async fn check_trigger(
        &self,
        position_id: Uuid,
        current_price: Decimal,
        position_side: &str,
    ) -> Result<bool, AppError> {
        let stop = self.get_current_stop(position_id).await?;
        match position_side {
            "long" => Ok(current_price <= stop),  // 多头止损下穿
            "short" => Ok(current_price >= stop), // 空头止损上穿
        }
    }
}
```

### atr_stop_loss 表

```sql
CREATE TABLE IF NOT EXISTS atr_stop_loss (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID NOT NULL REFERENCES positions(id),
    entry_price DECIMAL(20, 8) NOT NULL,
    current_stop DECIMAL(20, 8) NOT NULL,
    atr_value DECIMAL(20, 8) NOT NULL,
    atr_period INT NOT NULL DEFAULT 14,
    multiplier DECIMAL(10, 4) NOT NULL,
    position_side VARCHAR(10) NOT NULL, -- 'long' | 'short'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_atr_position ON atr_stop_loss(position_id);
```

### ATR Stop Loss Monitor（Background Task）

```rust
pub struct AtrStopLossMonitor {
    service: Arc<AtrStopLossService>,
}

impl AtrStopLossMonitor {
    pub async fn run(&self, rx: Receiver<TickerEvent>) {
        // 每次行情更新时：
        // 1. 获取持仓对应交易对的 ATR
        // 2. 调用 update_stop_loss 更新止损价
        // 3. 调用 check_trigger 检测是否触发
        // 4. 触发则调用 trigger_order 平仓
    }
}
```

---

## API 端点

无新增外部 API —— ATR 追踪止损是内部服务，触发时直接调用平仓逻辑。

内部方法：
```rust
impl AtrStopLossService {
    pub async fn init_stop_loss(...);  // 开仓时调用
    pub async fn update_and_check(...); // 行情更新时调用
}
```

---

## 集成点

| 模块 | 集成方式 |
|------|---------|
| WsHub | TickerEvent 广播（已有 ticker_snapshot） |
| trigger_order.rs | 调用 `trigger_market_close()` 平仓 |
| risk_logs | ATR 触发时记录日志 |
| 告警服务 | 触发时推送微信通知 |

---

## 交付物

1. `backend/src/services/atr_stop_loss.rs` — ATR 追踪止损服务
2. `backend/src/db/atr_stop_loss.rs` — SeaORM Entity
3. 数据库迁移 `migrations/..._create_atr_stop_loss_table.sql`
4. 单元测试 `atr_stop_loss_test.rs`
