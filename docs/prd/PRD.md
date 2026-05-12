# 量化交易系统 — 产品需求文档 (PRD)

> 版本: v1.0
> 状态: Draft
> 作者: PM

---

## 1. 背景

量化交易在国内资本市场已从早期对冲基金的工具演进为机构和个人投资者的核心基础设施。传统交易平台存在以下痛点：

1. **行情数据滞后**：多数平台实时性不足，无法满足高频/半高频策略对微秒级行情的需求
2. **策略开发门槛高**：量化策略的编写、回测、部署流程割裂，缺乏一站式工作流
3. **回测不可靠**：回测引擎对交易成本、滑点、市场冲击的模拟过于简化，导致回测结果与实盘差距大
4. **风险管理缺失**：缺少实时风控指标（VaR、最大回撤、夏普比率等），持仓风险不可见

本项目旨在构建一套全栈量化交易系统，覆盖行情查看 → 策略管理 → 回测验证 → 交易执行 → 持仓监控的完整闭环。

---

## 2. 目标

| 目标 | 指标 |
|------|------|
| 行情数据延迟 | WebSocket 推送延迟 < 500ms |
| 回测精度 | 模拟交易成本/滑点，回测夏普比率误差 < 5% |
| 策略管理 | 支持同时运行 ≥ 20 个策略实例 |
| 用户体验 | 核心功能（行情、策略、回测）3 次点击内可达 |
| 系统可用性 | 99.5%（每月计划内维护 ≤ 2h） |

---

## 3. 用户角色

| 角色 | 描述 | 权限范围 |
|------|------|----------|
| **普通用户 (trader)** | 个人交易者 | 查看行情、管理自有策略、运行回测、模拟交易、查看自有持仓 |
| **交易员 (pro-trader)** | 专业交易者 | 普通用户权限 + 实盘交易、多账户管理 |
| **管理员 (admin)** | 系统管理员 | 用户管理、系统配置、风控参数设置、日志审计 |

---

## 4. 用户故事与验收条件

### 4.1 用户系统 (P0)

#### US-01: 用户注册
> **As a** 新用户  
> **I want to** 注册账号  
> **So that** 我可以使用量化交易系统

```gherkin
Feature: 用户注册
  Scenario: 成功注册新用户
    Given 访问注册页面
    When 输入邮箱 "trader@example.com"
      And 输入密码 "Abc@123456"
      And 输入确认密码 "Abc@123456"
      And 勾选"同意服务条款"
    Then 提交注册
      And 系统返回 201
      And 发送验证邮件至 "trader@example.com"
      And 提示"注册成功，请查收验证邮件"

  Scenario: 注册时邮箱已存在
    Given 邮箱 "trader@example.com" 已注册
    When 尝试使用相同邮箱注册
    Then 系统返回 409
      And 提示"该邮箱已被注册"

  Scenario: 密码强度不足
    Given 访问注册页面
    When 输入密码 "123"
    Then 提示"密码至少8位，需包含大小写字母和数字"
      And 注册按钮禁用

  Scenario: 两次密码输入不一致
    Given 访问注册页面
    When 输入密码 "Abc@123456"
      And 输入确认密码 "Abc@654321"
    Then 提示"两次密码不一致"
      And 注册按钮禁用
```

---

#### US-02: 用户登录与 Token 鉴权
> **As a** 已注册用户  
> **I want to** 登录系统  
> **So that** 我可以访问受保护的交易功能

```gherkin
Feature: 用户登录与鉴权
  Scenario: 邮箱密码成功登录
    Given 用户 "trader@example.com" 已激活
    When 输入邮箱 "trader@example.com"
      And 输入密码 "Abc@123456"
      And 点击登录
    Then 系统返回 access_token (有效期15分钟)
      And 系统返回 refresh_token (有效期7天)
      And 跳转至仪表盘页面

  Scenario: 密码错误登录失败
    Given 用户 "trader@example.com" 已注册
    When 输入邮箱 "trader@example.com"
      And 输入密码 "WrongPass@123"
    Then 提示"邮箱或密码错误"
      And 连续失败5次后, 账号锁定30分钟

  Scenario: access_token 过期自动刷新
    Given 用户已登录且 access_token 已过期
    When 调用受保护API
    Then 客户端使用 refresh_token 自动刷新
      And 返回新的 access_token
      And 原请求重试成功

  Scenario: refresh_token 过期需重新登录
    Given 用户 7 天未登录
    When 调用受保护API
    Then 提示"登录已过期，请重新登录"
      And 跳转至登录页

  Scenario: 用户主动登出
    Given 用户已登录
    When 点击登出
    Then 服务端将当前 access_token 加入黑名单
      And 清除客户端 token
      And 跳转至登录页
```

