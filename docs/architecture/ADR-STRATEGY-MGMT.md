# ADR-STRATEGY-MGMT: 策略管理模块 Entity 重设计 & API 路由规划

| 字段 | 值 |
|------|-----|
| **ID** | ADR-STRATEGY-MGMT |
| **状态** | Draft |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |
| **影响范围** | 策略管理完整链路（handler → service → db → frontend） |
| **基于** | strategy-management-prd.md (T1产出), TECH_CHARTER |
| **前置 ADR** | ADR-001（Rust/Axum后端）, ADR-004（回测引擎顶层设计） |

---

## 背景

T1 PRD (`strategy-management-prd.md`) 定义了策略管理模块的需求：
- 8个用户故事（P0×4 / P1×4 / P2×1）
- 完整数据模型（`user_strategies` + `strategy_templates`）
- 13个API端点 + 7个错误码
- 状态机：draft / active / paused / stopped

T2 的工作是审查现有代码与 PRD 的 gap，产出：
1. **ADR**：Entity重设计评估 + API路由规划
2. **Contract-Review**：B1-B4字段对齐
3. **代码骨架**：backend/src/handlers/strategy.rs + frontend/src/api/strategies.ts

---

## 审核发现

### 发现 1: Backend Entity 缺少 symbol/timeframe（CRITICAL）

**PRD section 6.2** `user_strategies` 表定义：

| 字段 | 类型 | 约束 |
|------|------|------|
| id | UUID | PK |
| user_id | UUID | FK → users.id, NOT NULL |
| name | VARCHAR(100) | NOT NULL, (user_id,name)唯一 |
| description | TEXT | |
| template_id | UUID | FK → strategy_templates.id, NOT NULL |
| params | JSONB | NOT NULL |
| status | VARCHAR(20) | NOT NULL, DEFAULT 'draft' |
| **symbol** | VARCHAR(20) | NOT NULL（交易对，PRD必填） |
| **timeframe** | VARCHAR(10) | NOT NULL（时间周期，PRD必填） |
| created_at | TIMESTAMPTZ | NOT NULL |
| updated_at | TIMESTAMPTZ | NOT NULL |

**当前 `schemas.rs` 中 `CreateStrategyRequest`**：

```rust
pub struct CreateStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,   // ← 应该是 template_id: Uuid
    pub parameters: serde_json::Value,
    // ❌ MISSING: symbol: String
    // ❌ MISSING: timeframe: String
}
```

**当前 `schemas.rs` 中 `StrategyResponse`**：同样缺少 symbol/timeframe。

**决策：**

| 决策点 | 方案 | 理由 |
|--------|------|------|
| symbol | `CreateStrategyRequest` 增加 `symbol: String`（必填） | PRD section 4 US-SM-02 明确必填交易对 |
| timeframe | `CreateStrategyRequest` 增加 `timeframe: String`（必填） | PRD section 4 US-SM-02 明确必填时间周期 |
| template_id | `template_type: String` → `template_id: Uuid` | 模板是 UUID FK，一对一引用更精确 |
| symbol/timeframe 可更新性 | 仅创建时必填，后续不可更改 | 策略创建后改交易对/周期无意义 |

---

### 发现 2: 状态机 validation 缺失（CRITICAL）

**PRD section 5 定义了完整状态转换规则：**

| 当前状态 | 允许操作 | 目标状态 | 约束 |
|---------|---------|---------|------|
| draft | 启用 | active | — |
| draft | 编辑 | draft | 可修改参数 |
| draft | 删除 | — (已删除) | 直接删除 |
| active | 暂停 | paused | 确认弹窗 |
| active | 停止 | stopped | 确认弹窗 |
| active | 编辑 | (拒绝) | 提示先暂停 |
| active | 删除 | (拒绝) | 提示先停止 |
| paused | 启用 | active | — |
| paused | 停止 | stopped | 确认弹窗 |
| paused | 编辑 | paused | 可修改参数 |
| paused | 删除 | (拒绝) | 提示先停止 |
| stopped | 删除 | — (已删除) | 确认弹窗 |
| stopped | 启用 | (拒绝) | 不允许 |
| stopped | 编辑 | (拒绝) | 不允许 |

