# 策略管理模块 T3 设计走查报告

> 版本: T3 | 状态: Design Review | 日期: 2026-05-15
> 基于: PRD-strategy-management.md (v1.0, 665行) + StrategiesView.vue (986行)
> 驱动 US: US-SM-01 ~ US-SM-08

---

## 1. PRD vs 现有代码完成度核对

### US-SM-01: 查看策略列表 (P0)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| 页面标题"策略管理" | ✅ StrategiesView.vue L5 | 完成 | |
| 每行包含: 名称/模板类型/参数摘要/状态/创建时间 | ⚠️ 卡片式布局，缺少参数摘要 | 部分完成 | 卡片显示 symbol/timeframe/模板类型/绩效指标，但无参数摘要 |
| 分页 20 条 | ✅ L339 `pageSize = ref(20)` | 完成 | |
| 状态筛选器 | ❌ 严重问题 | **未完成** | 详见 1.1 节 |
| 搜索策略名称 | ✅ L52-60 | 完成 | |

### US-SM-02: 创建策略 (P0)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| 模板选择面板 | ✅ StrategyCreateView.vue | 完成 | |
| 默认参数填充 | ✅ | 完成 | |
| 必填参数校验 | ✅ | 完成 | |
| 策略名称重复校验 | ✅ 后端 40901 | 完成 | |
| 取消不保存 | ✅ | 完成 | |

### US-SM-03: 策略状态机 (P0)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| draft → active 启用 | ✅ | 完成 | |
| active → paused 暂停 | ✅ + 确认对话框 L533 | 完成 | |
| active → stopped 停止 | ✅ + 确认对话框 L533 | 完成 | |
| paused → active 恢复 | ✅ | 完成 | |
| stopped 无法重新启用 | ✅ stopped 状态无 start 菜单项 | 完成 | |
| 非法状态转换返回错误 | ✅ 后端 42201 | 完成 | |

### US-SM-04: 编辑策略 (P1)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| draft 可编辑 | ✅ | 完成 | |
| active 编辑被拒绝 | ⚠️ 菜单项仍显示"编辑" | **需修复** | 应隐藏或禁用 |
| paused 可编辑 | ✅ | 完成 | |
| stopped 编辑被拒绝 | ✅ stopped 状态无编辑菜单项 | 完成 | |

### US-SM-05: 删除策略 (P1)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| stopped 可删除 | ✅ L256-262 | 完成 | |
| active 删除被阻止 | ⚠️ 菜单项仍显示"删除" | **需修复** | 应提示"请先停止" |
| paused 删除被阻止 | ⚠️ 菜单项仍显示"删除" | **需修复** | 应提示"请先停止" |
| draft 直接删除 | ✅ L214-220 | 完成 | |
| 删除确认框显示策略名称 | ❌ 不显示名称 | **需修复** | P2-1 bug |

### US-SM-06: 策略导入/导出 (P1)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| 导出 JSON | ⚠️ API 存在，后端未实现 | **未完成** | P1-2 |
| 导入策略 | ⚠️ API 存在，后端未实现 | **未完成** | P1-3 |
| 导入/导出 UI 入口 | ❌ 无 | **未完成** | Header 无导入按钮 |

### US-SM-07: 策略模板浏览 (P1)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| 模板选择面板 | ✅ StrategyTemplateView.vue | 完成 | |
| 模板详情 | ✅ | 完成 | |

### US-SM-08: 权限控制 (P0)

| 验收条件 | 现有代码 | 完成度 | 备注 |
|---------|---------|--------|------|
| 用户隔离策略 | ✅ 后端校验 | 完成 | |

---

## 2. 关键差异分析

### 2.1 状态筛选器不一致 (P1-4)

**问题位置:** StrategiesView.vue L43-48

**现状:**
```vue
<el-radio-button value="archived" class="filter-pill">已归档</el-radio-button>  <!-- 值为 "archived" -->
```
**PRD 要求:** 筛选器应为 "全部 / 草稿 / 运行中 / 已暂停 / **已停止**"
**实际 UI:** 显示"已归档"但值为 "archived"
**状态机定义:** 4状态 = draft/active/paused/stopped，无"archived"

**修复方案:** 将 `archived` 改为 `stopped`，label 改为"已停止"

### 2.2 缺少导入/导出 UI 入口 (US-SM-06)

**问题位置:** StrategiesView.vue L6-19 (page-header)

**现状:** Header 只有"策略模板市场"和"创建策略"按钮
**PRD 要求 (US-SM-06):**
- 每个策略行有"导出"按钮
- 页面有"导入策略"按钮

**修复方案:**
1. Header 右侧增加"导入"按钮 (el-button type="default")
2. 策略卡片操作菜单增加"导出"菜单项（所有状态均可导出）

### 2.3 编辑/删除权限校验缺失 (US-SM-04/05)

**问题位置:** StrategiesView.vue L229-237 (active 状态操作菜单)

