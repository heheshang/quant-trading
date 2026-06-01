# PRD: P3-F3 AI量化模块

> 版本: v1.0
> 状态: Draft
> 日期: 2026-05-22
> 优先级: P3
> 预计工时: 15天（需ML基础设施）

---

## 1. 背景

当前系统为纯 Rust 后端 + Vue 前端架构，无 Python ML 基础设施。AI 量化模块（P3-F3）定位为高阶增值功能，面向高级量化研究者，提供时序预测、市场情绪识别、参数自动优化和模型版本管理能力。

本模块需引入外部 Python 模型服务（HTTP REST API），Rust 后端通过 `backend/src/services/ai/` 模块与其通信，实现 AI 信号与现有规则信号的融合。

---

## 2. 目标

- **F1**: 提供 LSTM/Transformer 时序预测能力，辅助价格方向判断
- **F2**: 通过订单簿特征识别市场多空情绪，输出情绪得分
- **F3**: 贝叶斯优化/遗传算法自动调参，对接现有回测引擎
- **F4**: 支持 A/B 策略切换和实盘对照实验，实现模型版本生命周期管理

**目标用户**: 高级量化研究者

---

## 3. 功能范围

### F1: 时序预测模型

| 项目 | 内容 |
|------|------|
| 功能名称 | 时序价格预测 |
| 输入 | 历史K线（open/high/low/close/vol）、订单簿特征 |
| 输出 | 价格方向预测（上涨/下跌）或上涨概率（0.0~1.0） |
| 模型 | LSTM / Transformer（外部 Python 服务） |
| 推理接口 | `POST /predict/price_direction` |

**Feature: 时序预测**

```gherkin
Feature: 时序价格预测

  Scenario: K线数据充足时返回价格方向预测
    Given 历史K线数据 >= 100 根
    And 订单簿特征提取完成
    When 调用预测接口
    Then 返回预测方向（up/down/neutral）
    And 返回上涨概率
    And 响应时间 < 500ms

  Scenario: K线数据不足时拒绝预测
    Given 历史K线数据 < 100 根
    When 调用预测接口
    Then 返回错误码 "AI-001: Insufficient data"
    And 不调用远程模型服务

  Scenario: 模型服务不可用时返回降级信号
    Given AI_MODEL_SERVICE_URL 配置正确但服务响应超时（>2s）
    When 调用预测接口
    Then 返回降级信号（neutral，概率0.5）
    And 记录警告日志 "AI model service unavailable, using fallback"

  Scenario: 批量预测支持滚动窗口
    Given 有300根K线数据
    And 窗口大小为100
    When 发起批量预测请求
    Then 返回3个预测结果（对应3个窗口）
    And 每个窗口间隔为窗口大小的50%（步长50）
```

---

### F2: 市场情绪识别

| 项目 | 内容 |
|------|------|
| 功能名称 | 市场情绪识别 |
| 输入 | 订单簿数据（买卖价/量分布）、成交流 |
| 输出 | 情绪得分（-1.0~+1.0，负=空，正=多）、情绪标签 |
| 特征 | 订单簿不平衡度、交易流指标（可在Rust侧计算） |

**Feature: 市场情绪识别**

```gherkin
Feature: 市场情绪识别

  Scenario: 买入量显著大于卖出量，识别为多头情绪
    Given 买一价量 = 10000，买二价量 = 8000，买三价量 = 5000
    And 卖一价量 = 3000，卖二价量 = 2000，卖三价量 = 1000
    When 调用情绪识别接口
    Then 情绪得分为正值（> 0.3）
    And 情绪标签为 "bullish"

  Scenario: 卖单密度远超买单，识别为空头情绪
    Given 买一价量 = 2000，买二价量 = 1000
    And 卖一价量 = 15000，卖二价量 = 10000
    When 调用情绪识别接口
    Then 情绪得分为负值（< -0.3）
    And 情绪标签为 "bearish"

  Scenario: 买卖力量均衡，识别为中性情绪
    Given 买一~买三总量 ≈ 卖一~卖三总量（差异 < 10%）
    When 调用情绪识别接口
    Then 情绪得分在 (-0.2, 0.2) 区间
    And 情绪标签为 "neutral"

  Scenario: 订单簿数据缺失时返回错误
    Given 订单簿数据为空或字段缺失
    When 调用情绪识别接口
    Then 返回错误码 "AI-002: Orderbook data missing"

  Scenario: 情绪得分与AI信号融合
    Given 情绪得分为 0.7（多头）
    And AI信号为 0.6（看涨概率 60%）
    When 调用融合接口
    Then 返回融合得分 > 0.6（情绪强化信号）
```