---

#### US-03: 角色与权限管理
> **As a** 管理员  
> **I want to** 管理用户角色和权限  
> **So that** 不同用户只能访问其授权范围内的功能

```gherkin
Feature: 角色与权限管理
  Scenario: 管理员查看用户列表
    Given 当前登录用户角色为 "admin"
    When 访问系统管理 → 用户管理
    Then 显示用户列表（分页）
      And 每行包含: 头像、用户名、邮箱、角色、状态、注册时间、操作

  Scenario: 管理员修改用户角色
    Given 管理员已登录
      And 目标用户 "trader@example.com" 当前角色为 "trader"
    When 将目标用户角色改为 "pro-trader"
    Then 角色更新成功
      And 目标用户下次登录生效新权限
      And 操作记录写入审计日志

  Scenario: 非管理员无法访问管理页面
    Given 当前登录用户角色为 "trader"
    When 直接访问 /admin/users
    Then 返回 403
      And 提示"权限不足"

  Scenario: 权限校验中间件
    Given 角色为 "trader" 的用户
    When 调用 POST /api/v1/trades/execute (实盘交易)
    Then 中间件拦截并返回 403
```

---

### 4.2 行情模块 (P0)

#### US-04: 查看实时K线
> **As a** 交易者  
> **I want to** 查看实时 K 线图  
> **So that** 我可以分析价格走势和识别交易机会

```gherkin
Feature: 实时K线
  Scenario: 加载指定交易对的K线数据
    Given 选择交易对 "BTC/USDT"
      And 选择时间周期 "1d"
    When 打开K线页面
    Then 显示最近 500 根 K 线
      And 支持缩放（1m/5m/15m/1h/4h/1d/1w）
      And 支持移动（左/右拖拽查看历史）

  Scenario: K线 WebSocket 实时更新
    Given K线页面已打开
    When 新的 1m K线收盘
    Then 通过 WebSocket 推送新 K 线数据
      And 图表自动更新（不刷新页面）

  Scenario: 指标叠加
    Given K线图已渲染
    When 选择叠加指标 "MA(5,20)"
    Then K线上叠加均线
      And 切换至 "BOLL" 时均线隐藏，布林带显示

  Scenario: 跨周期视图
    Given 当前查看 1h K线
    When 侧边栏显示同时间段的 15m/4h/1d 缩略图
    Then 点击任一缩略图切换至对应周期
```

---

#### US-05: 深度数据与盘口
> **As a** 交易者  
> **I want to** 查看市场深度  
> **So that** 我可以判断买卖压力和流动性

```gherkin
Feature: 市场深度
  Scenario: 加载深度数据
    Given 选择交易对 "BTC/USDT"
    When 点击深度标签
    Then 显示买一至买十档位（价格 + 数量 + 累计）
      And 显示卖一至卖十档位（价格 + 数量 + 累计）
      And 显示深度图（横轴价格、纵轴累计量）

  Scenario: 深度实时更新
    Given 深度页面已打开
    When 市场出现新订单
    Then 深度数据通过 WebSocket 实时更新
      And 深度图平滑过渡动画
```

---

#### US-06: Ticker 实时行情
> **As a** 交易者  
> **I want to** 查看 Ticker 概览  
> **So that** 我可以快速掌握市场概况

```gherkin
Feature: Ticker 行情
  Scenario: Ticker 面板显示
    Given 行情页面
    When 页面加载
    Then 显示所有交易对 Ticker
      And 每行包含: 交易对、最新价、24h涨跌幅、24h最高/最低、24h成交量
      And 支持按涨跌幅排序
      And 支持搜索交易对

  Scenario: Ticker 价格闪烁
    Given Ticker 面板打开
    When 最新价发生变化
    Then 价格上涨时数字绿色闪烁
      And 价格下跌时数字红色闪烁
      And 0.5s 后恢复正常颜色

  Scenario: 自选列表
    Given 用户已登录
    When 点击交易对旁的星标
    Then 该交易对加入自选列表
      And 自选列表独立 Tab 展示
```

