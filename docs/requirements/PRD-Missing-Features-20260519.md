# 量化交易系统 — 未完成功能 PRD 总表

> **文档类型**：功能需求总览（PM 输出）
> **生成日期**：2026-05-19
> **基于审计**：代码库 vs 行业标准功能清单对比
> **后续流水线**：每个功能独立 PRD → T1-T9 流水线

---

## 一、功能缺口总览

| 功能模块 | P0（致命刚需） | P1（核心业务） | P2（增强体验） | P3（高阶增值） |
|----------|---------------|---------------|---------------|---------------|
| **阶段一：底层基建** | 签名加密、分表 | — | — | — |
| **阶段二：策略引擎+交易** | 实盘止盈止损 | KDJ、条件单、仓位实盘化、滑点补偿 | — | — |
| **阶段三：回测+模拟盘** | — | 涨跌停限制 | — | — |
| **阶段四：风控+告警** | 单日亏损限制、单笔亏损阈值、宕机平仓、断线暂停、一键全平 | 频率限制、大额拦截、告警推送 | — | — |
| **阶段五：后台+报表** | — | — | 密钥管理、策略审核、Excel导出、压测 | — |
| **阶段六：高阶功能** | — | — | — | 套利、网格马丁、AI量化 |

---

## 二、P0 致命刚需（立即开发）

### P0-F1：交易所网关 — API 签名加密

| 项目 | 内容 |
|------|------|
| 功能名称 | Binance API 签名认证（HMAC-SHA256） |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员（使用需要签名验证的私有 API） |
| 优先级 | **P0** — 无法连接需要认证的交易所 API |
| 预计工时 | 3 天 |
| 依赖 | 阶段一 REST/WebSocket 已完成 |

### 2.1.1 功能描述

后端 `binance_rest.rs` 目前仅调用公开端点（无需签名）。需要扩展支持：
- **HMAC-SHA256 签名**：`X-MBX-APIKEY` header + `signature` query param
- **时间戳校验**：防止重放攻击（timestamp offset 检查 ±5s）
- **频率限制**：Binance 签署请求限制 1200/分钟（公开行情 4800/分钟）
- **密钥存储**：API Key/Secret 需加密存储（非明文配置文件）

### 2.1.2 用户故事

**作为** 交易员
**我希望** 系统能使用我的 Binance API Key 签名请求
**以便** 访问需要认证的私有端点（账户信息、划转、挂单等）

### 2.1.3 验收条件（Gherkin）

```gherkin
Feature: Binance API 签名认证

  Scenario: 使用有效签名访问私有端点
    Given 用户已配置 API Key 和 Secret
    And 签名算法为 HMAC-SHA256
    When 发起已签名的 REST 请求
    Then 返回 200 OK
    And 响应数据解密正常

  Scenario: 签名缺失被拒绝
    Given 请求头不包含 X-MBX-APIKEY
    When 发起请求到需要认证的端点
    Then 返回 401 Unauthorized
    And 错误码为 "-2015 - Invalid API-key"

  Scenario: 签名错误被拒绝
    Given 使用错误的 Secret 生成签名
    When 发起请求
    Then 返回 401 Unauthorized
    And 错误码为 "-1022 - Invalid signature"

  Scenario: 时间戳过期被拒绝
    Given 签名中的时间戳偏移超过 5 秒
    When 发起请求
    Then 返回 401 Unauthorized
    And 错误码为 "-1021 - Timestamp for this request was not received"

  Scenario: API Key 已禁用
    Given API Key 已在 Binance 侧禁用
    When 发起已签名请求
    Then 返回 401 Unauthorized
    And 错误码为 "-2015 - Invalid API-key"

  Scenario: 频率超限被限流
    Given 在 1 分钟内发出超过 1200 个签名请求
    When 发起下一个签名请求
    Then 返回 429 Too Many Requests
    And 错误码为 "-1003 - Too many requests"
```

### 2.1.4 API 端点定义