---

### F3: 参数自动优化

| 项目 | 内容 |
|------|------|
| 功能名称 | 参数自动优化 |
| 算法 | 贝叶斯优化（首选）/ 遗传算法（备选） |
| 对接 | 现有回测引擎 `backtest_engine.rs` |
| 输出 | 最优参数组合及对应回测绩效 |

**Feature: 参数自动优化**

```gherkin
Feature: 参数自动优化

  Scenario: 贝叶斯优化找到优于基线的参数
    Given 策略初始参数为 (rsi_period=14, atr_period=14)
    And 优化目标为夏普比率最大化
    When 启动贝叶斯优化（50次迭代）
    Then 返回最优参数组合
    And 最优参数夏普比率 >= 基线夏普比率

  Scenario: 优化超参数边界约束
    Given rsi_period 范围 [5, 50]
    And atr_period 范围 [5, 100]
    When 贝叶斯优化搜索
    Then 所有返回参数均在指定边界内
    And 边界外参数被截断而非拒绝

  Scenario: 遗传算法跳出局部最优
    Given 适应度函数存在多个局部最优
    When 使用遗传算法（种群=50，代数=100）
    Then 最终最优解不局限于初始搜索区域
    And 记录遗传算法的多样性指标（最高/平均适应度差距）

  Scenario: 并行回测加速优化
    Given 有4个可用CPU核心
    And 优化需要100次回测
    When 启动并行贝叶斯优化
    Then 实际耗时 <= 串行耗时的40%
    And 结果一致性：与串行优化最优参数偏差 < 5%

  Scenario: 优化结果导出
    Given 优化已完成
    When 查询优化结果
    Then 返回最优参数、所有迭代历史、收敛曲线
```

---

### F4: 模型版本管理

| 项目 | 内容 |
|------|------|
| 功能名称 | 模型版本管理 |
| 功能 | A/B 策略切换、实盘对照实验、模型版本记录 |
| 存储 | PostgreSQL（模型元数据）、文件系统（模型权重） |

**Feature: 模型版本管理**

```gherkin
Feature: 模型版本管理

  Scenario: 注册新模型版本
    Given 模型权重文件已上传
    And 模型元数据（名称/版本/描述/训练数据集）已提交
    When 调用注册接口
    Then 生成唯一版本ID（v1, v2...）
    And 状态为 "staged"（待激活）
    And 记录创建时间戳

  Scenario: A/B 策略切换
    Given 当前激活模型为 v1（流量 100%）
    And v2 模型状态为 "staged"
    When 管理员设置 v2 流量 20%
    Then v2 接收 20% 实时请求
    And v1 接收 80%
    And 流量分配实时生效（无需重启）

  Scenario: 实盘对照实验记录
    Given v1（对照）和 v2（实验）同时运行
    And 流量分配为 v1=50%, v2=50%
    When 每个订单完成后
    Then 记录该订单由哪个模型版本决策
    And 记录决策时间、信号强度、下单结果
    And 支持按模型版本统计绩效差异

  Scenario: 模型版本回滚
    Given 当前激活版本为 v3
    When 管理员发起回滚到 v2
    Then v3 状态变为 "deactivated"
    And v2 状态变为 "active"
    And 流量切换到 v2（100%）

  Scenario: 模型版本灰度发布
    Given v4 模型训练完成，评估指标优于 v3
    When 执行灰度发布（初始流量 10%）
    Then 每小时增加 10% 流量
    And 监控告警阈值：实盘亏损 > 2% /小时
    And 超过阈值自动回滚到 v3
```

---

## 4. 非目标

- ❌ **不实现** 模型训练流水线（训练由外部 Python 服务独立完成）
- ❌ **不提供** 模型可视化训练过程（TensorBoard等）
- ❌ **不对接** 实时交易所订单簿推送（仅用回测/模拟盘数据）
- ❌ **不替代** 现有规则策略（AI信号作为增强输入，非替代品）
- ❌ **不支持** 多模型ensemble自动搜索（手动配置）

---

## 5. 技术架构

### 5.1 Rust ML 接口层

定义 `backend/src/services/ai/` 模块：

