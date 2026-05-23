# ADR-015: P3-F3 AI量化模块架构

> **状态**: Proposed
> **日期**: 2026-05-22
> **决策者**: ssk

---

## 背景

量化交易系统当前无 ML 基础设施。P3-F3 AI量化模块面向高级量化研究员，提供时序预测、市场情绪识别、参数自动优化和模型版本管理能力。

**核心约束**：
- Rust 后端（无 Python 运行时）
- AI 模型由外部 Python 服务提供（Docker 独立部署）
- Rust 特征工程可在无 AI 服务时独立运行（降级策略）

---

## 决策

### 1. 整体架构：Rust + 外部 Python 模型服务

```
┌─────────────────────────────────────────────────────────┐
│                   Rust Backend (Axum)                    │
│  ┌────────────────────────────────────────────────────┐ │
│  │            services/ai/ ML Interface Layer          │ │
│  │  ┌─────────────────┐  ┌──────────────────────────┐ │ │
│  │  │ feature_engine  │  │      model_client        │ │ │
│  │  │  (Rust本地计算)  │  │   (HTTP → Python服务)   │ │ │
│  │  └─────────────────┘  └──────────────────────────┘ │ │
│  │  ┌─────────────────┐  ┌──────────────────────────┐ │ │
│  │  │  signal_fusion  │  │    model_version_mgr     │ │ │
│  │  │ (AI+规则融合)    │  │    (内存流量分配)         │ │ │
│  │  └─────────────────┘  └──────────────────────────┘ │ │
│  └────────────────────────────────────────────────────┘ │
│              ↑ 降级: AI不可用 → neutral signal           │
└─────────────────────────────────────────────────────────┘
                          ↕ HTTP/JSON
┌─────────────────────────────────────────────────────────┐
│        Python Model Service (Docker独立部署)             │
│  ┌────────────────────────────────────────────────────┐ │
│  │  LSTM/Transformer   │  贝叶斯优化(optuna)        │ │
│  │  price_direction     │  /optimize/bayesian        │ │
│  └────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Faiss向量索引  │  Candle(ML框架)  │  FastAPI     │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 2. Rust 服务模块结构

```
backend/src/services/ai/
├── mod.rs               # 模块入口，导出 public API
├── feature_engine.rs    # K线特征 + 订单簿特征提取（纯Rust）
├── model_client.rs      # HTTP推理客户端（reqwest）
├── signal_fusion.rs     # AI信号 + 规则信号加权融合
└── model_version_mgr.rs # 模型版本管理 + A/B流量分配
```

### 3. 新增 Handler（REST API）

```
POST /ai/predict/price_direction   # 时序预测
POST /ai/predict/sentiment          # 市场情绪识别
POST /ai/optimize/bayesian          # 贝叶斯优化
GET  /ai/models                     # 列出模型版本
POST /ai/models/{version}/traffic   # 设置A/B流量
GET  /ai/experiments/{id}/results   # A/B实验结果
```

### 4. 新增数据库表

| 表名 | 用途 |
|------|------|
| `model_versions` | 模型版本元数据 + 状态机 |
| `ai_signals` | AI信号记录（历史追溯） |
| `ab_experiment_logs` | A/B实验决策日志 |

### 5. 降级策略

| 场景 | 行为 |
|------|------|
| Python 服务不可达 | 超时2s → 返回 `neutral` (prob=0.5) |
| 模型版本不存在 | fallback 到 `v1` |
| K线数据 < 100根 | 返回错误码 `AI-001` |
| 订单簿数据缺失 | 返回错误码 `AI-002` |

### 6. 融合策略

```rust
// AI信号权重配置
const AI_SIGNAL_WEIGHT: f64 = 0.6;  // 环境变量 AI_SIGNAL_WEIGHT

// 最终得分
let final_score = AI_SIGNAL_WEIGHT * ai_prob + (1.0 - AI_SIGNAL_WEIGHT) * sentiment_score;

// 方向判定
let direction = if final_score > 0.55 { "up" }
                else if final_score < 0.45 { "down" }
                else { "neutral" };
```

---

## 候选方案考虑

### 方案A（采用）：外部 Python 服务 + Rust HTTP客户端
- ✅ Rust 后端无需引入 Python 运行时
- ✅ Python ML 生态完整（LSTM/Transformer/optuna）
- ✅ AI 服务独立迭代，不影响主系统
- ❌ 网络延迟（需降级策略）
- ❌ 额外部署成本

### 方案B（拒绝）：Rust 原生 ML（RustNN/candle）
- ❌ Rust ML 库生态不成熟
- ❌ LSTM/Transformer 实现成本高
- ❌ 与现有 Python 研究流程集成困难

### 方案C（拒绝）：嵌入式 Python（PyO3）
- ❌ 增加部署复杂度
- ❌ 版本冲突风险
- ❌ GIL 并发限制

---

## 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `reqwest` | 0.12 | HTTP 客户端（已有） |
| `serde` | 1.0 | JSON 序列化（已有） |
| `tokio` | 1.0 | 异步运行时（已有） |
| Python Docker | 3.11+ | 模型服务 |

---

## 后果

### 正面
- AI 信号增强现有规则策略，不破坏现有稳定性
- Rust 后端保持轻量，无需 ML 运行时
- 独立 Docker 部署，故障隔离

### 负面
- 引入外部服务依赖（需监控）
- 网络延迟需要处理（降级已覆盖）

### 待定
- Python 服务 Dockerfile 路径（待 T3 确定）
- AI_MODEL_SERVICE_URL 配置位置（`.env` 或 `config.yaml`）