| 端点 | 方法 | 路径 | 描述 | 认证 | 请求体 | 响应 |
|------|------|------|------|------|--------|------|
| 签名请求测试 | GET | /api/v1/exchange/ping | 测试签名连通性 | HMAC | — | `{ "serverId": "..." }` |
| 账户信息 | GET | /api/v1/exchange/account | 账户余额 | HMAC | — | AccountInfo |
| 挂单 | POST | /api/v1/exchange/order | 创建订单 | HMAC | NewOrderRequest | Order |
| 撤单 | DELETE | /api/v1/exchange/order/{orderId} | 撤销订单 | HMAC | — | Order |
| 频率限制状态 | GET | /api/v1/exchange/rate-limit | 当分钟已用额度 | HMAC | — | RateLimitStatus |

### 2.1.5 数据模型

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| api_key_id | UUID | ✅ | 密钥记录主键 |
| user_id | UUID | ✅ | 所属用户 |
| exchange | String | ✅ | 交易所标识（"binance"） |
| api_key | String | ✅ | 加密存储的 API Key |
| secret_key_encrypted | String | ✅ | AES-256-GCM 加密存储 |
| permissions | String[] | ✅ | 权限范围（read/margin/futures） |
| created_at | DateTime | ✅ | 创建时间 |
| last_used_at | DateTime | ❌ | 最后使用时间 |
| is_active | Bool | ✅ | 是否启用 |

---

### P0-F2：资金风控 — 单日/单笔亏损限制

| 项目 | 内容 |
|------|------|
| 功能名称 | 资金风控规则引擎 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员、量化运营者 |
| 优先级 | **P0** — 实盘保命机制，PRD 明确"单日亏损达到阈值强制停仓" |
| 预计工时 | 4 天 |
| 依赖 | 阶段二订单执行、阶段四风控基础框架 |

### 2.2.1 功能描述

在 `services/risk_manager.rs` 中实现独立的资金风控拦截层（ADR-010 规划但未落地）：
- **单日最大亏损限制**：当日累计浮动亏损达到阈值时，禁止开新仓位并可选择自动平仓
- **单笔最大亏损限制**：单笔订单亏损超过阈值时自动拒绝或平仓
- **总回撤限制**：账户权益从峰值回撤超过阈值时，触发全平保护
- **止损联动**：支持 ATR 百分比/固定金额两种止损模式

### 2.2.2 用户故事

**作为** 量化运营者
**我希望** 系统能自动执行资金风控规则
**以便** 在极端行情下防止账户爆仓，不需要人工盯盘

### 2.2.3 验收条件（Gherkin）

```gherkin
Feature: 资金风控规则引擎

  Scenario: 单日亏损触及阈值，禁止开仓
    Given 当日累计亏损为 980 USDT
    And 单日亏损阈值为 1000 USDT
    When 用户发起新的买入开仓信号
    Then 系统拒绝下单
    And 返回错误码 "RF-001: Daily loss limit reached"
    And 记录风控日志

  Scenario: 单日亏损超限，自动平仓保护
    Given 当日累计亏损达到 1005 USDT
    And 自动平仓开关为开启
    When 系统检测到超限状态
    Then 自动发送市价全平指令
    And 推送微信告警："当日亏损超限，已执行自动平仓"

  Scenario: 单笔亏损超限，拒绝下单
    Given 账户权益为 10000 USDT
    And 单笔最大亏损比例为 2%（即 200 USDT）
    When 用户尝试以市价单开仓 10 手（预估亏损 500 USDT）
    Then 系统拒绝下单
    And 返回错误码 "RF-002: Single trade loss limit exceeded"

  Scenario: 总回撤超限，触发全局保护
    Given 账户历史峰值为 15000 USDT
    And 当前权益为 12750 USDT（回撤 15%）
    And 总回撤阈值为 10%
    When 下一笔订单触发风控检查
    Then 拒绝开仓
    And 推送告警："总回撤 15% 超过阈值 10%，已暂停交易"

  Scenario: 止损单实盘执行
    Given 用户持有 BTC 多单，开仓价 65000
    And 设置 ATR 止损，ATR(14) = 300，系数 1.5
    And 当前价格跌至 65550（触发止损价 65550）
    When 止损条件触发
    Then 系统发送市价平多指令
    And 记录触发价格和时间

  Scenario: 风控规则修改实时生效
    Given 当前风控规则为单日亏损 1000 USDT
    When 管理员将阈值修改为 800 USDT
    Then 新规则立即生效（无需重启）
    And 记录规则变更日志
```

