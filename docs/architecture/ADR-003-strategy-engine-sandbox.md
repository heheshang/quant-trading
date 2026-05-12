# ADR-003: 策略引擎沙箱架构

| 字段 | 值 |
|------|------|
| **ID** | ADR-003 |
| **状态** | 已批准 ✅ → 细化实现方案 |
| **日期** | 2026-05-12 |
| **决策者** | Tech Lead |
| **影响范围** | 策略执行、安全、运行时隔离、可行性 |

## 背景

PRD 要求用户创建量化策略（"使用代码编辑器创建策略"），支持 Python/Pine Script 语法高亮，并在系统中执行策略逻辑。这是一个**重大架构问题**——后端采用 Rust，无法直接运行用户提交的 Python/Pine Script 代码。

核心挑战：
1. 后端是 Rust，不能直接 eval Python 代码
2. 用户策略可能包含无限循环、内存泄漏、恶意代码
3. 需要支持 ≥ 20 个策略实例并发运行
4. 每个策略需要有独立的执行环境

## 技术选项

### 选项 A: WASM 沙箱执行策略 (RLang/SDK) ✅ 选定

**设计：**
- 定义一套 **DSL (Domain-Specific Language)** 或使用 **Rust 编写的策略 SDK**
- 用户用 Rust 或受限语法编写策略 → 编译为 WASM
- WASM 运行时（wasmtime）在受限 sandbox 中执行
- 通过 import/export 接口传递行情数据和交易信号

**优势：**
- WASM sandbox 天然安全：无文件/网络/系统调用
- 性能极高，接近原生
- 内存和时间可精确限制（wasmtime 资源限制）
- 确定性执行，回测与实盘逻辑一致
- 可编译为单二进制部署

**劣势：**
- 用户学习成本高（需要写 Rust 或受限 DSL）
- 策略开发门槛高于 Python
- 非主流量化策略编写方式

### 选项 B: Python 子进程沙箱

**设计：**
- 策略代码通过子进程执行（`std::process::Command`）
- 每个策略实例启动一个 Python 子进程
- 通过 stdin/stdout pipe 传递行情/信号
- 使用 PyO3 调用 Python 解释器

**优势：**
- 用户可继续使用 Python 编写策略
- 与现有量化社区生态兼容（Pandas, TA-Lib）
- Python 学习门槛低

**劣势：**
- **安全风险**：子进程始终有系统调用能力，沙箱不彻底
- **性能开销**：进程间通信 + Python GIL 限制
- **资源管理复杂**：进程泄漏、僵尸进程、内存封控
- 回测性能差（Python 循环速度慢）

### 选项 C: 策略定义使用 JSON/YAML 配置 + 内置策略模板

**设计：**
- 用户不直接写代码，而是通过配置参数组合内置策略
- 系统内置 10-20 个标准策略模板（双均线、布林带、MACD 等）
- 用户自定义参数范围

**优势：**
- 零代码，用户门槛最低
- 完全安全，无执行任意代码风险
- 开发工作量最小

**劣势：**
- 灵活性极低，无法实现自定义逻辑
- 不符合 PRD "代码编辑器创建策略" 的要求

## 决策

采用 **混合方案（WASM 为主 + Python 子进程辅助）**：

### 第一阶段（MVP - 策略模板 + 简单 DSL）
- 内置 10+ 标准策略模板（双均线、布林带、MACD、RSI 等）
- 用户通过参数配置策略，不直接写代码
- 策略逻辑用 Rust 实现，编译到主程序中

### 第二阶段（策略沙箱 - WASM）
- 提供 Rust 策略 SDK（封装指标计算、交易信号）
- 用户编写的策略编译为 WASM
- wasmtime 运行时执行，限制内存 64MB 和指令时间 100ms/tick

### 第三阶段（高级 - Python 桥接）
- 通过 PyO3 选项性支持 Python 策略
- 运行在受限的 Linux namespace 中（seccomp + cgroup）
- 默认禁用，仅对 pro-trader 开放

## 详细实现方案 (MVP 第一阶段)

### 1. 模板引擎架构

```
backend/src/
├── templates/
│   ├── mod.rs              # 模块入口 + 全局模板注册表
│   ├── traits.rs           # StrategyTemplate trait 定义
│   ├── ma_cross.rs         # 双均线交叉
│   ├── triple_ma.rs        # 三均线
│   ├── macd.rs             # MACD 交叉
│   ├── bollinger.rs        # 布林带反转
│   ├── rsi.rs              # RSI 超买超卖
│   ├── keltner.rs          # Keltner 通道
│   ├── atr_stop.rs         # ATR 止损
│   ├── mean_reversion.rs   # 均值回归
│   ├── ichimoku.rs         # ICHIMOKU 云图
│   └── double_bollinger.rs # 双布林带
```

每个模板通过 `register_templates()` 函数注册到全局 `HashMap<&'static str, &'static dyn StrategyTemplate>`。

