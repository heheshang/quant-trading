# ADR-022: AI量化模块架构

## Status: Proposed

## Context: Rust后端 + 外部Python模型服务的架构选择

在构建AI量化交易模块时，我们需要选择合适的架构来处理：
- 特征工程计算
- 机器学习模型训练与推理
- 与现有Rust后端系统的集成

### 方案A：Rust特征工程 + 外部Python模型服务（HTTP REST）

**架构：**
```
[Rust Backend] --> [Python Model Service (FastAPI + PyTorch)]
       |                        |
  feature_engine.rs      /predict
  signal_fusion.rs       /health
```

**优点：**
- 不污染Rust生态，保持后端简洁
- 利用Python成熟ML生态（PyTorch/TensorFlow）进行模型训练
- 模型可独立部署、迭代和扩展
- Rust后端专注于业务逻辑和高性能计算

**缺点：**
- 增加网络延迟（HTTP调用）
- 部署复杂度增加（需维护Python服务）
- 需要定义清晰的接口契约

### 方案B：Candle（Rust ML框架）

**优点：**
- 纯Rust实现，无外部依赖
- 减少部署复杂度

**缺点：**
- 生态不成熟
- 模型支持有限
- 不适合复杂模型训练

## Decision: 采用外部Python模型服务 + Rust HTTP客户端

综合考虑ML模型训练需求和系统可维护性，选择方案A：
- 特征工程使用Rust实现（高性能、纯计算）
- 模型服务使用Python + FastAPI + PyTorch（灵活训练）
- 通过HTTP REST进行通信

## Implementation:

### Rust Backend: `backend/src/services/ai/`

```
model_client.rs      - HTTP客户端，调用远程模型服务
feature_engine.rs   - K线数据 → 特征向量（Rust实现，纯计算）
signal_fusion.rs    - AI信号与规则信号加权融合
```

#### model_client.rs
- 封装HTTP POST请求到Python模型服务
- 处理请求超时、重试逻辑
- 解析JSON响应
- 模型版本管理

#### feature_engine.rs
- OHLCV数据转换为特征向量
- 技术指标计算（MA, RSI, MACD, Bollinger Bands等）
- 时间序列特征提取
- 归一化处理

#### signal_fusion.rs
- AI信号与规则信号的加权融合
- 置信度计算
- 多信号冲突处理

### Python模型服务（独立部署）

```
ai_model_service/
├── main.py           - FastAPI应用
├── model.py          - PyTorch模型定义
├── predict.py        - 推理逻辑
└── requirements.txt  - Python依赖
```

#### API Endpoints

**POST /predict**
- 输入：特征向量、symbol、interval、model_version
- 输出：direction、confidence、price_target

**GET /health**
- 服务健康检查

**GET /models**
- 列出可用模型版本

## Consequences

### Positive
- 利用成熟Python ML生态
- 前后端解耦，便于独立迭代
- 特征计算高性能

### Negative
- 引入网络延迟（~10-50ms）
- 需要维护额外的Python服务
- 接口契约必须严格遵守

### Risks & Mitigations
- **风险**：Python服务宕机 → **缓解**：熔断器模式 + 降级策略
- **风险**：网络延迟影响交易 → **缓解**：本地缓存 + 异步调用
- **风险**：模型版本不一致 → **缓解**：版本协商 + 健康检查