### 2.2.4 API 端点定义

| 端点 | 方法 | 路径 | 描述 | 认证 | 请求体 | 响应 |
|------|------|------|------|------|--------|------|
| 查询风控规则 | GET | /api/v1/risk/rules | 获取当前风控配置 | JWT | — | RiskRulesResponse |
| 更新风控规则 | PUT | /api/v1/risk/rules | 修改风控参数 | JWT+Admin | RiskRulesUpdate | RiskRulesResponse |
| 查询风控日志 | GET | /api/v1/risk/logs | 分页查询风控触发记录 | JWT | QueryParams | RiskLogListResponse |
| 手动触发检查 | POST | /api/v1/risk/check | 手动执行风控检查 | JWT+Admin | — | RiskCheckResult |
| 紧急全平 | POST | /api/v1/risk/emergency-close | 紧急一键全平 | JWT+Admin | — | EmergencyCloseResult |
| 暂停/恢复交易 | POST | /api/v1/risk/pause | 暂停所有策略执行 | JWT+Admin | RiskPauseRequest | RiskPauseResponse |

### 2.2.5 数据模型

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| id | UUID | ✅ | 主键 |
| user_id | UUID | ✅ | 所属用户 |
| daily_loss_limit | Decimal | ✅ | 单日亏损限制（USDT） |
| daily_loss_auto_close | Bool | ✅ | 超限是否自动平仓 |
| single_trade_loss_ratio | Decimal | ✅ | 单笔最大亏损比例（0.0-1.0） |
| max_drawdown_ratio | Decimal | ✅ | 最大回撤比例（0.0-1.0） |
| drawdown_auto_close | Bool | ✅ | 回撤超限是否自动平仓 |
| stop_loss_type | Enum | ✅ | "fixed" / "atr_multiplier" |
| atr_period | i32 | ❌ | ATR 周期（默认 14） |
| atr_multiplier | Decimal | ❌ | ATR 系数（默认 1.5） |
| emergency_contact | String | ❌ | 紧急联系人 |
| is_active | Bool | ✅ | 是否启用 |
| created_at | DateTime | ✅ | 创建时间 |
| updated_at | DateTime | ✅ | 更新时间 |

**风控日志表（risk_logs）**

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| id | UUID | ✅ | 主键 |
| user_id | UUID | ✅ | 触发用户 |
| rule_type | Enum | ✅ | "daily_loss" / "single_trade" / "drawdown" / "emergency" |
| triggered_at | DateTime | ✅ | 触发时间 |
| position_value | Decimal | ❌ | 触发时持仓金额 |
| account_equity | Decimal | ✅ | 账户权益 |
| threshold | Decimal | ✅ | 触发的阈值 |
| actual_value | Decimal | ✅ | 实际值 |
| action_taken | Enum | ✅ | "rejected" / "paused" / "closed_all" |
| order_id | UUID | ❌ | 关联订单（如有） |
| notification_sent | Bool | ✅ | 是否已发送通知 |

---

### P0-F3：系统应急风控

| 项目 | 内容 |
|------|------|
| 功能名称 | 系统应急风控 — 断线暂停、宕机平仓、紧急全平 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员、量化运营者 |
| 优先级 | **P0** — "服务器宕机自动平仓保护" 为上线验收硬性标准 |
| 预计工时 | 3 天 |
| 依赖 | 阶段四风控基础框架（P0-F2） |