**当前 `strategy.rs` handler (`update_status`)**：
```rust
pub async fn update_status(...) {
    let result = strategy::update_strategy_status(&db, user.user_id, strategy_id, body.status).await?;
    // 直接透传 body.status，无校验
}
```

**当前 `schemas.rs` 中 `UpdateStatusRequest`**：
```rust
pub struct UpdateStatusRequest {
    pub status: String,  // ← 无枚举约束，任意字符串均可传入
}
```

**决策：**

1. 在 `services/strategy.rs` 层实现状态机验证（service层而非handler层）
2. `UpdateStatusRequest.status` 保持 String，但 service 层需校验合法转换
3. 错误时返回 `ERR_STRATEGY_INVALID_TRANSITION`（已定义于 PRD error codes: 42201）
4. 新增 Rust enum `StrategyStatus` 在 models 层定义所有合法值

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyStatus {
    Draft,
    Active,
    Paused,
    Stopped,
}

impl StrategyStatus {
    pub fn can_transition_to(&self, target: &StrategyStatus) -> bool {
        match (self, target) {
            (Draft, Active) => true,
            (Active, Paused) | (Active, Stopped) => true,
            (Paused, Active) | (Paused, Stopped) => true,
            _ => false,
        }
    }
}
```

---

### 发现 3: template_type vs template_id 类型混淆（MEDIUM）

**Backend**：`CreateStrategyRequest.template_type: String` — 存储模板名称（字符串）

**Frontend**：`CreateStrategyPayload.template_type: string` — 同上

**PRD**：`template_id: UUID → strategy_templates.id`

**问题**：
- 前端传入模板名称字符串，后端需要解析为 UUID 再查询
- 模板重命名时会导致已有策略引用失效
- 无法做外键约束

**决策**：
- `CreateStrategyRequest.template_id: Uuid` — 前端传入模板 UUID
- 前端 `CreateStrategyPayload.template_type` 改名为 `template_id: string`
- 后端 list_templates 返回 `TemplateInfo { id: Uuid, ... }` 供前端选择
- 废弃 `template_type` 字符串方案

---

### 发现 4: 缺少 import/export 端点实现（P1）

**PRD section 7.3** 定义了两个 P1 端点但当前 handler 未实现：

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/strategies/{id}/export` | 导出单条策略为 JSON |
| POST | `/api/v1/strategies/import` | 从 JSON 导入策略 |

**决策：** T3/T4 阶段实现。当前 T2 阶段仅规划骨架。

---

### 发现 5: 缺少 strategy_templates 完整 CRUD（P1/P2）

**PRD section 7.1** 定义了模板管理端点，当前仅实现了 `GET /api/v1/strategies/templates`（list_templates），其余均缺失：

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| GET | `/api/v1/strategy-templates/{id}` | 模板详情 | P1 |
| POST | `/api/v1/strategy-templates` | 创建自定义模板 | P1 |
| PUT | `/api/v1/strategy-templates/{id}` | 更新自定义模板 | P2 |
| DELETE | `/api/v1/strategy-templates/{id}` | 删除自定义模板 | P2 |

**决策：** T3/T4 阶段实现。当前 T2 阶段仅规划骨架。

---

### 发现 6: Frontend-Backend API 路由已对齐 ✅

| Backend Handler Route | Frontend API Call | 状态 |
|----------------------|-------------------|------|
| GET /api/v1/strategies/templates | GET /strategies/templates | ✅ |
| GET /api/v1/strategies | GET /strategies | ✅ |
| POST /api/v1/strategies | POST /strategies | ✅ |
| GET /api/v1/strategies/{id} | GET /strategies/${id} | ✅ |
| PUT /api/v1/strategies/{id} | PUT /strategies/${id} | ✅ |
| DELETE /api/v1/strategies/{id} | DELETE /strategies/${id} | ✅ |
| POST /api/v1/strategies/{id}/status | POST /strategies/${id}/status | ✅ |