### 2. 核心接口定义

```rust
/// 参数定义类型
pub struct ParameterDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub param_type: ParamType,
    pub default_value: serde_json::Value,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub step: Option<f64>,
    pub required: bool,
}

pub enum ParamType {
    Integer(i32, i32),        // (min, max)
    Float(f64, f64),          // (min, max)
    Select(Vec<&'static str>),// 枚举值列表
    Boolean,
}

/// 策略模板 trait
pub trait StrategyTemplate: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn category(&self) -> TemplateCategory;
    fn params(&self) -> &'static [ParameterDef];

    /// 校验参数值是否合法
    fn validate_params(&self, params: &serde_json::Value) -> Result<(), ValidationError>;

    /// 生成参数摘要文本（用于列表展示）
    fn param_summary(&self, params: &serde_json::Value) -> String;
}
```

### 3. 数据库模型 (Sea-ORM Entity)

**表名：** `user_strategies`

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | |
| user_id | UUID | FK → users.id, NOT NULL | 策略属主 |
| name | VARCHAR(100) | NOT NULL | 策略名，同一用户下唯一 |
| template_id | VARCHAR(50) | NOT NULL | 引用内置模板 id |
| params | JSONB | NOT NULL | 用户配置的参数 |
| status | VARCHAR(20) | NOT NULL, DEFAULT 'draft' | draft/active/paused/stopped |
| symbol | VARCHAR(20) | NOT NULL | 交易对 |
| timeframe | VARCHAR(10) | NOT NULL | 时间周期 |
| created_at | TIMESTAMPTZ | NOT NULL | |
| updated_at | TIMESTAMPTZ | NOT NULL | |

**索引：**
- `(user_id, status)` — 用户策略列表查询 + 活跃策略计数
- `(user_id, name)` — 唯一性约束
- `(template_id)` — 模板统计查询

**Sea-ORM Entity 代码**：详见下方 model/strategy.rs

### 4. API 端点完整设计

| 方法 | 路径 | 说明 | 请求/响应 | 优先级 |
|------|------|------|-----------|--------|
| GET | `/api/v1/strategies/templates` | 获取所有模板列表 | → `Template[]` | P0 |
| GET | `/api/v1/strategies/templates/{id}` | 获取模板详情（含参数定义） | → `Template` | P1 |
| GET | `/api/v1/strategies` | 获取当前用户策略列表 | `?page=1&size=20` → `Paginated<Strategy>` | P0 |
| POST | `/api/v1/strategies` | 创建新策略 | `CreateStrategyRequest` → `Strategy` | P0 |
| GET | `/api/v1/strategies/{id}` | 获取单策略详情 | → `Strategy` | P1 |
| PUT | `/api/v1/strategies/{id}` | 更新策略参数/名称 | `UpdateStrategyRequest` → `Strategy` | P1 |
| PATCH | `/api/v1/strategies/{id}/status` | 切换策略状态 | `{status: "active"}` → `Strategy` | P0 |
| DELETE | `/api/v1/strategies/{id}` | 删除策略 | → `{}` | P1 |

**PATCH /status 状态机校验逻辑：**

```
当前状态 → 目标状态 → 允许？ → 后端校验
draft    → active    ✅      检查活跃数量 ≤ 5
draft    → draft     ✅      (编辑)
draft    → (删除)    ✅      
active   → paused    ✅      需要确认弹窗
active   → stopped   ✅      需要确认弹窗（不可逆）
active   → draft     ❌      必须先暂停
paused   → active    ✅      
paused   → stopped   ✅      需要确认弹窗（不可逆）
paused   → paused    ✅      (编辑)
stopped  → any       ❌      不可恢复
stopped  → (删除)    ✅      
```

### 5. 前端组件架构

```
StrategiesView.vue
├── PageHeader
│   ├── 标题: "策略管理"
│   └── "创建策略" 按钮 (ElButton)
├── StrategyTable.vue (ElTable)
│   ├── 列: 名称 / 模板类型 / 参数摘要 / 交易对 / 时间周期 / 状态 / 创建时间 / 操作
│   ├── StatusTag.vue (ElTag)
│   │   ├── draft    → 灰色 (info)
│   │   ├── active   → 绿色 (success)
│   │   ├── paused   → 橙色 (warning)
│   │   └── stopped  → 红色 (danger)
│   ├── ActionButtons
│   │   ├── draft:   [启用] [编辑] [删除]
│   │   ├── active:  [暂停] [停止]
│   │   ├── paused:  [启用] [编辑] [停止]
│   │   └── stopped: [删除]
│   └── (无数据 → EmptyState.vue)
├── CreateStrategyDialog.vue (ElDialog)
│   ├── Step 1: TemplateSelector.vue
│   │   ├── 搜索框 (ElInput)
│   │   └── 模板卡片网格 (按分类分组)
│   │       └── 每张卡片: 名称 + 描述 + 分类标签
│   ├── Step 2: ParameterForm.vue
│   │   ├── 策略名称 (ElInput)
│   │   ├── 交易对 (ElSelect 固定列表)
│   │   ├── 时间周期 (ElSelect: 1m/5m/15m/1h/4h/1d)
│   │   └── 动态参数 (根据模板 params 渲染)
│   │       ├── Integer/Float → ElSlider + ElInputNumber
│   │       ├── Select → ElSelect
│   │       └── Boolean → ElSwitch
│   └── 底部: [取消] [保存] 按钮
├── ConfirmDialog.vue (ElDialog)
│   ├── 暂停: "确定要暂停该策略吗？"
│   └── 停止: "策略停止后不可恢复，确定吗？"
└── Pagination (ElPagination)
```