### 2.3.1 功能描述

三大应急保护机制，对应 PRD 上线验收标准第 1/3/10 条：

1. **断线暂停**：WebSocket 断连超过 N 秒自动禁止开仓（可配置， 默认 30s）
2. **宕机平仓**：后端进程异常退出时，系统自动平仓保护（依赖外部看门狗）
3. **紧急一键全平**：管理员手动触发全平，不受任何其他条件限制

### 2.3.2 验收条件（Gherkin）

```gherkin
Feature: 系统应急风控

  Background:
    Given 用户持有 BTC 多单 5 手

  Scenario: WebSocket 断连超过阈值，暂停开仓
    Given WebSocket 连接状态为 "connected"
    And 断线检测阈值为 30 秒
    When WebSocket 断连持续 31 秒
    Then 系统自动将所有策略状态切换为 "paused"
    And 禁止新订单发出
    And 推送告警："WebSocket 断连 31 秒，已暂停策略"

  Scenario: WebSocket 重连后恢复
    Given 策略状态为 "paused"（断线触发）
    And WebSocket 已重连成功
    When 重连持续稳定 10 秒
    Then 系统自动恢复策略状态为 "active"
    And 推送通知："WebSocket 已恢复，策略已重新激活"

  Scenario: 宕机平仓 — 后端异常退出
    Given 后端进程异常崩溃
    And 看门狗检测到进程消失
    When 看门狗触发紧急平仓流程
    Then 向交易所发送全部持仓市价平仓指令
    And 等待成交确认（超时 10s）
    And 记录宕机平仓日志
    And 推送告警："检测到后端进程异常，已执行紧急平仓"

  Scenario: 紧急一键全平
    Given 用户持有多个币种持仓
    When 管理员点击"紧急全平"按钮
    Then 系统立即向所有持仓发送市价平仓指令
    And 不受任何风控规则限制
    And 返回全平结果（含每笔成交详情）
    And 推送通知："管理员已执行紧急全平"

  Scenario: 紧急全平权限校验
    Given 当前用户角色为 "trader"
    When 用户尝试调用紧急全平接口
    Then 返回 403 Forbidden
    And 记录非法尝试日志
```

### 2.3.3 API 端点定义

| 端点 | 方法 | 路径 | 描述 | 认证 | 请求体 | 响应 |
|------|------|------|------|------|--------|------|
| 紧急全平 | POST | /api/v1/risk/emergency-close | 一键平所有持仓 | JWT+Admin | — | EmergencyCloseResponse |
| 暂停策略执行 | POST | /api/v1/risk/pause-strategies | 暂停所有策略 | JWT+Admin | PauseRequest | PauseResponse |
| 恢复策略执行 | POST | /api/v1/risk/resume-strategies | 恢复所有策略 | JWT+Admin | — | ResumeResponse |
| 查询连接状态 | GET | /api/v1/risk/connection-status | WebSocket/交易所连接状态 | JWT | — | ConnectionStatus |
| 告警配置 | PUT | /api/v1/risk/alert-config | 配置告警阈值 | JWT+Admin | AlertConfig | AlertConfigResponse |

---

## 三、P1 核心业务

### P1-F1：技术指标 — KDJ

| 项目 | 内容 |
|------|------|
| 功能名称 | KDJ 随机指标 |
| 功能类型 | 新增功能 |
| 目标用户 | 策略开发者 |
| 优先级 | P1 |
| 预计工时 | 1 天 |
| 依赖 | 阶段二策略引擎（strategy.rs） |

### 3.1.1 功能描述

在 `strategy.rs` 中实现 KDJ 指标计算函数，供策略模板使用：
- K = RSV 的 N 日平滑均值
- D = K 的 M 日平滑均值
- J = 3×K - 2×D
- 默认参数：N=9, M=3

### 3.1.2 验收条件（Gherkin）