---

### 4.3 策略管理 (P0)

#### US-07: 创建策略
> **As a** 交易者  
> **I want to** 创建量化策略  
> **So that** 我可以定义交易逻辑并自动化执行

```gherkin
Feature: 创建策略
  Scenario: 使用代码编辑器创建策略
    Given 使用 "双均线交叉策略" 模板
    When 编辑策略代码（Python/Pine Script 语法高亮）
      And 设置策略参数: fast_period=5, slow_period=20
      And 设置交易对: "BTC/USDT"
      And 设置时间周期: "1h"
    When 点击"保存"
    Then 策略保存成功
      And 显示在策略列表中
      And 自动验证代码语法，如有错误提示具体行号

  Scenario: 策略参数校验
    Given 创建策略页面
    When fast_period 设置为 0
    Then 提示"周期必须为正整数"
      And 保存按钮禁用

  Scenario: 策略名称重复
    Given 已存在策略 "双均线策略"
    When 创建同名策略
    Then 提示"策略名称已存在"
```

---

#### US-08: 启用/停止策略
> **As a** 交易者  
> **I want to** 启用或停止策略  
> **So that** 我可以控制策略的执行

```gherkin
Feature: 启停策略
  Scenario: 启用策略（模拟模式）
    Given 策略 "双均线策略" 状态为 "已停止"
    When 点击"启用"
      And 选择交易模式 "模拟"
    Then 策略状态变为 "运行中(模拟)"
      And 开始接收实时行情执行策略逻辑
      And 产生模拟委托但不下发至交易所
      And 日志面板显示 "策略已启动 - 模拟模式"

  Scenario: 启用策略（实盘模式）
    Given 策略 "双均线策略" 状态为 "已停止"
      And 当前用户角色为 "pro-trader"
    When 点击"启用"
      And 选择交易模式 "实盘"
    Then 弹出风控确认对话框
      And 显示: 最大持仓、单笔止损、每日最大亏损
      And 用户确认后
    Then 策略状态变为 "运行中(实盘)"
      And 委托下发至交易所
      And 日志面板显示 "策略已启动 - 实盘模式"

  Scenario: 停止策略
    Given 策略 "双均线策略" 状态为 "运行中"
    When 点击"停止"
    Then 策略停止接收新行情
      And 当前未成交委托保留（可选撤单）
      And 策略状态变为 "已停止"
      And 日志面板显示 "策略已停止"

  Scenario: trader 角色无法实盘交易
    Given 当前用户角色为 "trader"
    When 启用策略时选择 "实盘" 模式
    Then 提示"权限不足，请联系管理员开通实盘权限"
      And 切换至模拟模式
```

---

### 4.4 回测引擎 (P0)

#### US-09: 运行回测
> **As a** 交易者  
> **I want to** 对策略进行历史回测  
> **So that** 我可以评估策略的历史表现

```gherkin
Feature: 运行回测
  Scenario: 配置并运行回测
    Given 策略 "双均线策略" 已创建
    When 进入回测页面
      And 选择回测时间段: 2024-01-01 ~ 2024-12-31
      And 设置初始资金: ¥100,000
      And 设置手续费率: 0.1%
      And 设置滑点: 0.05%
    When 点击"开始回测"
    Then 回测任务提交成功
      And 显示回测进度条
      And 进度到 100% 后显示结果

  Scenario: 回测参数校验
    Given 回测配置页面
    When 结束时间早于开始时间
    Then 提示"结束时间不能早于开始时间"
      And 开始回测按钮禁用

  Scenario: 回测结果包含核心指标
    Given 回测已完成
    Then 结果页显示:
      | 指标 | 说明 |
      | 总收益率 | 策略整体收益百分比 |
      | 年化收益率 | 年化后的收益率 |
      | 最大回撤 | 峰值到谷底的最大亏损 |
      | 夏普比率 | 风险调整后收益 |
      | 胜率 | 盈利交易占比 |
      | 交易次数 | 总交易次数 |
      | 盈亏比 | 平均盈利/平均亏损 |

  Scenario: 回测资金曲线
    Given 回测结果已展示
    When 查看资金曲线
    Then 显示 X 轴时间，Y 轴权益的折线图
      And 标注最大回撤区间（高亮区域）
      And 标注每次开仓/平仓点
```

---

