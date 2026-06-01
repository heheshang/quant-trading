# T1 检查清单 — Phase 4 风险控制

**项目**: 量化交易系统 - 风险控制
**日期**: 2026-05-21
**状态**: In Review

---

## T1 门禁检查

| # | 检查项 | 状态 | 备注 |
|---|--------|------|------|
| 1 | ADR-013 资金风控规则引擎 | ✅ 已存在 | 70% 实现 |
| 2 | ADR-014 系统应急风控 | ✅ 已存在 | 断线检测已实现 |
| 3 | ADR-015 ATR 追踪止损 | ✅ 新增 | 本次新建 |
| 4 | ADR-016 策略状态控制 | ✅ 新增 | 本次新建 |
| 5 | ADR-017 风控仪表盘前端 | ✅ 新增 | 本次新建 |
| 6 | 后端代码基础已检查 | ✅ | risk_manager 70% / ws_hub 断线已实现 |
| 7 | 前端 UI 架构已定义 | ✅ | RiskDashboardView 5 个子组件 |
| 8 | 断线暂停集成方案 | ✅ | StrategyStateManager 后台任务 |

---

## T1 架构决策

| ADR | 文件 | 决策 |
|-----|------|------|
| ADR-013 | services/risk_manager.rs | 已有骨架，F2 单笔亏损 stub 需修复 |
| ADR-014 | services/exchange/ws_hub.rs | `last_heartbeat` / `strategy_paused_by_disconnect` 已实现 |
| ADR-015 | services/atr_stop_loss.rs（新建） | ATR 追踪止损，P1 |
| ADR-016 | services/strategy_state_manager.rs（新建） | 断线监控后台任务 |
| ADR-017 | views/risk/RiskDashboardView.vue（新建） | 前端 UI，P2 |

---

## 剩余风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| ATR 依赖 K 线数据 | 高 | Phase 4 历史 K 线持久化（Phase 3）已部分实现 |
| 宕机看门狗需 systemd | 中 | 提供模板，用户自行配置 |
| 前端 RiskDashboard 需 Admin 权限 | 低 | JWT role 检查 |

---

## T1 → T2 下一步

1. F2 单笔亏损精确计算（修复 stub）
2. F6 断线暂停集成（StrategyStateManager）
3. F8 紧急全平增强（完善现有 stub）
4. F14 紧急全平前端按钮
5. F15 断线状态指示器

---