```gherkin
Feature: KDJ 指标计算

  Scenario: KDJ 金叉买入信号
    Given 最近 K=35, D=30, J=45
    When 计算新 bar 后 K=45, D=40, J=55
    And 前一 bar K < D
    And 当前 bar K > D
    Then 生成信号 Buy with confidence=0.7

  Scenario: KDJ 死叉卖出信号
    Given K=70, D=65
    When K 下穿 D（K < D）
    Then 生成信号 Sell

  Scenario: KDJ 超买区间
    Given K > 80 且 D > 80
    When 策略检测到超买
    Then 生成信号 Sell（高置信度）

  Scenario: KDJ 超卖区间
    Given K < 20 且 D < 20
    When 策略检测到超卖
    Then 生成信号 Buy（高置信度）
```

---

### P1-F2：实盘止盈止损

| 项目 | 内容 |
|------|------|
| 功能名称 | 实盘独立止盈止损单 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员 |
| 优先级 | **P1** — 回测有信号但实盘无法落地 |
| 预计工时 | 3 天 |
| 依赖 | P0-F2 资金风控框架 |

### 3.2.1 功能描述

当前止盈止损仅在 `backtest_engine.rs` 中实现。需要在实盘订单服务中独立实现：
- **止盈单**：价格达到目标价时自动触发平仓
- **止损单**：价格达到止损价时自动触发平仓
- **追踪止损**：随价格有利方向移动止损价
- **支持限价/市价两种触发方式**

### 3.2.2 验收条件（Gherkin）

```gherkin
Feature: 实盘止盈止损

  Scenario: 开仓时附加止盈止损
    Given 用户以 65000 USDT 开多 BTC
    When 用户设置止盈价 68000，止损价 63000
    Then 订单创建成功，返回订单 ID 和附加的 TP/SL ID

  Scenario: 价格触及止盈，触发平仓
    Given 用户持有 BTC 多单，止盈价 68000
    When 市场最新价达到 68000
    Then 系统发送市价平多指令
    And 记录止盈触发日志

  Scenario: 价格触及止损，触发平仓
    Given 用户持有 BTC 多单，止损价 63000
    When 市场最新价达到 63000
    Then 系统发送市价平多指令
    And 推送告警："止损触发，当前价格 63000"

  Scenario: 追踪止损 — 随价格上涨调整
    Given 用户持有 BTC 多单，开仓价 65000，追踪止损系数 0.5%
    And 最高价涨至 67000（回撤点 = 67000 × 0.5% = 66.65）
    When 价格从 67000 回落至 66600
    Then 止损价自动更新为 66600 × (1-0.5%) = 66.267

  Scenario: 手动修改止盈止损
    Given 用户持有持仓，止盈价 68000
    When 用户将止盈价修改为 70000
    Then 止盈价立即更新
    And 记录修改日志

  Scenario: 部分持仓止盈
    Given 用户持有 10 手 BTC 多单
    And 止盈触发时成交 6 手
    When 剩余 4 手继续持有
    Then 更新持仓数量为 4 手
    And 止盈止损规则对剩余持仓继续生效
```

---

### P1-F3：条件单

| 项目 | 内容 |
|------|------|
| 功能名称 | 条件触发单（Trigger Order） |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员 |
| 优先级 | P1 |
| 预计工时 | 3 天 |
| 依赖 | P1-F2 实盘止盈止损 |

### 3.3.1 功能描述

支持以下条件单类型：
- **止损单（Stop-Loss）**：价格跌破触发价时激活市价/限价单
- **止盈单（Take-Profit）**：价格涨超触发价时激活市价/限价单
- **OCO（One-Cancels-Other）**：同时设置止盈和止损，触发一个则取消另一个
- **时间加权平均价格（TWAP）**：在指定时间内分批成交

### 3.3.2 验收条件（Gherkin）