#### US-10: 回测报告导出
> **As a** 交易者  
> **I want to** 导出回测报告  
> **So that** 我可以分享或存档分析结果

```gherkin
Feature: 回测报告
  Scenario: 导出 CSV 交易明细
    Given 回测已完成
    When 点击"导出交易明细"
    Then 下载 CSV 文件
      And 包含字段: 开仓时间、平仓时间、方向、开仓价、平仓价、数量、盈亏、盈亏率
      And 编码为 UTF-8 with BOM

  Scenario: 导出 PDF 报告
    Given 回测已完成
    When 点击"导出 PDF 报告"
    Then 下载 PDF 文件
      And 包含: 策略概要、核心指标、资金曲线、月度收益热力图

  Scenario: 回测记录保存
    Given 回测已完成
    When 保存回测记录
    Then 该回测出现在历史回测列表中
      And 支持查看、比较、删除
```

---

### 4.5 交易执行 (P1)

#### US-11: 手动下单
> **As a** 交易者  
> **I want to** 手动创建委托  
> **So that** 我可以灵活执行交易

```gherkin
Feature: 手动下单
  Scenario: 限价单下单
    Given 交易页面已打开
    When 选择交易对 "BTC/USDT"
      And 选择委托类型 "限价单"
      And 输入价格 ¥50,000
      And 输入数量 0.1 BTC
      And 选择方向 "买入"
    When 点击"提交委托"
    Then 弹出确认对话框（显示价格、数量、总金额）
      And 用户确认后
    Then 委托提交成功
      And 显示在"当前委托"列表中
      And 状态为 "待成交"

  Scenario: 市价单下单
    Given 交易页面已打开
    When 选择委托类型 "市价单"
      And 输入数量 0.1 BTC
      And 选择方向 "卖出"
    When 点击"提交委托"
    Then 委托提交成功
      And 立即按市场最优价成交
      And 显示在"历史成交"列表中

  Scenario: 委托数量校验
    Given 下单页面
    When 买入数量为 0
    Then 提示"数量必须大于0"
      And 提交按钮禁用

  Scenario: 模拟模式下下单确认
    Given 当前为模拟交易
    When 提交限价单
    Then 委托进入模拟撮合引擎
      And 不产生实际资金变动
```

---

#### US-12: 委托管理
> **As a** 交易者  
> **I want to** 查看和管理我的委托  
> **So that** 我可以跟踪订单状态

```gherkin
Feature: 委托管理
  Scenario: 查看当前委托
    Given 用户有未成交委托
    When 打开"当前委托"页面
    Then 显示所有未完全成交的委托
      And 每行包含: 时间、交易对、方向、类型、价格、数量、已成交、状态、操作
      And 支持按交易对筛选

  Scenario: 撤单
    Given 存在状态为 "待成交" 的委托
    When 点击该委托的"撤单"按钮
    Then 发起撤单请求
      And 委托状态变为 "已撤销"
      And 已成交部分仍保留

  Scenario: 查看历史委托
    Given 用户有历史委托记录
    When 打开"历史委托"页面
    Then 显示所有已成交/已撤销/已过期的委托
      And 支持按日期范围筛选
      And 支持按状态筛选
```

---

### 4.6 持仓分析 (P1)

#### US-13: 持仓概览
> **As a** 交易者  
> **I want to** 查看我的持仓  
> **So that** 我可以了解当前资产配置

```gherkin
Feature: 持仓概览
  Scenario: 查看持仓列表
    Given 用户有持仓
    When 打开"持仓"页面
    Then 显示所有持仓汇总
      And 每行包含: 交易对、持仓数量、开仓均价、当前价、浮动盈亏、盈亏率、占用保证金

  Scenario: 资产分布饼图
    Given 持仓页面已打开
    When 查看资产分布
    Then 显示饼图: 各交易对市值占比
      And 显示现金占比

  Scenario: 持仓盈亏实时更新
    Given 持仓页面已打开
    When 某持仓的当前价变化
    Then 浮动盈亏和盈亏率实时更新
      And 盈利显示绿色、亏损显示红色

  Scenario: 平仓操作
    Given 存在持仓 "BTC/USDT"
    When 点击该持仓的"平仓"按钮
      And 选择平仓数量（全部/部分）
    Then 生成市价卖出委托
      And 成交后持仓数量减少
      And 盈亏计入已实现盈亏
```

---

