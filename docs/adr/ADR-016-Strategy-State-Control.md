# ADR-016: 策略状态控制 — 断线暂停与恢复

**ID**: ADR-016
**日期**: 2026-05-21
**状态**: Proposed
**关联**: ADR-014, ADR-013, PRD-Phase4-Risk-Management

---

## 背景

ADR-014 定义了 `strategy_paused_by_disconnect` 原子标志，但**未实现**：
- 后台监控任务（定期检查断线状态）
- 暂停后自动调用 `RiskManager::pause()`
- 重连稳定后自动调用 `RiskManager::resume()`
- 暂停期间阻止新订单发出

---

## 技术方案

### 位置

`backend/src/services/strategy_state_manager.rs`（新建）

### 核心职责

```
DisconnectionMonitor（后台任务，每 5s 运行）
    ↓
WsHub.is_binance_connected() 查询
    ↓
判断：connected / disconnected
    ├─ disconnected > 30s → 调用 RiskManager.pause() → 标记 strategy_paused = true
    └─ connected 持续稳定 10s → 调用 RiskManager.resume() → 标记 strategy_paused = false
```

### Struct 设计

```rust
pub struct StrategyStateManager {
    ws_hub: Arc<WsHub>,
    risk_manager: Arc<RiskManager>,
    redis: Arc<RedisPool>,
    reconnect_stable_threshold_secs: u64,  // 默认 10s
}

impl StrategyStateManager {
    /// 启动后台监控任务
    pub fn start_monitor(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut disconnect_start: Option<u64> = None;
            let mut reconnect_stable_start: Option<u64> = None;
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let connected = self.ws_hub.is_binance_connected().await;
                if connected {
                    // 重连稳定计时
                    if reconnect_stable_start.is_none() {
                        reconnect_stable_start = Some(util::now_secs());
                    }
                    if util::now_secs() - reconnect_stable_start.unwrap() >= self.reconnect_stable_threshold_secs {
                        if self.ws_hub.is_strategy_paused().await {
                            self.resume_strategies().await;
                        }
                        reconnect_stable_start = None;
                    }
                } else {
                    reconnect_stable_start = None;
                    let elapsed = self.ws_hub.disconnect_elapsed_secs().await;
                    if elapsed >= self.ws_hub.disconnect_threshold() {
                        if !self.ws_hub.is_strategy_paused().await {
                            self.pause_strategies(elapsed).await;
                        }
                    }
                }
            }
        })
    }
}
```

### 订单拦截

所有订单创建前检查 `ws_hub.is_strategy_paused()`：

```rust
// handlers/order.rs — 新增检查
pub async fn create_order(...) -> Result<Json<ApiResponse<...>>, AppError> {
    if ws_hub.is_strategy_paused().await {
        return Err(AppError::RiskViolation(
            ERR_PAUSED,
            "策略已暂停（断线保护），禁止开仓"
        ));
    }
    // ... 正常流程
}
```

### WsHub 新增方法

```rust
impl WsHub {
    pub async fn is_strategy_paused(&self) -> bool;
    pub async fn disconnect_elapsed_secs(&self) -> u64;
    pub async fn disconnect_threshold(&self) -> u64;
}
```

---

## 状态机

```
[Active] --断线30s--> [PausedByDisconnect]
    ↑                      ↓
 [Resume]          <--重连稳定10s--
```

| 状态 | 说明 |
|------|------|
| Active | 正常交易 |
| PausedByDisconnect | 断线触发，自动暂停 |
| PausedByAdmin | 管理员手动暂停（RiskManager.pause） |

---

## 交付物

1. `backend/src/services/strategy_state_manager.rs` — 状态管理器
2. WsHub 新增 `is_strategy_paused()` / `disconnect_elapsed_secs()` 方法
3. `handlers/order.rs` 新增断线暂停拦截
4. 单元测试