```gherkin
Feature: 条件触发单

  Scenario: 止损单激活
    Given 用户持有空仓，当前价格 64000
    When 用户设置触发价 65000 的止损多单（做多平空）
    And 市场最新价达到 65000
    Then 系统激活限价买单，价格 = 触发价 + 滑点补偿

  Scenario: OCO 单 — 止盈触发，止损取消
    Given 用户设置 OCO：止盈 68000，止损 62000
    When 价格达到 68000（止盈触发）
    Then 激活市价平多
    And 取消止损单 62000

  Scenario: OCO 单 — 止损触发，止盈取消
    Given 用户设置 OCO：止盈 68000，止损 62000
    When 价格达到 62000（止损触发）
    Then 激活市价平多
    And 取消止盈单 68000

  Scenario: 条件单撤销
    Given 用户有待激活的止损单
    When 用户主动撤销
    Then 条件单状态变更为 cancelled
    And 不再触发
```

---

### P1-F4：下单频率限制

| 项目 | 内容 |
|------|------|
| 功能名称 | 下单频率限制（Rate Limiting） |
| 功能类型 | 新增功能 |
| 目标用户 | 运营者（配置）、交易员（被限制） |
| 优先级 | P1 |
| 预计工时 | 2 天 |
| 依赖 | 阶段二订单执行 |

### 3.4.1 功能描述

防止刷单行为的频率限制：
- **单用户频率限制**：每分钟/每秒最大下单数（可按角色区分 trader/admin）
- **单交易对频率限制**：每个交易对每分钟最大下单数
- **全局频率限制**：整个系统每分钟最大下单总数
- **频率限制策略**：拒绝/排队/节流

### 3.4.2 验收条件（Gherkin）

```gherkin
Feature: 下单频率限制

  Scenario: 正常频率下单成功
    Given 用户在过去 60 秒内已下 5 单
    And 频率限制为 10 单/分钟
    When 用户发起第 6 单
    Then 下单成功

  Scenario: 频率超限被拒绝
    Given 用户在过去 60 秒内已下 9 单
    And 频率限制为 10 单/分钟
    When 用户发起第 10 单
    Then 下单成功（未超限）

  Scenario: 第 11 单被拒绝
    Given 用户在过去 60 秒内已下 10 单
    And 频率限制为 10 单/分钟
    When 用户发起第 11 单
    Then 返回 429 Too Many Requests
    And 错误码 "RL-001: Order rate limit exceeded"
    And 告知剩余等待时间

  Scenario: 频率限制按交易对独立计算
    Given 用户在 BTC/USDT 交易对已达 20 单/分钟限制
    And 在 ETH/USDT 交易对仅下 5 单
    When 用户在 ETH/USDT 发起第 6 单
    Then 下单成功（Binance 按交易对独立计数）
```

---

### P1-F5：分表存储（数据库）

| 项目 | 内容 |
|------|------|
| 功能名称 | 历史行情数据分表存储 |
| 功能类型 | 架构优化 |
| 目标用户 | 系统（内部使用） |
| 优先级 | P1 |
| 预计工时 | 2 天 |
| 依赖 | 阶段一数据库架构 |

### 3.5.1 功能描述

对 `klines` 历史行情数据进行分表存储优化：
- **按月分区**：`klines_2026_01`, `klines_2026_02`
- **按交易对分区**：`klines_btcusdt`, `klines_ethusdt`（可选）
- **自动分区管理**：PostgreSQL PARTITION BY RANGE
- **分区自动清理**：超过 N 个月的分区自动归档/删除

---

### P1-F6：告警推送

| 项目 | 内容 |
|------|------|
| 功能名称 | 多渠道告警推送 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员、运营者 |
| 优先级 | P1 |
| 预计工时 | 3 天 |
| 依赖 | P0-F2/F3 告警触发点 |

### 3.6.1 功能描述

实现多渠道告警推送服务：
- **微信（WeChat）**：通过企业微信 Webhook 推送
- **邮件（Email）**：SMTP 发送告警邮件
- **短信（SMS）**：可选（成本考虑，可后置）
- **告警收敛**：同类告警 N 分钟内合并为一条

