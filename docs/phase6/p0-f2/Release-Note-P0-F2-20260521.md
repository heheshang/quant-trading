# Release Note v0.9.0 — P0-F2 单日亏损限制

- Version: 0.9.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级实现 P0-F2 单日亏损限制核心计算逻辑。

## 变更内容

### get_daily_loss() 实现

从 positions 表查询 unrealized_pnl 总和，每日 UTC 00:00 重置。

### check_order() 拦截逻辑

- 当日亏损 < 1000 USDT → 允许开仓
- 当日亏损 ≥ 1000 USDT → 拒绝下单，返回 RF-001
- 超限 + auto_close=true → TODO: 调用 emergency-close

## 已知限制

- 自动平仓需 Phase 2 完善
- realized_pnl 使用 unrealized_pnl 近似

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| 后端单元测试 | 263 | ✅ |
