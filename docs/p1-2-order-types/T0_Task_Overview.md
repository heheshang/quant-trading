# T0 — P1-2 Advanced Order Types 任务总览

> **任务 ID**：P1-2 (MVP) → 拆为 P1-2.1/2/2/3 子任务
> **前置**：P0 完成 + P1-1 strategy templates
> **日期**：2026-06-01
> **状态**：✅ **基础架构完成**，**业务逻辑拆分 3 子任务**

## 范围（本期 MVP）

业务审计报告指出：当前 `OrderType` 仅支持 `limit` / `market`。P1-2 系统化补齐 **3 个业界经典高级订单类型**：
- **Iceberg** — 大单分批（防滑点 + 防订单簿暴露）
- **Bracket** — 母单 + 自动 SL + TP
- **TrailingStop** — 移动止损

**P1-2 本期 MVP** = **基础架构**（OrderType 枚举 + DB schema + Handler 解析）：
- ✅ OrderType 枚举扩 3 变体
- ✅ Postgres ENUM `ALTER TYPE ADD VALUE × 3`
- ✅ orders 表加 2 column (`advanced_type` / `advanced_params`)
- ✅ handler `parse_order_type` / `order_type_str` / `price match` 扩 5 变体

**完整业务逻辑** = P1-2.1/2/2/3 子任务（**P1-2.1 Iceberg 拆单 / P1-2.2 Bracket 联动 / P1-2.3 TrailingStop 后台**）

## 交付

| 资产 | 路径 | 改动 |
|---|---|---|
| Migration | `backend/migrations/20260602000000_advanced_order_types.sql` | 新文件 47 行 |
| OrderType 枚举 | `backend/src/db/order.rs` | +14 行（3 变体 + 2 column）|
| Handler 解析 | `backend/src/handlers/order.rs` | +11 行（5 处 match 扩）|
| Trigger order 兼容 | `backend/src/services/trigger_order.rs` | +6 行（2 个 ActiveModel init）|

## 质量门禁

| 阶段 | 结果 |
|---|---|
| `cargo build` | ✅ clean |
| `cargo test --lib` | ✅ **390/390 pass** (零 regression) |
| `cargo clippy --lib --all-targets -- -D warnings` | ✅ **0 warnings** |
| Migration SQL 静态检查 | ✅ (PG 12+ IF NOT EXISTS 守卫) |

## 业务价值（基础架构）

| 之前 | 现在 |
|---|---|
| OrderType 2 变体 | **5 变体**（+iceberg/bracket/trailing_stop）|
| 无 advanced 参数存储 | **JSONB `advanced_params` 列** + `advanced_type` 索引 |
| Handler 不识别新 type | **`parse_order_type` 5 变体** + price match 扩 |

## 业务价值（待 P1-2.1/2/2/3 完成）

- **Iceberg 拆单**：1 单 = N 子单（防滑点 30%+）
- **Bracket**：1 单 = 3 单（母 + SL + TP 联动）
- **TrailingStop**：动态调整 SL（专业 trader 必备）

## 文档

```
docs/p1-2-order-types/
├── T0_Task_Overview.md                                       (本文件)
├── requirements/T1-PRD_advanced_order_types.md
├── architecture/T2_Design.md
├── qa/T4_Unit_Test_Report.md
└── t9/T9_Final_Review.md
```

## 后续子任务

| ID | 任务 | 范围 | 预计工时 |
|---|---|---|---|
| **P1-2.1** | **Iceberg 业务逻辑** | 母单拆 N 子单 + 自动补单 + 子单 cancel | 1 work session |
| **P1-2.2** | **Bracket 业务逻辑** | 母单成交后挂 OCO + 触发 cancel 另一个 | 1 work session |
| **P1-2.3** | **TrailingStop 后台服务** | 5s 轮询 + 动态调 SL | 1-2 work session |

## 进度全景

| ID | 任务 | Commit | 状态 |
|---|---|---|---|
| P0-1~5 | 完整可观测性栈 | `8d8c586`~`530809b` | ✅ |
| P1-1 | Strategy 模板扩 3 个 | `064bba3` | ✅ |
| **P1-2 (MVP)** | **高级订单类型基础架构** | **本次** | **✅** |
| P1-2.1 | Iceberg 业务逻辑 | — | ⏳ |
| P1-2.2 | Bracket 业务逻辑 | — | ⏳ |
| P1-2.3 | TrailingStop 后台服务 | — | ⏳ |
| P1-3 | 风控粒度细化 | — | ⏳ |