告警触发场景：
- 开平仓通知
- 止损/止盈触发
- 风控规则触发
- 策略异常/停止
- WebSocket 断连
- 宕机/进程异常

---

## 四、P2 增强体验

### P2-F1：密钥管理

| 项目 | 内容 |
|------|------|
| 功能名称 | API 密钥管理界面 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员（管理自己的 Key）、管理员（管理全局配置） |
| 优先级 | P2 |
| 预计工时 | 2 天 |
| 依赖 | P0-F1 签名加密后端 |

### 4.1.1 功能描述

提供前端 UI 供用户管理自己的交易所 API Key：
- 添加/编辑/删除 API Key
- 设置 Key 权限范围（只读/交易/划转）
- 测试 Key 连通性（调用 /exchange/ping）
- 查看 Key 最后使用时间
- 管理员可查看全局 Key 状态

---

### P2-F2：策略审核工作流

| 项目 | 内容 |
|------|------|
| 功能名称 | 策略提交-审核工作流 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员（提交）、管理员（审核） |
| 优先级 | P2 |
| 预计工时 | 2 天 |
| 依赖 | 阶段二策略管理 |

### 4.2.1 功能描述

当前策略状态仅有 `draft/active/paused/stopped`。需要新增审核状态：
- `pending_review`：策略提交审核，等待管理员审批
- `rejected`：审核未通过，附拒绝理由
- `approved`：审核通过，可切换为 active

---

### P2-F3：涨跌停限制

| 项目 | 内容 |
|------|------|
| 功能名称 | 回测涨跌停价格限制 |
| 功能类型 | 新增功能 |
| 目标用户 | 量化研究员（回测用） |
| 优先级 | P2 |
| 预计工时 | 1 天 |
| 依赖 | 阶段三回测撮合引擎 |

### 4.3.1 功能描述

在 `backtest_engine.rs` 中实现涨跌停限制：
- 价格达到涨跌停板时，无法以更劣价格成交
- 涨跌停时订单簿仅有卖一/买一，无对手方流动性
- 实际成交价超出涨跌停价时，以涨跌停价替代

---

### P2-F4：交易/账单 Excel 导出

| 项目 | 内容 |
|------|------|
| 功能名称 | 交易记录与账单导出 |
| 功能类型 | 新增功能 |
| 目标用户 | 交易员、财务 |
| 优先级 | P2 |
| 预计工时 | 2 天 |
| 依赖 | 阶段五已有基础 UI |

### 4.4.1 功能描述

在已有 K 线导出的基础上，增加：
- 订单历史导出（CSV/Excel）：时间、交易对、方向、价格、数量、手续费、盈亏
- 账单导出（CSV/Excel）：日/周/月账单，含汇总统计
- 持仓报告导出

---

### P2-F5：压测与极端行情测试

| 项目 | 内容 |
|------|------|
| 功能名称 | 7×24h 压测 + 极端行情测试套件 |
| 功能类型 | 新增功能 |
| 目标用户 | DevOps、QA |
| 优先级 | P2 |
| 预计工时 | 2 天 |
| 依赖 | 阶段五部署完成 |

### 4.5.1 功能描述

建立 `devops/` 压测目录：
- **负载测试**：wrk2 持续发送订单请求，验证 QPS 上限
- **极端行情注入**：模拟价格瞬间涨跌 20%、WebSocket 消息洪泛
- **断网测试**：kill WebSocket 连接，验证自动重连和订单状态
- **长时间稳定性**：7×24h 持续运行，无内存泄漏

---

## 五、P3 高阶增值

### P3-F1：套利模块

| 项目 | 内容 |
|------|------|
| 功能名称 | 跨期/跨品种/期现套利 |
| 功能类型 | 新增功能 |
| 目标用户 | 专业量化交易员 |
| 优先级 | P3 |
| 预计工时 | 7 天 |

