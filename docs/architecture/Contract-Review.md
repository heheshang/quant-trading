# Contract-Review: B1-B4 策略管理字段对齐

| 字段 | 值 |
|------|-----|
| **关联 ADR** | ADR-STRATEGY-MGMT |
| **审核范围** | frontend ↔ backend 契约字段对齐 |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |

---

## B1: CreateStrategyRequest / CreateStrategyPayload 对齐

### Backend `CreateStrategyRequest` (schemas.rs:184-189)

```rust
#[derive(Debug, Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,                    // ✅ 对齐
    pub description: Option<String>,     // ✅ 对齐
    pub template_type: String,          // ⚠️ 见下方
    pub parameters: serde_json::Value,   // ✅ 对齐 (serde_json::Value ↔ Record<string,any>)
}
```

### Frontend `CreateStrategyPayload` (types/index.ts:164-168)

```typescript
export interface CreateStrategyPayload {
  name: string                         // ✅
  template_type: string               // ⚠️ 应该是 template_id
  parameters: Record<string, any>      // ✅
  // 注意: frontend 没有 description 字段
}
```

### 对齐矩阵

| 字段 | Backend | Frontend | PRD要求 | 状态 | 修复方案 |
|------|---------|----------|---------|------|----------|
| name | `String` | `string` | 必填 | ✅ OK | — |
| description | `Option<String>` | MISSING | 选填 | ⚠️ | Frontend 需增加 `description?: string` |
| template_type | `String` | `string` | 应为 UUID | ⚠️ | 改名为 `template_id: string` (UUID) |
| parameters | `serde_json::Value` | `Record<string,any>` | JSONB | ✅ | — |
| **symbol** | MISSING | MISSING | **必填** | ❌ | 参见 D1 |
| **timeframe** | MISSING | MISSING | **必填** | ❌ | 参见 D1 |

---

## B2: StrategyResponse / StrategyFull 对齐

### Backend `StrategyResponse` (schemas.rs:171-181)

```rust
pub struct StrategyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub template_type: String,       // ⚠️ 应该是 template_id: Uuid
    pub parameters: serde_json::Value,
    pub status: String,              // ⚠️ 无枚举约束
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // MISSING: symbol, timeframe
}
```

### Frontend `StrategyFull` (types/index.ts:129-139)

```typescript
export interface StrategyFull {
  id: string
  user_id: string
  name: string
  description: string
  template_type: string          // ⚠️ 与后端相同，应为 template_id
  parameters: Record<string, any>
  status: 'active'|'paused'|'stopped'|'draft'  // ⚠️ 后端无枚举
  created_at: string
  updated_at: string
}
```

### 对齐矩阵

| 字段 | Backend | Frontend | 状态 |
|------|---------|----------|------|
| id | `Uuid` | `string` | ✅ JSON序列化一致 |
| user_id | `Uuid` | `string` | ✅ |
| name | `String` | `string` | ✅ |
| description | `String` | `string` | ✅ |
| template_type | `String` | `string` | ⚠️ 双方都用字符串，应为 UUID |
| parameters | `serde_json::Value` | `Record<string,any>` | ✅ |
| status | `String`（无枚举） | `'active'\|'paused'\|'stopped'\|'draft'` | ⚠️ 后端无约束 |
| created_at | `chrono::DateTime<Utc>` | `string` | ✅ ISO8601 |
| updated_at | `chrono::DateTime<Utc>` | `string` | ✅ ISO8601 |
| **symbol** | MISSING | MISSING | ❌ |
| **timeframe** | MISSING | MISSING | ❌ |

---

## B3: StrategyTemplate / TemplateInfo 对齐

### Backend `TemplateInfo` (schemas.rs:204-211)

```rust
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_parameters: serde_json::Value,
    pub parameter_schema: Vec<ParameterDef>,
}

pub struct ParameterDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,   // ⚠️ 字段名是 param_type
    pub label: String,
    pub description: String,
    pub default: serde_json::Value,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub options: Option<Vec<String>>,
}
```

### Frontend `StrategyTemplate` (types/index.ts:154-161)

