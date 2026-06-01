# PRD: Phase 4 风险控制系统

**项目**: 量化交易系统 - 风险控制模块
**日期**: 2026-05-21
**状态**: Proposed
**负责人**: @ssk
**Phase**: P4

---

## 背景

Phase 1-2 已完成核心交易执行，Phase 3 完成回测引擎。**缺失**：上线验收硬性要求的风控保护机制。

当前 `risk_manager.rs` 骨架已有单日亏损/总回撤/紧急全平逻辑，但：
- 单笔亏损为 stub（`estimated_loss_ratio = Decimal::ZERO`）
- ATR 追踪止损未实现
- 断线暂停未与 ws_hub 集成
- 宕机平仓（看门狗）未实现
- 前端风控 UI 未开发

---

## 功能列表

### P0-F2：资金风控规则引擎（增强）

| # | 功能 | 优先级 | 状态 |
|---|------|--------|------|
| F1 | 单日亏损限制（超限禁止开仓/自动平仓） | P0 | ✅ 骨架 |
| F2 | 单笔最大亏损阈值（精确计算，非 stub） | P0 | ⚠️ stub |
| F3 | 总回撤限制（超限暂停/自动平仓） | P0 | ✅ 骨架 |
| F4 | ATR 追踪止损（系数可配） | P1 | ❌ 未实现 |
| F5 | 风控规则热更新（无需重启） | P1 | ⚠️ 部分 |

### P0-F3：系统应急风控

| # | 功能 | 优先级 | 状态 |
|---|------|--------|------|
| F6 | 断线暂停（ws_hub 集成，30s 阈值） | P0 | ❌ 未集成 |
| F7 | WebSocket 重连后自动恢复策略 | P0 | ❌ 未实现 |
| F8 | 紧急一键全平（Admin） | P0 | ✅ 骨架 |
| F9 | 宕机看门狗自动平仓 | P0 | ❌ 未实现 |
| F10 | 告警推送（微信/WeChat） | P1 | ⚠️ 骨架 |

### P2-F1：风控前端 UI

| # | 功能 | 优先级 | 状态 |
|---|------|--------|------|
| F11 | 风控仪表盘（规则状态/当日亏损/回撤） | P1 | ❌ 未实现 |
| F12 | 风控规则编辑（阈值配置） | P1 | ❌ 未实现 |
| F13 | 风控日志查看 | P1 | ❌ 未实现 |
| F14 | 紧急全平按钮 | P0 | ❌ 未实现 |
| F15 | 断线状态指示器 | P0 | ❌ 未实现 |

---

## 用户故事

**作为** 量化运营者
**我希望** 系统在账户亏损达到阈值时自动保护
**以便** 极端行情下防止账户爆仓，不需要人工盯盘

**作为** 交易员
**我希望** 断线后系统自动暂停策略
**以便** 网络波动时不会失控下单

**作为** 管理员
**我希望** 一键全平所有持仓
**以便** 紧急情况下快速控制风险

---

## 验收条件（Gherkin）

### 场景 1：单日亏损触及阈值，禁止开仓

```gherkin
Given 当日累计亏损为 980 USDT
And 单日亏损阈值为 1000 USDT
When 用户发起新的买入开仓信号
Then 系统拒绝下单
And 返回错误码 "RF-001: Daily loss limit reached"
And 记录风控日志
```

### 场景 2：单日亏损超限，自动平仓保护

```gherkin
Given 当日累计亏损达到 1005 USDT
And 自动平仓开关为开启
When 系统检测到超限状态
Then 自动发送市价全平指令
And 推送微信告警
```

### 场景 3：单笔亏损超限，拒绝下单

```gherkin
Given 账户权益为 10000 USDT
And 单笔最大亏损比例为 2%（即 200 USDT）
When 用户尝试以市价单开仓（预估亏损 500 USDT）
Then 系统拒绝下单
And 返回错误码 "RF-002: Single trade loss limit exceeded"
```

### 场景 4：总回撤超限，触发全局保护

```gherkin
Given 账户历史峰值为 15000 USDT
And 当前权益为 12750 USDT（回撤 15%）
And 总回撤阈值为 10%
When 下一笔订单触发风控检查
Then 拒绝开仓
And 推送告警
```

### 场景 5：WebSocket 断连超过 30 秒，暂停策略

```gherkin
Given WebSocket 连接状态为 "connected"
And 断线检测阈值为 30 秒
When WebSocket 断连持续 31 秒
Then 系统自动将所有策略状态切换为 "paused"
And 禁止新订单发出
And 推送告警
```

### 场景 6：WebSocket 重连后恢复策略