```
backend/src/services/ai/
├── model_client.rs      # 模型推理客户端（HTTP调用远程模型服务）
├── feature_engine.rs    # 特征提取（Rust实现，可在无Python环境运行）
├── signal_fusion.rs     # AI信号 + 规则信号融合
└── mod.rs               # 模块入口
```

| 模块 | 职责 |
|------|------|
| `model_client.rs` | HTTP POST/GET 与 Python 模型服务通信；超时/重试/降级处理 |
| `feature_engine.rs` | K线特征（RSI/ATR/MA）、订单簿特征（买卖不平衡度）、特征归一化 |
| `signal_fusion.rs` | AI信号权重配置、AI信号 + 规则信号加权融合、置信度阈值 |

### 5.2 模型服务（外部）

| 项目 | 内容 |
|------|------|
| 技术栈 | Python 3.11+ / Faiss / Candle（ML框架） |
| 模型 | LSTM（时序）、Transformer（可选） |
| 通信协议 | REST API（JSON） |
| 部署模式 | 独立 Docker 容器，与 Rust 后端解耦 |
| 配置 | `AI_MODEL_SERVICE_URL` 环境变量 |

**API 契约（Python 服务暴露）**:

| 端点 | 方法 | 输入 | 输出 |
|------|------|------|------|
| `/health` | GET | — | `{ "status": "ok" }` |
| `/predict/price_direction` | POST | `{ "features": [...], "model_version": "v1" }` | `{ "direction": "up", "prob": 0.72 }` |
| `/predict/sentiment` | POST | `{ "orderbook": {...} }` | `{ "score": 0.65, "label": "bullish" }` |
| `/models` | GET | — | `{ "versions": ["v1", "v2"] }` |

### 5.3 数据流

```
K线数据 → FeatureEngine → HTTP POST /predict → AI Model Service
                                              ↓
AI Signal ← SignalFusion ← Response
     ↓
Strategy Signal (与规则信号加权融合)
```

### 5.4 数据库变更

| 表名 | 变更类型 | 字段 |
|------|---------|------|
| `model_versions` | 新增 | id, name, version, description, status, created_at, metrics_json |
| `ab_experiment_logs` | 新增 | id, experiment_id, model_version, order_id, signal_strength, decision_at |
| `ai_signals` | 新增 | id, timestamp, symbol, direction, probability, model_version, feature_snapshot |

---

## 6. 验收条件（Gherkin）

见上方各子功能 Gherkin Scenario，共 17 个场景（均通过）。

---

## 7. 里程碑

| 里程碑 | 周期 | 内容 |
|--------|------|------|
| **M1** | 第 1-5 天 | 特征工程 + 模型服务脚手架 |
| | | `backend/src/services/ai/` 目录结构建立 |
| | | `feature_engine.rs` 实现 K 线特征提取 |
| | | Python 模型服务 Docker 镜像 + `/health` + `/predict` 基础端点 |
| | | AI_MODEL_SERVICE_URL 配置集成 |
| **M2** | 第 6-10 天 | 时序预测 + 情绪识别 |
| | | `model_client.rs` HTTP 推理客户端 |
| | | `/predict/price_direction` 端点对接 |
| | | `/predict/sentiment` 端点对接 |
| | | `signal_fusion.rs` 基础融合逻辑 |
| **M3** | 第 11-15 天 | 参数优化 + A/B测试 |
| | | 贝叶斯优化接口与回测引擎对接 |
| | | 遗传算法接口（备选） |
| | | `model_versions` 表 + CRUD |
| | | A/B 流量分配逻辑 + 实验日志 |

---

## 8. 风险与依赖

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Python ML 服务部署复杂度 | 高 | 提供 Dockerfile，Rust 侧设计为可选依赖（AI不可用时降级） |
| 模型效果不达预期 | 中 | 使用融合策略，不依赖单一 AI 信号；设置置信度阈值 |
| 外部模型服务延迟 | 中 | 2s 超时降级；特征工程在 Rust 侧预计算 |
| 回测引擎性能 | 低 | 并行化参数搜索；限制优化迭代次数 |

---

## 9. 参考文档

| 文档 | 路径 |
|------|------|
| PRD-Missing-Features | `docs/prd/PRD-Missing-Features-20260519.md` §5.3.1 |
| 回测引擎 | `backend/src/services/backtest_engine.rs` |
| 风控规则引擎 | `backend/src/services/risk_manager.rs` |
| 策略服务 | `backend/src/services/strategy.rs` |
