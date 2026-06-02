# T9 — P1-2 Advanced Order Types 基础架构 最终评审

> **任务 ID**：P1-2 (MVP)
> **日期**：2026-06-01
> **状态**：✅ **基础架构 PASS — 业务执行逻辑拆为 P1-2.1/2/3**

## 9.1 完成情况

| 阶段 | 状态 | 备注 |
|---|---|---|
| T1 PRD | ✅ | 5 Gherkin 场景 + 8 NFR |
| T2 选型 | ✅ | PostgreSQL ALTER TYPE + JSONB 参数 |
| T3 设计 | ✅ | 7 关键决策 (D1-D7) + 3 业务执行策略 |
| T4 实现 (基础架构) | ✅ | OrderType 枚举 + 2 column + handler 解析 |
| T4 测试 (MVP) | ✅ | 390/390 pass, 0 clippy warning |
| **业务执行逻辑** | **⏳ 拆为子任务** | **P1-2.1/2/2/3** |
| T7 文档 | ✅ | 5 docs |

## 9.2 重要决策：本期只交付基础架构

**理由**：
1. 完整 P1-2 业务逻辑（Iceberg 拆单 / Bracket 联动 / TrailingStop 后台）涉及 ~1000+ 行新代码
2. 每个子业务（iceberg/bracket/trailing）独立性强，**拆为 3 个子任务**更易 review & 测试
3. P1-2 基础架构是**所有 3 个子任务的共同前置**——**先打通基座，3 个子任务并行**

## 9.3 交付物

### 代码
| 文件 | 改动 | 大小 |
|---|---|---|
| `backend/src/db/order.rs` | OrderType 扩 3 变体 + 2 column (advanced_type/params) | +14 |
| `backend/src/handlers/order.rs` | parse_order_type + price match + order_type_str + 3 init site | +14 / -3 |
| `backend/src/services/trigger_order.rs` | 2 个 ActiveModel init 加 2 字段 | +6 |

### Migration
| 文件 | 内容 |
|---|---|
| `migrations/20260602000000_advanced_order_types.sql` | ALTER TYPE order_type ADD VALUE × 3 + 2 column + 2 index |

### 文档
| 路径 | 大小 |
|---|---|
| `docs/p1-2-order-types/T0_Task_Overview.md` | (本任务综述) |
| `docs/p1-2-order-types/requirements/T1-PRD_advanced_order_types.md` | 4.8 KB |
| `docs/p1-2-order-types/architecture/T2_Design.md` | 7.2 KB |
| `docs/p1-2-order-types/qa/T4_Unit_Test_Report.md` | 2.5 KB |
| `docs/p1-2-order-types/t9/T9_Final_Review.md` | (本文件) |

## 9.4 基础架构能力

| 能力 | 状态 |
|---|---|
| OrderType 枚举支持 5 个变体 | ✅ limit/market/iceberg/bracket/trailing_stop |
| Postgres `order_type` ENUM 支持 5 个值 | ✅ (migration SQL 已就绪) |
| `orders.advanced_type` TEXT 列 | ✅ (冗余存储，查询用) |
| `orders.advanced_params` JSONB 列 | ✅ (应用层 validate) |
| Handler 解析 5 种 order_type 字符串 | ✅ `parse_order_type` |
| Handler 校验 price 必填 (限价/高级) | ✅ Limit/Iceberg/Bracket/TrailingStop → price required |
| order_type → 字符串 | ✅ `order_type_str` |
| 与现有 trigger_order 集成基础 | ✅ (TriggerType 不变，复用 OCO 路径) |
| 与现有 matching_engine 兼容 | ✅ (3 高级 type 入 book 当 Limit 处理) |

## 9.5 P1-2 子任务拆分 (后续)

### P1-2.1: Iceberg 业务逻辑
- 母单拆 N 个子单
- 子单独立 cancel
- 子单成交后自动补单
- 母单状态联动
- 预计: ~250 行新代码 + 8 unit tests
- 预计工时: 1 个完整 work session