---

### 发现 7: Backend Entity 权限控制已正确实现 ✅

所有 handler 均通过 `AuthenticatedUser.user_id` 过滤数据：

```rust
// list_strategies: filter by user_id ✅
let result = strategy::list_strategies(&db, user.user_id, params).await?;

// get_strategy: filter by user_id + strategy_id ✅
let result = strategy::get_strategy(&db, user.user_id, strategy_id).await?;

// delete_strategy: filter by user_id ✅
strategy::delete_strategy(&db, user.user_id, strategy_id).await?;

// update_status: filter by user_id ✅
let result = strategy::update_strategy_status(&db, user.user_id, strategy_id, body.status).await?;
```

list_templates 无用户过滤（模板是全局的）✅

---

## 完整 API 路由规划

### Phase 1（T2-T4，实现核心CRUD）

```
策略实例管理：
GET    /api/v1/strategies                      → list_strategies
POST   /api/v1/strategies                      → create_strategy
GET    /api/v1/strategies/{id}                 → get_strategy
PUT    /api/v1/strategies/{id}                 → update_strategy
DELETE /api/v1/strategies/{id}                 → delete_strategy
PATCH  /api/v1/strategies/{id}/status          → update_status

模板列表：
GET    /api/v1/strategies/templates            → list_templates
```

### Phase 2（T3-T4，模板管理 + 导入导出）

```
模板管理：
GET    /api/v1/strategy-templates/{id}         → get_template      [P1]
POST   /api/v1/strategy-templates              → create_template   [P1]
PUT    /api/v1/strategy-templates/{id}          → update_template   [P2]
DELETE /api/v1/strategy-templates/{id}          → delete_template   [P2]

导入导出：
POST   /api/v1/strategies/{id}/export          → export_strategy   [P1]
POST   /api/v1/strategies/import                → import_strategy   [P1]
```

---

## 决策汇总

| # | 决策 | 影响范围 | 优先级 |
|---|------|---------|--------|
| D1 | CreateStrategyRequest 增加 symbol, timeframe, template_id (Uuid) | schemas.rs | P0 |
| D2 | StrategyResponse 增加 symbol, timeframe | schemas.rs | P0 |
| D3 | UpdateStrategyRequest 保持现有字段（symbol/timeframe 不可改） | schemas.rs | P0 |
| D4 | service层实现状态机验证，错误返回 42201 | services/strategy.rs | P0 |
| D5 | 新增 StrategyStatus Rust enum | models/ | P0 |
| D6 | 前后端 template_type → template_id (Uuid) 重命名 | frontend+backend | P0 |
| D7 | import/export 和 template CRUD 延后到 T3/T4 | handlers | P1 |
| D8 | symbol/timeframe 列加入 user_strategies 表 | DB migration | P0 |

---

## 后续任务

| Task | 内容 |  assignee |
|------|------|-----------|
| T3 | 实现 Entity 修改（schemas.rs + DB migration） | backend-worker |
| T4 | 实现状态机验证逻辑（services/strategy.rs） | backend-worker |
| T5 | 前后端 template_id 对齐（API types + API calls） | frontend-worker |
| T6 | Contract-Review 修复确认（B1-B4字段对齐） | tech-lead（当前任务） |

---

## 参考

- PRD: `strategy-management-prd.md`（T1产出）
- Feature: `strategy-management.feature`
- TECH_CHARTER: `/home/ssk/workspace/TECH_CHARTER-quant.md`
- 前端类型: `frontend/src/types/index.ts` (`StrategyFull`, `CreateStrategyPayload`)
- 后端Schema: `backend/src/models/schemas.rs` (`CreateStrategyRequest`, `StrategyResponse`)