```gherkin
Given 策略状态为 "paused"（断线触发）
And WebSocket 已重连成功
When 重连持续稳定 10 秒
Then 系统自动恢复策略状态为 "active"
And 推送通知
```

### 场景 7：紧急一键全平

```gherkin
Given 用户持有多个币种持仓
When 管理员点击"紧急全平"按钮
Then 系统立即向所有持仓发送市价平仓指令
And 不受任何风控规则限制
And 返回全平结果
```

### 场景 8：ATR 追踪止损触发

```gherkin
Given 用户持有 BTC 多单，开仓价 65000
And 设置 ATR 止损，ATR(14) = 300，系数 1.5
And 止损触发价 = 65000 - 300 * 1.5 = 64550
When 当前价格跌至 64550
Then 系统发送市价平多指令
And 记录触发价格和时间
```

---

## API 端点

| 端点 | 方法 | 路径 | 描述 |
|------|------|------|------|
| 查询风控规则 | GET | /api/v1/risk/rules | 获取当前风控配置 |
| 更新风控规则 | PUT | /api/v1/risk/rules | 修改风控参数（热更新） |
| 查询风控日志 | GET | /api/v1/risk/logs | 分页查询风控触发记录 |
| 手动触发检查 | POST | /api/v1/risk/check | 手动执行风控检查 |
| 紧急全平 | POST | /api/v1/risk/emergency-close | 紧急一键全平 |
| 暂停策略 | POST | /api/v1/risk/pause-strategies | 暂停所有策略 |
| 恢复策略 | POST | /api/v1/risk/resume-strategies | 恢复所有策略 |
| 连接状态 | GET | /api/v1/risk/connection-status | WebSocket 连接状态 |

---

## 数据模型

### risk_rules 表（已存在，部分 stub）

| 字段 | 类型 | 状态 |
|------|------|------|
| daily_loss_limit | Decimal | ✅ |
| daily_loss_auto_close | Bool | ✅ |
| single_trade_loss_ratio | Decimal | ⚠️ stub 需精确计算 |
| max_drawdown_ratio | Decimal | ✅ |
| drawdown_auto_close | Bool | ✅ |
| stop_loss_type | Enum | ⚠️ 需新增 ATR |
| atr_period | i32 | ❌ |
| atr_multiplier | Decimal | ❌ |
| is_active | Bool | ✅ |

### risk_logs 表（已存在）

### 新增：atr_stop_loss 表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| position_id | UUID | 持仓 ID |
| entry_price | Decimal | 开仓价 |
| current_stop | Decimal | 当前追踪止损价 |
| atr_value | Decimal | 当前 ATR 值 |
| updated_at | DateTime | 更新时间 |

---

## 技术方案

### 1. 单笔亏损精确计算

```rust
// 修复 stub，改为精确计算
let estimated_loss = (order_price - strategy_stop_price) * quantity;
let loss_ratio = estimated_loss / snapshot.equity;
```

### 2. ATR 追踪止损

- 从 `indicator.rs` 已有 ATR 实现复用
- 每次行情更新时计算最新 ATR
- 通过 `trigger_order.rs` 的止盈止损框架落地

### 3. 断线暂停集成

- ws_hub 记录最后心跳时间
- 新增 `DisconnectionMonitor` background task（每 5s 检查）
- 断线 30s 后自动调用 `risk_manager.pause()`
- 重连稳定 10s 后自动调用 `risk_manager.resume()`

### 4. 宕机看门狗

- 提供 systemd unit 模板或 docker restart policy
- 后端退出时写 pidfile，看门狗脚本检测到进程消失则全平
- **注意**：此功能依赖外部看门狗，非纯软件实现

### 5. 前端

- 新增 `RiskDashboardView.vue`
- 集成到现有 `DashboardView.vue` 作为风控 tab
- 紧急全平按钮需二次确认（Admin only）

---

## 依赖关系

```
Phase 4 Risk
├── P0-F2 资金风控（F1-F5）
│   └── 依赖：position_service（获取持仓）, account_snapshot
├── P0-F3 系统应急（F6-F10）
│   └── 依赖：ws_hub（断线检测）, trigger_order（平仓）
└── P2-F1 前端（F11-F15）
    └── 依赖：后端 API 完成
```

---

## 交付物

1. `services/risk_manager.rs` — 完整风控逻辑
2. `services/disconnection_monitor.rs` — 断线监控 background task
3. `handlers/risk.rs` — 风控 API（含热更新）
4. `services/trigger_order.rs` — ATR 追踪止损
5. `frontend/src/views/risk/RiskDashboardView.vue` — 前端 UI
6. `systemd/quant-trading-watchdog.service` — 看门狗模板
7. 单元测试：风控规则 覆盖率 ≥80%
