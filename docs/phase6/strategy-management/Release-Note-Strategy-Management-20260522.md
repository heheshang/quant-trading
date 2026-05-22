# Release Note — 策略管理模块

- Version: 1.0.0
- Date: 2026-05-22

## 1. 策略 CRUD API

| 接口 | 方法 | 说明 |
|------|------|------|
| /api/strategies | GET | 列表查询（支持分页、状态过滤） |
| /api/strategies | POST | 创建策略 |
| /api/strategies/{id} | GET | 获取单条策略详情 |
| /api/strategies/{id} | PUT | 更新策略 |
| /api/strategies/{id} | DELETE | 删除策略 |
| /api/strategies/bulk_delete | POST | 批量删除 |
| /api/strategies/bulk_update_status | POST | 批量更新状态 |
| /api/strategies/import | POST | JSON 导入单条策略 |
| /api/strategies/{id}/export | GET | JSON 导出单条策略 |

## 2. 状态机设计

- **4 种状态**: draft → active → paused → stopped
- **状态转换规则**: 通过 StrategyStateManager 统一管理
- **断线暂停**: StrategyStateManager 监听连接状态，断线自动暂停策略
- **原子回滚**: 状态机设计保证状态转换的原子性，支持快速回滚

## 3. 审核工作流 (P2-F2)

- **数据表**: strategy_reviews
- **工作流**: draft → pending_review → approved/rejected → active
- **前端**: StrategyReviewView 审核界面
- **Commits**: 7ff7908 (审核工作流), 0cbf60a (审核前端)

## 4. 前后端实现概览

### 后端
- `backend/src/handlers/strategy.rs` (196 行) — HTTP handler 层
- `backend/src/services/strategy.rs` (2320 行) — 业务逻辑层

### 前端
- StrategyReviewView — 策略审核页面

### Commits
- `b3b8756` — StrategyStateManager
- `0cbf60a` — P2-F2 策略审核前端
- `7ff7908` — P2-F2 策略审核工作流

## 5. 测试状态

```
cargo test strategy: 45 passed, 0 failed
```

## 6. 相关文档

- PRD: `docs/prd/PRD-strategy-management.md`
- PRD: `docs/prd/strategy-management-prd.md`
- Feature Spec: `docs/prd/strategy-management.feature`
