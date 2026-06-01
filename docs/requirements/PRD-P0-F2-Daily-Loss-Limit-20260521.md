# PRD: P0-F2 单日亏损限制

> 版本: v1.0  
> 状态: Draft  
> Author: PM  
> Date: 2026-05-21

---

## 1. 背景

Phase 4 风控框架（ADR-010）已建立单笔亏损和总回撤限制，但**单日累计亏损**计算尚未实现。单日亏损限制是量化交易保命机制的核心——当日累计亏损达到阈值时必须强制停仓。

## 2. 目标

| 目标 | 指标 |
|------|------|
| 单日亏损计算 | 每日零点重置，按持仓浮动盈亏 + 已成交订单计算 |
| 超阈值禁止开仓 | 当日亏损 ≥ daily_loss_limit 时拒绝新订单 |
| 自动平仓（可选） | 超限后自动市价全平 |
| 实时监控 | 每笔订单成交后更新当日亏损计数 |

## 3. 功能范围

### F1: 当日累计亏损计算
- 每日 UTC 00:00 重置计数器
- 计入：已成交订单的 realized PnL
- 计入：当前持仓的 unrealized PnL（浮动）
- 排除：挂单中的冻结保证金

### F2: 超限拦截
- `daily_loss_limit` 字段，单位 USDT
- `daily_loss_auto_close` 布尔，是否超限时自动平仓
- 拦截点：order.rs 下单前风控检查

### F3: 告警触发
- 超限瞬间推送微信告警
- 日志记录风控事件

## 4. 验收标准

| AC | 标准 |
|----|------|
| AC1 | 当日亏损 980 USDT（阈值 1000），允许开仓 |
| AC2 | 当日亏损 1000+ USDT，禁止开仓，返回 RF-001 |
| AC3 | 超限 + auto_close=true，发送市价全平 |
| AC4 | UTC 00:00 计数器自动重置 |

## 5. 数据模型

```rust
// risk_rules 表新增字段（已在 Phase 4 存在）
daily_loss_limit: Decimal    // 单日亏损限制
daily_loss_auto_close: bool  // 超限是否自动平仓
daily_loss_used: Decimal     // 当日已亏损（内存，不持久化）
```

## 6. 技术方案

- 使用 `parking_lot::Mutex` 在 `risk_manager.rs` 中维护 `daily_loss_used`
- UTC 零点：使用 tokio cron 每日重置
- 订单成交回调：调用 `risk_manager.update_daily_loss(pnl)`
- 下单前检查：`risk_manager.check_daily_loss()?`