### 6. Pinia Store

```typescript
// useStrategyStore
interface StrategyState {
  strategies: Strategy[]
  total: number
  page: number
  pageSize: number
  loading: boolean
  templates: Template[]
  currentTemplate: Template | null
}

// Actions
- fetchStrategies()           // GET /api/v1/strategies
- fetchTemplates()            // GET /api/v1/strategies/templates
- createStrategy(data)        // POST /api/v1/strategies
- updateStrategy(id, data)    // PUT /api/v1/strategies/{id}
- updateStatus(id, status)    // PATCH /api/v1/strategies/{id}/status
- deleteStrategy(id)          // DELETE /api/v1/strategies/{id}
```

### 7. 错误码扩展

在 `utils/response.rs` 中添加：

```rust
pub const ERR_STRATEGY_INVALID_TRANSITION: i32 = 42201;
pub const ERR_STRATEGY_NAME_DUPLICATE: i32 = 40901;      // 复用现有 ERR_CONFLICT
pub const ERR_STRATEGY_LIMIT_EXCEEDED: i32 = 42901;       // 复用现有 ERR_RATE_LIMIT
pub const ERR_STRATEGY_INVALID_TEMPLATE: i32 = 40003;
pub const ERR_STRATEGY_INVALID_PARAMS: i32 = 40004;
```

在 `utils/error.rs` 的 `AppError` 中添加变体：

```rust
#[error("Strategy error: {0}")]
Strategy(String, i32),  // 消息 + 自定义错误码
```

### 8. 事务安全性 (并发控制)

```rust
// PATCH /status 的事务逻辑伪码
async fn update_status(tx, strategy_id, user_id, target_status) -> Result {
    // 1. SELECT ... FOR UPDATE 锁行
    let strategy = strategy::Entity::find_by_id(strategy_id)
        .filter(strategy::Column::UserId.eq(user_id))
        .lock_exclusive()           // FOR UPDATE
        .one(&tx)
        .await?;

    // 2. 校验状态转换合法性
    validate_transition(strategy.status, target_status)?;

    // 3. 如果目标是 active，统计当前 active 数量
    if target_status == StrategyStatus::Active {
        let active_count = strategy::Entity::find()
            .filter(strategy::Column::UserId.eq(user_id))
            .filter(strategy::Column::Status.eq(StrategyStatus::Active))
            .count(&tx)
            .await?;
        if active_count >= 5 {
            return Err(AppError::Strategy("超过活跃策略上限(5)".into(), 42901));
        }
    }

    // 4. 更新
    let mut model: strategy::ActiveModel = strategy.into();
    model.status = Set(target_status);
    model.updated_at = Set(chrono::Utc::now());
    model.update(&tx).await?;

    Ok(())
}
```

---

## 预期后果

**正面：**
- MVP 可快速上线（不需要解决沙箱问题）
- WASM 提供了最强的安全隔离
- 策略执行确定性高，回测结果可信
- Rust SDK 性能接近原生

**负面：**
- 需要自建策略 SDK 和模板库
- WASM 编译链增加了用户使用复杂度
- MVP 阶段限制了自定义策略能力
- 需要开发策略编写/调试的 IDE 辅助功能

## 风险缓解

| 风险 | 缓解措施 |
|------|----------|
| 用户无法写自定义策略 | 内置模板 + 参数配置覆盖 80% 常见策略 |
| WASM 策略开发体验差 | Web IDE 提供在线编译 + 错误提示 |
| Python 子进程安全 | seccomp BPF + cgroup 限制 CPU/内存 |
| 策略执行超时 | tokio task 超时取消 + watchdog 监控 |
| 并发状态竞争 | SELECT FOR UPDATE 行级锁 |
| 参数校验不一致 | 前后端共用参数定义 → 前端从后端获取模板定义 |

## 一致性声明

本 ADR 与以下文档一致：
- PRD-strategy-sandbox-mvp.md (2026-05-12 v1.0)
- TECH_CHARTER-quant.md (接口规范、项目结构)
- 现有后端架构 (Axum + Sea-ORM + UUID PK + JWT 鉴权)