#### US-14: 盈亏分析
> **As a** 交易者  
> **I want to** 查看盈亏统计  
> **So that** 我可以评估交易表现

```gherkin
Feature: 盈亏分析
  Scenario: 按时间维度的盈亏统计
    Given 用户有交易记录
    When 打开"盈亏分析"页面
    Then 显示今日盈亏、本周盈亏、本月盈亏、累计盈亏
      And 显示每日盈亏柱状图

  Scenario: 盈亏排行榜
    Given 用户有多个交易对的历史盈亏
    When 查看盈亏分析
    Then 按交易对统计总盈亏
      And 按盈亏金额降序排列

  Scenario: 月度收益热力图
    Given 用户有超过1个月的交易记录
    When 查看月度统计
    Then 显示热力图: X轴月份, Y轴年份
      And 颜色表示收益率（绿色为正、红色为负、无色为无交易）
```

---

#### US-15: 风控指标
> **As a** 交易者  
> **I want to** 查看风险指标  
> **So that** 我可以控制交易风险

```gherkin
Feature: 风控指标
  Scenario: 查看风险仪表盘
    Given 用户有持仓和交易记录
    When 打开"风险分析"页面
    Then 显示:
      | 指标 | 说明 |
      | VaR (95%, 1d) | 95%置信度下日最大可能亏损 |
      | 杠杆率 | 当前杠杆倍数 |
      | 集中度 | 最大持仓占比 |
      | 最大回撤 | 历史最大回撤 |

  Scenario: 风控预警
    Given 风控规则已配置
      And 杠杆率 > 3
    When 用户尝试开新仓
    Then 弹出警告"当前杠杆率超过阈值（3x）"
      And 确认后继续或取消
```

---

### 4.7 仪表盘 (P1)

#### US-16: 仪表盘总览
> **As a** 交易者  
> **I want to** 在一个页面查看关键信息  
> **So that** 我可以快速掌握整体状况

```gherkin
Feature: 仪表盘
  Scenario: 仪表盘初始加载
    Given 用户已登录
    When 访问首页
    Then 仪表盘显示以下卡片:
      - 总资产（折线图，近30天趋势）
      - 今日盈亏（数字 + 百分比）
      - 运行中的策略数
      - 当前委托数
      - 最近交易（列表，近5条）
      - 自选行情（迷你 K 线）

  Scenario: 仪表盘卡片可自定义
    Given 仪表盘页面
    When 点击"编辑布局"
    Then 卡片可拖拽排序
      And 卡片可显示/隐藏
      And 布局设置保存至用户偏好

  Scenario: 仪表盘数据定时刷新
    Given 仪表盘已打开
    When 每 30 秒
    Then 自动刷新仪表盘数据
      And 刷新时不重置卡片滚动位置
```

---

## 5. 非功能性需求

### 5.1 性能 (P0)

| 指标 | 目标值 |
|------|--------|
| 页面首屏加载 | < 2s (3G 网络) |
| API 响应时间 (P99) | < 200ms |
| WebSocket 推送延迟 | < 500ms |
| 回测计算 (1年1h数据) | < 30s |
| 并发用户支持 | ≥ 1000 同时在线 |

### 5.2 安全 (P0)

| 要求 | 说明 |
|------|------|
| 传输加密 | 全站 HTTPS, WebSocket WSS |
| 密码策略 | 最小8位, 含大小写字母+数字, bcrypt 加密 |
| Token 安全 | Access 15min + Refresh 7d, Refresh Token 轮换 |
| API 限流 | 每用户 100 req/min |
| 敏感数据 | 不记录密码/密钥/token 到日志 |
| SQL 注入防护 | ORM 参数化查询 |
| CORS | 仅允许配置的前端域名 |

### 5.3 可用性 (P1)

| 要求 | 说明 |
|------|------|
| 系统可用性 | 99.5%（月停机 ≤ 2h） |
| 数据备份 | 数据库每日自动备份, 保留30天 |
| 故障恢复 | RTO ≤ 1h, RPO ≤ 15min |
| 降级策略 | 行情模块故障时, 交易功能仍可用（手动输入价格） |

### 5.4 可扩展性 (P1)

- 行情网关支持接入多个交易所（Binance, OKX, Bybit 等）
- 策略引擎支持插件化加载自定义策略
- 数据模型支持新增交易品种（股票、期货、期权）

---

## 6. 边界情况与异常处理