**现状:** active 状态菜单包含"编辑"和"删除"项
**PRD 要求:**
- active → 编辑：拒绝，提示"策略运行中，请先暂停后再编辑"
- active → 删除：拒绝，提示"请先停止策略后再删除"

**修复方案:**
```vue
<template v-else-if="strategy.status === 'active'">
  <!-- 移除 edit 和 delete 项，保留 pause/stop/clone -->
</template>
```

### 2.4 导入按钮触发方式

**现状:** importStrategies API 存在于前端但无 UI 入口
**PRD 要求 (US-SM-06):** 列表页有"导入策略"按钮
**修复方案:** Header 增加导入按钮，点击后触发 file input

---

## 3. 增量修改清单

### P0 (阻塞 - 必须修复)

| # | 模块 | 文件 | 修改内容 |
|---|------|------|---------|
| P0-1 | 筛选器 | StrategiesView.vue L48 | `archived` → `stopped`，label "已归档" → "已停止" |
| P0-2 | 导入入口 | StrategiesView.vue | Header 增加"导入策略"按钮 |
| P0-3 | 导出入口 | StrategiesView.vue | 操作菜单增加"导出"菜单项（所有状态） |

### P1 (功能阻塞)

| # | 模块 | 文件 | 修改内容 |
|---|------|------|---------|
| P1-1 | 后端 | backend/handlers/strategies.rs | 实现 bulk/status 端点 |
| P1-2 | 后端 | backend/handlers/strategies.rs | 实现 export 端点 |
| P1-3 | 后端 | backend/handlers/strategies.rs | 实现 import 端点 |
| P1-4 | 编辑校验 | StrategiesView.vue | active 状态移除"编辑"菜单项 |
| P1-5 | 删除校验 | StrategiesView.vue | active/paused 状态移除"删除"菜单项（改为提示"请先停止"） |

### P2 (功能缺陷)

| # | 模块 | 文件 | 修改内容 |
|---|------|------|---------|
| P2-1 | 删除确认 | StrategiesView.vue L286-298 | 确认框应显示策略名称 |
| P2-2 | 克隆功能 | StrategiesView.vue L511 | `clone` case 为空，需实现 |
| P2-3 | CSS动画 | StrategiesView.vue L739-748 | transition 违反"禁止动画"规则，需移除 |

### P3 (体验优化)

| # | 模块 | 文件 | 修改内容 |
|---|------|------|---------|
| P3-1 | 状态dot | StrategiesView.vue L956-961 | dot 尺寸为6px而非8px（轻微） |

---

## 4. 现有实现优点（保留）

| 特性 | 位置 | 说明 |
|------|------|------|
| 卡片式布局 | L769-782 | 比 table 更适合展示绩效指标 |
| 绩效指标行 | L179-202 | 收益率/夏普率/最大回撤/交易次数 |
| symbol/timeframe 显示 | L166-178 | 已对齐 ADR D1 |
| template_type 标签 | L170-177 | 类型映射表完整 |
| 批量选择 | L70-103 | 已实现，状态过滤逻辑正确 |
| 分页组件 | L271-282 | 20条/页，支持切换 |

---

## 5. 设计 token 核对

| Token | 规格 | 现状 |
|-------|------|------|
| --color-bg: #08090a | 背景色 | ✅ CSS 变量已定义 |
| --color-accent: #7170ff | 主色调 | ✅ 用于选中状态、按钮 |
| --color-buy: #67C23A | 买入色 | ⚠️ 代码中用 `--color-success` 而非 `--color-buy` |
| --color-sell: #F56C6C | 卖出色 | ⚠️ 代码中用 `--color-error` 而非 `--color-sell` |
| Inter + JetBrains Mono | 字体 | ✅ 已使用 |
| 无 CSS 动画 | 规范 | ❌ transition: all 0.2s ease (L790, L739-748) 违反规定 |

---

## 6. 总结

### 已完成 (Core 实现)
- ✅ 策略列表卡片式布局 + 绩效指标
- ✅ symbol/timeframe/strategy_type 字段 (ADR D1)
- ✅ 状态机核心转换 (draft/active/paused/stopped)
- ✅ 批量选择与操作
- ✅ 分页 + 搜索 + 排序
- ✅ 模板市场入口
- ✅ 筛选栏 Pills UI

### 缺失项 (T3 增量)
1. **筛选器状态值错误**: `archived` 应为 `stopped`
2. **导入/导出 UI 入口**: Header 无导入按钮，操作菜单无导出
3. **active/paused 状态操作限制**: 编辑/删除应拒绝而非显示
4. **后端 bulk/export/import 端点**: 前端 API 存在但后端未实现
5. **克隆功能**: handleActionCommand 中 clone case 为空
6. **CSS 动画**: 存在 transition 违反规范

---

*文档版本: T3 | 审核状态: Pending Review*