```typescript
export interface StrategyTemplate {
  id: string
  name: string
  description: string
  category: string
  default_parameters: Record<string, any>
  parameter_schema: StrategyParamDef[]
}

export interface StrategyParamDef {
  name: string
  label: string
  type: 'integer'|'float'|'select'|'boolean'|'string'  // ⚠️ type 而非 param_type
  default?: any
  min?: number
  max?: number
  options?: string[]
  description?: string
}
```

### 对齐矩阵

| 字段 | Backend | Frontend | 状态 | 说明 |
|------|---------|----------|------|------|
| id | `String` | `string` | ✅ |  |
| name | `String` | `string` | ✅ | |
| description | `String` | `string` | ✅ | |
| category | `String` | `string` | ✅ | |
| default_parameters | `serde_json::Value` | `Record<string,any>` | ✅ | |
| parameter_schema[].name | `String` | `string` | ✅ | |
| parameter_schema[].label | `String` | `string` | ✅ | |
| parameter_schema[].type | `param_type` (serde rename) | `type` | ⚠️ | 见下方说明 |
| parameter_schema[].description | `String` | `string` | ✅ | |
| parameter_schema[].default | `serde_json::Value` | `any` | ✅ | |
| parameter_schema[].min | `Option<serde_json::Value>` | `number` | ✅ | |
| parameter_schema[].max | `Option<serde_json::Value>` | `number` | ✅ | |
| parameter_schema[].options | `Option<Vec<String>>` | `string[]` | ✅ | |

**注意：** `#[serde(rename = "type")]` 使 Rust 的 `param_type` 序列化为 JSON 的 `"type"`。因此前端收到的 JSON 字段名是 `"type"`，与 TypeScript 接口的 `type` 字段对齐 ✅。

---

## B4: UpdateStatusRequest / toggleStrategy 对齐

### Backend `UpdateStatusRequest` (schemas.rs:199-201)

```rust
pub struct UpdateStatusRequest {
    pub status: String,  // ← 任意字符串，无枚举约束
}
```

### Frontend `toggleStrategy` (api/strategies.ts:33-35)

```typescript
export function toggleStrategy(id: string, status: 'active' | 'paused'): Promise<StrategyFull> {
  return client.post(`/strategies/${id}/status`, { status })
}
```

### 对齐矩阵

| 字段 | Backend | Frontend | 状态 |
|------|---------|----------|------|
| status | `String`（无约束） | `'active'\|'paused'` | ⚠️ 后端接受任意值 |

**问题：** 前端只传 `active` 或 `paused`，但后端 `UpdateStatusRequest.status` 接受任意字符串。这是有意的——`stopped` 状态只能通过"停止"按钮触发，不能通过 toggle。但 service 层需要实现状态机校验，拒绝非法转换。

**修复：** service层已在 ADR-STRATEGY-MGMT D4 中规划验证逻辑。

---

## 修复优先级

| 优先级 | 问题 | 修复位置 | 说明 |
|--------|------|----------|------|
| P0 | CreateStrategyRequest 缺少 symbol, timeframe, template_id | schemas.rs | ADR-STRATEGY-MGMT D1 |
| P0 | StrategyResponse 缺少 symbol, timeframe | schemas.rs | ADR-STRATEGY-MGMT D2 |
| P0 | 状态机校验缺失 | services/strategy.rs | ADR-STRATEGY-MGMT D4 |
| P1 | Frontend 缺少 description 字段 | types/index.ts | B1 对齐 |
| P1 | Frontend template_type → template_id 重命名 | types/index.ts + strategies.ts | B1/B2 对齐 |
| P2 | Backend template_type → template_id UUID | schemas.rs + handlers | B1/B2 对齐 |

---

## 结论

**整体评估：⚠️ 需要重大修改（Entity缺失字段）**

- 核心 CRUD 路由已对齐 ✅
- 字段类型基本匹配 ✅
- **关键缺口**：symbol/timeframe 在 PRD 中是必填字段，但 backend + frontend 均缺失 ❌
- 状态机validation缺失（当前可传任意值）❌
- template_type 应改为 template_id (UUID) ⚠️

修复后需重新跑一遍 Contract-Review。