### 6.1 网络异常

```gherkin
Feature: 网络断线恢复
  Scenario: WebSocket 断线重连
    Given 行情页面已连接
    When 网络中断
    Then 客户端检测到 WebSocket 断开
      And 自动重连（指数退避: 1s → 2s → 4s → 8s, max 30s）
      And 重连成功后补发断线期间的增量数据
      And 页面显示"已重连"提示 3s 后消失

  Scenario: HTTP 请求超时
    Given 服务器响应缓慢
    When API 请求超时 (默认 10s)
    Then 前端显示"请求超时，请重试"
      And 提供"重试"按钮
```

### 6.2 数据异常

```gherkin
Feature: 数据异常处理
  Scenario: K线数据缺失
    Given 请求某交易对某时间段的 K 线
    When 数据库中该时间段无数据
    Then 返回空数组
      And 图表显示"暂无数据"占位

  Scenario: 回测数据不足
    Given 选择 2024-01-01 ~ 2024-01-05 回测
    When 该时间段交易日 < 5 天
    Then 提示"回测数据不足，请选择更长时间段"

  Scenario: 交易所 API 限流
    Given 行情网关通过交易所 API 获取数据
    When 交易所返回 429 (Too Many Requests)
    Then 行情网关自动退避重试
      And 降级至缓存数据
      And 记录告警日志
```

### 6.3 交易异常

```gherkin
Feature: 交易异常处理
  Scenario: 账户余额不足
    Given 用户模拟账户余额 ¥10,000
    When 下单金额 ¥100,000
    Then 提示"账户余额不足"
      And 委托不提交

  Scenario: 交易所维护
    Given 当前为实盘模式
    When 交易所返回维护状态
    Then 策略自动暂停
      And 通知用户"交易所维护中，策略已暂停"
      And 维护结束后自动恢复

  Scenario: 风控拦截
    Given 风控规则: 单笔亏损 > ¥5,000 拒绝
    When 策略生成的止损单预期亏损 ¥6,000
    Then 策略引擎拒绝该委托
      And 写入风控日志
      And 通知用户"风控规则触发，委托已拦截"
```

---

## 7. 功能优先级汇总

| 模块 | 功能 | 优先级 | 说明 |
|------|------|--------|------|
| 用户系统 | 注册/登录/Token | P0 | MVP 必基 |
| 用户系统 | 角色权限管理 | P0 | 多角色安全 |
| 行情模块 | K线查看 | P0 | 核心功能 |
| 行情模块 | 深度/Ticker | P0 | 核心功能 |
| 策略管理 | 创建/编辑策略 | P0 | 核心功能 |
| 策略管理 | 启停策略 | P0 | 核心功能 |
| 回测引擎 | 运行回测 | P0 | MVP 必基 |
| 回测引擎 | 回测指标与图表 | P0 | 核心功能 |
| 交易执行 | 手动下单 | P1 | 二期 |
| 交易执行 | 委托管理 | P1 | 二期 |
| 持仓分析 | 持仓概览 | P1 | 二期 |
| 持仓分析 | 盈亏统计 | P1 | 二期 |
| 持仓分析 | 风控指标 | P1 | 二期 |
| 仪表盘 | 总览面板 | P1 | 二期 |
| 用户系统 | 密码重置 | P2 | 增强 |
| 回测引擎 | 报告导出 | P2 | 增强 |
| 策略管理 | 策略模板市场 | P2 | 增强 |

---

## 8. 数据模型概要

| 表名 | 主要字段 | 说明 |
|------|----------|------|
| users | id, email, password_hash, role, status, created_at | 用户表 |
| strategies | id, user_id, name, code, params(JSON), status, mode, created_at | 策略表 |
| backtest_results | id, strategy_id, user_id, config(JSON), metrics(JSON), status, created_at | 回测结果 |
| orders | id, user_id, strategy_id, symbol, side, type, price, amount, filled, status, created_at | 委托表 |
| trades | id, order_id, symbol, side, price, amount, pnl, created_at | 成交记录 |
| positions | id, user_id, symbol, quantity, avg_price, unrealized_pnl, created_at | 持仓表 |
| kline_data | symbol, interval, open, high, low, close, volume, timestamp | K线数据 |

---

