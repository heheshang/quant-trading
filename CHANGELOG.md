# Changelog

## v0.3.0 (2026-05-13)

### 新增: 回测引擎模块

**回测引擎核心:**
- 全内存向量化回测引擎（O(n) 单次遍历 K 线数据）
- 异步执行：Semaphore 并发控制（上限 5 个同时运行）
- CancellationToken 取消支持（每 100 根 K 线检查）
- AtomicU32 实时进度追踪（每 ~10% 更新）
- 双向交易模拟：做多 (long) + 做空 (short)
- 真实成本模拟：手续费率 + 滑点率
- 爆仓检测：权益归零自动终止
- 合成 K 线数据生成（无真实数据时用于演示/测试）
- 权益曲线均匀采样（上限 2000 点，保留首尾）

**绩效指标 (14 项):**
- 总收益率、年化收益率、最大回撤
- Sharpe 比率、Sortino 比率、Calmar 比率
- 胜率、总交易次数、盈亏比
- 平均盈利、平均亏损、平均交易收益
- 总手续费、总滑点成本

**回测 API 端点 (7 个):**
- `POST /api/v1/backtest` — 运行回测（异步）
- `GET /api/v1/backtest/{id}` — 获取回测完整结果
- `GET /api/v1/backtest/{id}/trades` — 获取交易明细
- `GET /api/v1/backtest/{id}/equity` — 获取权益曲线
- `GET /api/v1/backtest/history` — 回测历史列表（分页）
- `DELETE /api/v1/backtest/{id}` — 删除回测记录
- `POST /api/v1/backtest/{id}/cancel` — 取消运行中回测

**文档:**
- 回测引擎 API 文档（7 个端点完整说明 + curl 示例 + 错误码）
- 回测引擎使用说明（流程 + 交易模拟 + 最佳实践 + FAQ）
- 绩效指标定义（14 项指标公式详解 + 参考标准）
- README 回测模块说明更新

## v0.2.0 (2026-05-13)

### 新增: 策略管理模块

**策略模板引擎:**
- 10 个预定义策略模板，覆盖趋势/均值回归/波动率/复合四大类别
- 统一的 `StrategyTemplate` trait，支持模板注册与按 ID 查询
- 每个模板包含独立参数 schema（`ParameterDef`: 名/类型/范围/默认值）
- 模板级参数校验（边界检查 + 业务约束如 `fast < slow`）
- MA Crossover、Triple MA、MACD、Bollinger Bands、RSI、Keltner Channels、ATR Stop Loss、Mean Reversion、Ichimoku Cloud、Double Bollinger

**策略 CRUD 端点 (7 个):**
- `GET /api/v1/strategies/templates` — 策略模板列表
- `POST /api/v1/strategies` — 创建策略（基于模板 + 参数校验）
- `GET /api/v1/strategies` — 分页查询用户策略列表
- `GET /api/v1/strategies/{id}` — 获取策略详情
- `PUT /api/v1/strategies/{id}` — 更新策略名称/参数
- `DELETE /api/v1/strategies/{id}` — 删除策略
- `POST /api/v1/strategies/{id}/status` — 策略状态流转

**状态机:**
- 四态流转: draft → active → paused → stopped
- 结构化校验拦截非法转换
- 所有端点绑定额外的所有权校验（`user_id` 匹配）

**文档:**
- 策略管理 API 文档（7 个端点完整说明 + curl 示例 + 错误码）
- 策略用户指南（模板详解 + 参数配置最佳实践 + 常见问题）
- README 策略模块说明更新