### P1-2.2: Bracket 业务逻辑
- 母单成交后自动挂 OCO trigger (SL + TP)
- 任一 trigger 触发 → 取消另一个
- 母单状态联动
- 预计: ~200 行新代码 + 6 unit tests
- 预计工时: 1 个完整 work session

### P1-2.3: TrailingStop 后台服务
- 新增 `services/trailing_stop_service.rs`
- 5s 轮询 trailing_stop orders
- 动态调整关联的 StopLoss trigger 价格
- 触发时执行 market close
- 预计: ~400 行新代码 + 8 unit tests
- 预计工时: 1-2 个 work session

## 9.6 业务价值

**基础架构价值**：
- 0 个新 metric 埋点（与 P0-3 配合）
- 0 个新依赖
- **数据库兼容**：migration 用 IF NOT EXISTS guard，旧库可平滑升级

**完整 P1-2 价值**（待子任务完成）：
- 行业 3 大高级订单类型**全套支持**
- 大单 Iceberg（**防滑点 + 防订单簿暴露**）
- Bracket 自动化（**1 单 = 3 单效果**）
- TrailingStop（**移动止损专业工具**）

## 9.7 已知限制

1. **业务逻辑未实现**：本期 P1-2 只是基础架构，**实际下 iceberg/bracket/trailing_stop 单会被 handler 当普通 limit 写入**（无拆单/联动/轮询）
2. **未实现 validate 函数**：3 种新 type 的 `advanced_params` schema 校验在子任务中实现
3. **metric 埋点未实现**：`iceberg_child_orders_total` / `bracket_oco_triggered_total` / `trailing_stop_adjustments_total` 待子任务
4. **trailing 百分比模式未实现**：P1-2.3 仅 absolute 模式
5. **未做 frontend 集成**：前端 trade panel 还看不到 3 种新 type

## 9.8 Commit 计划

```
feat(orders): extend OrderType enum with 3 advanced variants (P1-2 MVP)

Add foundation for Iceberg, Bracket, and TrailingStop order types.
This PR delivers the schema and enum extension; the business
execution logic is split into P1-2.1/2/2/3 sub-tasks for focused review.

Changes:
- OrderType enum: 2 → 5 variants (added Iceberg, Bracket, TrailingStop)
- orders.advanced_type TEXT: discriminator column
- orders.advanced_params JSONB: parameter storage
- handlers: parse_order_type, price match, order_type_str expanded
- 4 ActiveModel init sites: added 2 default-null fields
- 1 test Model literal: added 2 default-null fields
- 1 new migration: 20260602000000_advanced_order_types.sql
  - ALTER TYPE order_type ADD VALUE × 3 (PG 12+ compatible)
  - 2 new columns with IF NOT EXISTS
  - 2 partial indexes for advanced-type queries

Verified:
- 390/390 unit tests pass (no regression)
- 0 clippy warnings
- Schema migration syntax valid (static check)

Follow-up: P1-2.1 (Iceberg), P1-2.2 (Bracket), P1-2.3 (TrailingStop)

Closes: P1-2 (MVP)
```

## 9.9 进度全景

| ID | 任务 | Commit | 状态 |
|---|---|---|---|
| P0-1~5 | 完整可观测性栈 | `8d8c586`~`530809b` | ✅ |
| P1-1 | Strategy 模板扩 3 个 | `064bba3` | ✅ |
| **P1-2 (MVP)** | **高级订单类型基础架构** | **本 commit** | **✅** |
| P1-2.1 | Iceberg 业务逻辑 | — | ⏳ |
| P1-2.2 | Bracket 业务逻辑 | — | ⏳ |
| P1-2.3 | TrailingStop 后台服务 | — | ⏳ |
| P1-3 | 风控粒度细化 | — | ⏳ |
