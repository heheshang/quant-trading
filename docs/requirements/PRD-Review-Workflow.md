---
status: WIP
date: 20260601
---

# PRD-Review-Workflow-20260601 — 策略审核流程

## 1. 功能概述

Review Workflow（策略审核流程）模块为量化平台提供策略上线前的审核机制。交易员提交策略进行审核，管理员（Reviewer）审批或拒绝，审核通过后策略状态从 `draft` 变为 `active`，方可参与实盘或回测。

**目标用户**：量化交易员（提交策略）、平台管理员（审核策略）

---

## 2. 用户故事

- **US-RW1**：作为交易员，我希望提交策略进行审核，以便获得平台认证并参与实盘
- **US-RW2**：作为管理员，我希望查看所有待审核策略列表，以便批量处理审核队列
- **US-RW3**：作为管理员，我希望批准或拒绝策略，并填写拒绝原因，以便把控策略质量

---

## 3. API 端点

### 3.1 POST /api/v1/reviews/submit — 提交策略审核

**描述**：交易员提交策略进行审核

**认证**：JWT（交易员角色）

**请求体**：
```json
{
  "strategy_id": "uuid"
}
```

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "id": "uuid",
    "strategy_id": "uuid",
    "review_status": "pending",
    "submitted_at": "2026-06-01T12:00:00Z",
    "reviewed_at": null,
    "reviewed_by": null,
    "rejection_reason": null
  },
  "message": "success"
}
```

---

### 3.2 POST /api/v1/reviews/approve — 批准策略

**描述**：管理员批准策略

**认证**：JWT（管理员角色）

**请求体**：
```json
{
  "strategy_id": "uuid",
  "reason": "策略通过审核，上线实盘"
}
```

**响应** `200 OK`：返回 ReviewResponse，`review_status: "approved"`

---

### 3.3 POST /api/v1/reviews/reject — 拒绝策略

**描述**：管理员拒绝策略

**认证**：JWT（管理员角色）

**请求体**：
```json
{
  "strategy_id": "uuid",
  "reason": "策略风险过高，不符合上线标准"
}
```

**响应** `200 OK`：返回 ReviewResponse，`review_status: "rejected"`，`rejection_reason` 填充

---

### 3.4 GET /api/v1/reviews/pending — 查询待审核列表

**描述**：获取所有 pending 状态的审核记录

**认证**：JWT（管理员角色）

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": [
    {
      "id": "uuid",
      "strategy_id": "uuid",
      "review_status": "pending",
      "submitted_at": "2026-06-01T10:00:00Z"
    }
  ],
  "message": "success"
}
```

---

### 3.5 GET /api/v1/reviews/:id — 查询审核详情

**描述**：查看某条审核记录的完整信息

**认证**：JWT

---

## 4. 数据库模型

### 4.1 表：`reviews`

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| strategy_id | UUID | 关联策略ID |
| user_id | UUID | 提交人（交易员） |
| review_status | ENUM | `pending` / `approved` / `rejected` |
| submitted_at | TIMESTAMP | 提交时间 |
| reviewed_at | TIMESTAMP | 审核时间（可选） |
| reviewed_by | UUID | 审核人ID（管理员，可选） |
| rejection_reason | TEXT | 拒绝原因（可选） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### 4.2 表：`strategies`（关联字段变更）

审核流程关联 `strategies.status` 字段：

| 状态 | 说明 |
|------|------|
| draft | 草稿（未提交审核） |
| pending_review | 待审核 |
| active | 已通过审核 |
| rejected | 被拒绝 |
| paused | 暂停 |
| stopped | 停止 |

---

## 5. 业务流程

```
交易员提交策略
    │
    ├─→ strategy.status = pending_review
    ├─→ 创建 review 记录（status=pending）
    └─→ 通知管理员

管理员处理
    │
    ├─→ 批准：
    │       ├─→ review.status = approved
    │       ├─→ review.reviewed_at = now()
    │       ├─→ review.reviewed_by = admin_id
    │       └─→ strategy.status = active
    │
    └─→ 拒绝：
            ├─→ review.status = rejected
            ├─→ review.rejection_reason = reason
            ├─→ review.reviewed_at = now()
            ├─→ review.reviewed_by = admin_id
            └─→ strategy.status = rejected
```

**通知机制**：审核状态变更后通过 WebSocket 或 Email 通知交易员

---

## 6. 边界条件

- 同一策略不能重复提交审核（已有 pending 记录时拒绝）
- 只有 `draft` 或 `rejected` 状态的策略可以提交审核
- 管理员不能审核自己的策略（利益冲突检查）
- 策略被拒绝后，交易员修改代码可重新提交

---

## 7. 验收标准（Gherkin 格式）

```gherkin
Feature: 策略审核流程

  Scenario: 交易员提交策略审核
    Given 交易员有一笔 draft 状态策略
    When 交易员提交审核
    Then 策略状态变为 pending_review
    And 创建 pending 状态的 review 记录
    And 返回 review_id

  Scenario: 管理员批准策略
    Given 管理员有 pending_review 策略待审核
    When 管理员批准策略
    Then review.status = approved
    And strategy.status = active
    And reviewed_at 和 reviewed_by 被记录

  Scenario: 管理员拒绝策略
    Given 管理员有待审核策略
    When 管理员拒绝策略（reason="风险过高"）
    Then review.status = rejected
    And rejection_reason 被填充
    And strategy.status = rejected

  Scenario: 重复提交审核被拒绝
    Given 策略已有 pending 审核记录
    When 交易员再次提交审核
    Then 返回 400 错误
    And 提示"已有待审核记录"

  Scenario: 非管理员不能审核
    Given 普通交易员尝试批准策略
    When 调用 POST /reviews/approve
    Then 返回 403 Forbidden

  Scenario: 被拒绝策略可重新提交
    Given 策略被拒绝（reason="代码需修改"）
    When 交易员修改后重新提交
    Then 创建新 review 记录
    And strategy.status = pending_review
```