### 5.1.1 功能描述

- **跨期套利**：同交易对不同到期期货合约，捕捉近远月价差
- **跨品种套利**：两个相关性高的交易对（如 BTC/ETH），配对交易
- **期现套利**：期货 vs 现货价格收敛交易
- **价差监控**：实时监控套利机会，触发信号
- **自动对冲**：套利开仓时自动对冲两个方向

---

### P3-F2：网格马丁

| 项目 | 内容 |
|------|------|
| 功能名称 | 网格交易 + 马丁格尔加仓 |
| 功能类型 | 新增功能 |
| 目标用户 | 网格交易爱好者 |
| 优先级 | P3 |
| 预计工时 | 5 天 |

### 5.2.1 功能描述

- **静态等距网格**：价格区间内均匀布置网格，每格低买高卖
- **动态网格**：根据波动率自动调整网格密度
- **马丁格尔加仓**：亏损后加倍仓位，摊薄成本（高风险）
- **震荡自适应**：识别震荡/趋势市场，自动切换策略

---

### P3-F3：AI 量化模块

| 项目 | 内容 |
|------|------|
| 功能名称 | AI 行情预测 + 参数自动优化 |
| 功能类型 | 新增功能 |
| 目标用户 | 高级量化研究者 |
| 优先级 | P3 |
| 预计工时 | 15 天（需模型训练基础设施） |

### 5.3.1 功能描述

- **时序预测**：LSTM/Transformer 模型预测价格短期走势
- **市场情绪识别**：通过订单簿特征识别多空力量
- **参数自动优化**：贝叶斯优化/遗传算法自动调参
- **模型版本管理**：A/B 策略切换，实盘对照实验

---

## 六、开发优先级与排期建议

### 建议排期（按 P0→P1→P2→P3）

```
第 1-3 天  ：P0-F1 API 签名加密（阻塞其他所有带认证的功能）
第 4-7 天  ：P0-F2 资金风控（核心保命，并行 P0-F3）
第 8-10 天 ：P0-F3 应急风控（可与 P0-F2 并行）
第 11-13 天：P1-F2 实盘止盈止损（并行 P1-F3）
第 14-15 天：P1-F4 频率限制
第 16-17 天：P1-F1 KDJ + P1-F5 分表
第 18-20 天：P1-F6 告警推送
第 21-22 天：P2-F1 密钥管理 + P2-F2 策略审核
第 23-24 天：P2-F3 涨跌停 + P2-F4 导出
第 25-26 天：P2-F5 压测套件
第 27 天以后 ：P3 套利/网格/AI（商业化阶段）
```

---

## 七、附录

### A. 参考文档

| 文档 | 路径 |
|------|------|
| 技术架构文档 | `docs/architecture/` |
| ADR-010 风控设计 | `docs/architecture/ADR-010-order-management.md` |
| 现有风控规则 | `docs/architecture/risk-analysis.md` |
| 后端 Handlers | `backend/src/handlers/` |
| 策略服务 | `backend/src/services/strategy.rs` |
| 回测引擎 | `backend/src/services/backtest_engine.rs` |

### B. ADR 更新计划

以下 ADR 需要新建或更新：

| ADR | 内容 | 状态 |
|-----|------|------|
| ADR-011 | 风控规则引擎设计 | 待新建 |
| ADR-012 | API 签名认证架构 | 待新建 |
| ADR-013 | 条件单引擎设计 | 待新建 |
| ADR-014 | 告警推送服务架构 | 待新建 |
| ADR-010 更新 | 补充 risk_manager.rs 服务 | 待更新 |

### C. 依赖关系图

```
P0-F1 (API签名)
    ↓
P1-F2 (实盘止盈止损) ←→ P0-F2 (资金风控框架)
    ↓
P1-F3 (条件单) ← P1-F6 (告警推送)
    ↓
P2-F1 (密钥管理)
P2-F2 (策略审核)
```
