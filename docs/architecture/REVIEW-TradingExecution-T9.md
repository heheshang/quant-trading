# T9: 交易执行模块 — 最终评审

**评审人**: tech-lead  
**日期**: 2026-05-14  
**结论**: ## REJECTED

---

## 评审清单

| # | 质量门 | 状态 | 详情 |
|---|--------|------|------|
| 1 | cargo test 通过 | ❌ FAIL | 3 个编译错误，构建失败 |
| 2 | vitest 通过 | ❌ FAIL | 5 个测试文件失败，33/323 测试未通过 |
| 3 | P0=0, P1<=3 | ❌ FAIL | 交易执行 QA 报告不存在，无法评估 |
| 4 | API 文档完整 | ❌ FAIL | docs/api/ 无交易执行 API 文档 |
| 5 | CHANGELOG 已更新 | ❌ FAIL | CHANGELOG 无交易执行模块条目 |
| 6 | docker compose up 成功 | ❌ FAIL | 后端编译失败，Docker 构建无法通过 |

---

## 详细发现

### 1. cargo test — 编译失败 (3 errors)

**文件**: `backend/src/services/matching_engine.rs`

**错误 E0382 (borrow of moved value)**:
- `cancel_all_orders` 逻辑中 `active.clone().update(&*db).await` 后访问 `active.id`
- SeaORM `ActiveModel::update(self)` 消费 self，clone 后仍被 move
- 修复: update 前保存 `order_id`，或改用 `update` 返回值

**错误 E0433 (cannot find module `Status`)**:
- `order_model::Column::Status::is_in(...)` 语法错误
- SeaORM Column variant 不能用 `::` 调用方法
- 修复: 改为 `order_model::Column::Status.is_in(...)`

**影响**: 后端完全无法编译，0 个测试可运行

### 2. vitest — 5 文件失败 / 33 测试失败

**关键错误**: `BacktestConfigForm.vue:325`
```
TypeError: Cannot read properties of undefined (reading 'find')
const strategy = strategies.value.find((s) => s.id === form.strategy_id)
```
- `strategies.value` 为 undefined，说明 mock 数据不完整或组件未正确初始化依赖
- 影响范围: BacktestConfigForm 相关 4 个 unhandled rejection

**测试统计**: 323 tests total, 290 passed, 33 failed, 5 files failed

### 3. QA 报告 — 不存在

- `docs/qa/` 仅存在 `StrategyManagement_UI-Checklist.md`
- `kanban-archive/qa/` 仅存在 `qa_report_backtest_engine.md`
- **交易执行模块无任何 QA 走查报告**
- 父任务 T3 产出了 92 项 QA checklist (P0×60/P1×30/P2×2)，但该文件未归档到项目中

### 4. API 文档 — 不完整

`docs/api/` 现有文件:
- `backtest-api.md` (回测引擎)
- `KlineManagement_API.md` (K 线管理)

**缺失**:
- 交易执行 API 文档 (应覆盖: POST /api/v1/orders, GET /api/v1/orders, POST /api/v1/orders/:id/cancel, POST /api/v1/orders/cancel-all, GET /api/v1/trades, GET /api/v1/positions, GET /api/v1/account, GET /api/v1/symbols)

### 5. CHANGELOG — 未更新

当前 CHANGELOG.md 最新条目为 `[0.5.0] — 2026-05-13`，内容仅覆盖 K 线管理模块。

**缺失**:
- 交易执行模块所有端点、撮合引擎、持仓管理、模拟账户等条目
- 无 `[0.6.0]` 或对应版本条目

### 6. docker compose — 后端构建失败

- `docker compose config` 语法验证通过 (exit 0)
- 但 `Dockerfile.backend` 执行 `cargo build` 时会因上述 3 个编译错误失败
- 前端 `Dockerfile.frontend` 可能可构建，但依赖后端 healthcheck

---

## 代码质量补充发现

### matching_engine.rs — TODO 函数
- `on_depth_update()` (L331): 限价单撮合核心逻辑为空 TODO
- `check_expired_orders()` (L408): 过期委托检查为空 TODO
- `flush_trades()` (L445): 批量写入成交记录为空 TODO

### 上述 TODO 意味着:
- 限价单撮合不会触发 (P0 功能缺失)
- 过期委托永远不会被清理
- 成交记录不会持久化到数据库

### handlers/order.rs — 代码质量良好
- 8 个 REST handler 实现完整
- 请求/响应类型定义清晰
- 输入校验充分 (side/order_type/price/quantity)
- 单元测试覆盖 parse/serialize 逻辑

### db/order.rs — 数据模型完善
- 5 个 Entity: orders, trades, positions, paper_accounts, symbol_configs
- 状态机 `OrderStatus` 实现正确 (can_transition_to)
- 枚举序列化与 PRD 对齐

---

## 阻塞项汇总 (P0)

1. **后端编译失败** — 3 个 Rust 编译错误，阻塞所有后端功能
2. **撮合引擎核心未实现** — on_depth_update/flush_trades/check_expired_orders 均为 TODO
3. **前端测试失败** — 33 个测试未通过
4. **QA 报告缺失** — 无法评估 P0/P1 缺陷数
5. **API 文档缺失** — 交易执行 8 个端点无文档
6. **CHANGELOG 未更新** — 模块变更未记录

---

## REJECTED 原因

6 项质量门全部未通过。其中后端编译失败和撮合引擎核心逻辑为 TODO 是最严重的阻塞项，直接导致系统无法运行。必须在修复后重新提交评审。
