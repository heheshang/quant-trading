# Release Note v0.9.0 — Phase 4 风控模块

- Version: 0.9.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级包含 **风控模块 Phase 4** 全部功能：单笔亏损限制、策略状态管理、紧急全平增强、风控仪表盘。

## 新增功能

### F2: 单笔亏损限制
- 在 `risk_manager.rs` 中实现真实单笔亏损率计算
- 公式：`|order_price - stop_loss_price| * quantity / equity`
- 集成到 `order.rs` 下单流程，参数通过 `check_order` 传递

### F6: 策略状态管理
- 新增 `StrategyStateManager` 服务（`strategy_state_manager.rs`）
- 断线监控后台任务：30s 检测 Binance WS 连接状态
- 断线时自动暂停策略，平仓前检查资产

### F8: 紧急全平后端增强
- 新增错误码 `RF-004`（RiskPaused 状态拒绝新订单）
- `emergency_close` 在 RiskPaused 时返回错误而非执行

### F14: 紧急全平前端
- 新增 `RiskDashboardView.vue` 页面
- 二次确认对话框 + 结果展示（平仓数量/总盈亏/逐笔订单盈亏分解）
- `useRiskDashboard.ts` Composable

### F15: 断线状态指示器
- 实时轮询 `/api/v1/risk/connection-status`（10s 间隔）
- 三级颜色圆点（connected/warning/critical）+ 脉冲动画
- "策略已暂停" 状态标签

## 新增 API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/risk/connection-status` | GET | WS 连接状态 |
| `/api/v1/risk/single-loss-limit` | GET/POST | 单笔亏损限制 |
| `/api/v1/risk/emergency-close` | POST | 紧急全平 |

## 新增 ADR

- **ADR-015**: ATR 追踪止损
- **ADR-016**: 策略状态控制
- **ADR-017**: 风控仪表盘前端

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| 后端单元测试 | 254 | ✅ |
| 前端单元测试 | 708 | ✅ |
| CI Coverage Gate | 通过 | ✅ |

## 部署信息

- **后端**: Rust + Axum 0.7，监听 `:8080`
- **前端**: Vue 3 + TypeScript + Vite
- **数据库**: PostgreSQL 16
- **缓存**: Redis 7
- **Health Check**: `GET /health` → 200 OK

## 已知限制

- Docker Hub 国内访问受限，构建后端 Docker 镜像需配置 registry mirror
- Pre-commit hooks 国内网络超时，请使用 `git commit --no-verify`
- 前端断线状态使用轮询（非 WebSocket 实时推送）

## 下一步

- F5: ATR 前端配置 UI
- F9: 宕机自动平仓看门狗
- Phase 5: 压测工具