## 9. API 端点总览

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| POST | /api/v1/auth/register | 用户注册 | P0 |
| POST | /api/v1/auth/login | 用户登录 | P0 |
| POST | /api/v1/auth/refresh | 刷新 token | P0 |
| POST | /api/v1/auth/logout | 登出 | P0 |
| GET | /api/v1/market/kline | K线数据 | P0 |
| GET | /api/v1/market/ticker | 实时 Ticker | P0 |
| GET | /api/v1/market/depth | 深度数据 | P0 |
| WS | /api/v1/market/ws | 行情 WebSocket | P0 |
| CRUD | /api/v1/strategies | 策略 CRUD | P0 |
| POST | /api/v1/strategies/{id}/start | 启策略 | P0 |
| POST | /api/v1/strategies/{id}/stop | 停策略 | P0 |
| POST | /api/v1/backtest/run | 运行回测 | P0 |
| GET | /api/v1/backtest/{id} | 回测结果 | P0 |
| GET | /api/v1/orders | 委托列表 | P1 |
| POST | /api/v1/orders | 创建委托 | P1 |
| DELETE | /api/v1/orders/{id} | 撤单 | P1 |
| GET | /api/v1/portfolio | 持仓 | P1 |
| GET | /api/v1/portfolio/pnl | 盈亏分析 | P1 |
| GET | /api/v1/portfolio/risk | 风控指标 | P1 |
| GET | /api/v1/dashboard | 仪表盘数据 | P1 |
| CRUD | /api/v1/admin/users | 用户管理 | P0 |

---

## 10. 前端路由设计

| 路径 | 页面 | 权限 | 优先级 |
|------|------|------|--------|
| /login | 登录 | 公开 | P0 |
| /register | 注册 | 公开 | P0 |
| /dashboard | 仪表盘 | 登录 | P1 |
| /market | 行情列表 | 登录 | P0 |
| /market/:symbol | 交易对详情(K线) | 登录 | P0 |
| /strategies | 策略列表 | 登录 | P0 |
| /strategies/new | 创建策略 | 登录 | P0 |
| /strategies/:id | 策略详情 | 登录 | P0 |
| /backtest | 回测列表 | 登录 | P0 |
| /backtest/new | 新建回测 | 登录 | P0 |
| /backtest/:id | 回测结果 | 登录 | P0 |
| /trade | 交易/下单 | 登录 | P1 |
| /orders | 委托查询 | 登录 | P1 |
| /portfolio | 持仓 | 登录 | P1 |
| /portfolio/pnl | 盈亏分析 | 登录 | P1 |
| /portfolio/risk | 风险分析 | 登录 | P1 |
| /admin/users | 用户管理 | admin | P0 |
| /profile | 个人中心 | 登录 | P2 |

---

## 11. 用户故事地图

```
           发布 v1.0 (MVP)                    发布 v1.1                   发布 v2.0
           ──────────────────              ──────────────────         ──────────────────
 登入     注册 ─→ 登录 ─→ Token 鉴权
 行情     K线 + 指标叠加 ─→ 深度 ─→ Ticker
 策略     创建(代码编辑) ─→ 启停(模拟)
 回测     配置参数 ─→ 运行 ─→ 查看指标 ─→ 资金曲线
 权限     RBAC ─→ 管理后台              

                                        交易     手动下单 ─→ 委托管理
                                        持仓     持仓列表 ─→ 盈亏分析
                                        风控     VaR ─→ 预警
                                        仪表盘    总览面版

                                                                       导出     CSV/PDF 报告
                                                                       策略市场    模板共享
                                                                       密码重置    忘记密码
                                                                       多交易所    Binance/OKX
```

---

## 12. Glossary

| 术语 | 说明 |
|------|------|
| K线 (Candlestick) | 包含开盘/最高/最低/收盘价和成交量的时间周期柱状图 |
| Ticker | 交易对的最新行情概览（最新价、涨跌幅、成交量等） |
| Depth (深度) | 买卖盘口的挂单量分布，反映流动性 |
| 回测 (Backtest) | 使用历史数据模拟策略运行，评估策略表现 |
| 夏普比率 (Sharpe Ratio) | 衡量风险调整后收益的指标 |
| VaR (Value at Risk) | 在给定置信度下，一定持有期内的最大可能亏损 |
| 滑点 (Slippage) | 实际成交价与预期价的差异，由市场波动和流动性导致 |
| 撮合引擎 | 匹配买卖订单的机制，决定成交价格和数量 |
