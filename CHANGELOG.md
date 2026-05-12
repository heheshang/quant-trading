# Changelog

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
