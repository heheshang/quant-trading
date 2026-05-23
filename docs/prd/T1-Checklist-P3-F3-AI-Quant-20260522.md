# T1-Checklist-P3-F3-AI-Quant-20260522

## 功能名称
**P3-F3: AI量化模块**

## T0 通过时间
2026-05-22（130/130）

---

## T1 检查项：详细设计可执行

### ✅ F1 时序预测 — 技术方案

**Q1: feature_engine.rs 输入格式**

| 问题 | 答案 |
|------|------|
| K线数据字段 | `open, high, low, close, volume` 来自 `kline_data` 表 |
| 窗口大小 | 默认 100 根K线（可配置 `AI_FEATURE_WINDOW`） |
| 特征有哪些 | RSI, ATR, MA5/MA20, price_momentum, volume_ratio |
| 如何归一化 | Z-score 归一化（Rust 实现） |

**Q2: HTTP 客户端设计**

| 问题 | 答案 |
|------|------|
| 客户端库 | Rust `reqwest`（已有） |
| 超时配置 | 2s，超时 → 返回 `direction=neutral, prob=0.5` |
| 重试策略 | 不重试（AI信号为可选增强，延迟敏感） |
| 并发限制 | 最多 5 个并发请求（`Semaphore`） |

**Q3: 模型版本指定**

| 问题 | 答案 |
|------|------|
| 默认版本 | `AI_DEFAULT_MODEL_VERSION=v1` 环境变量 |
| 版本获取 | `GET /models` 启动时缓存到内存 |
| 降级 | 版本不可用 → 使用 `v1` |

**预估代码量**: `feature_engine.rs` ~150行 + `model_client.rs` ~200行

---

### ✅ F2 市场情绪识别 — 技术方案

**Q1: 订单簿数据来源**

| 问题 | 答案 |
|------|------|
| 数据来源 | `market_depth` 表（买卖盘口数据） |
| 层级数 | 取前 3 档（卖1-3，买1-3） |
| 频率 | 每 5 秒采样一次（可配置） |

**Q2: 情绪特征计算**

| 问题 | 答案 |
|------|------|
| 核心指标 | 订单簿不平衡度 = `(bid_vol - ask_vol) / (bid_vol + ask_vol)` |
| 买卖密度 | 每档价格量加权平均 |
| 情绪标签 | `>0.3` → bullish, `<-0.3` → bearish, else → neutral |
| 实现位置 | `feature_engine.rs` 的 `extract_orderbook_features()` |

**Q3: 融合策略**

| 问题 | 答案 |
|------|------|
| AI信号 × 情绪得分 | 加权平均，权重 `AI_SIGNAL_WEIGHT=0.6` |
| 最终信号 | `final_score = 0.6 * ai_prob + 0.4 * sentiment_score` |
| 方向判定 | `>0.55` → up, `<0.45` → down, else → neutral |

---

### ✅ F3 参数自动优化 — 技术方案

**Q1: 优化算法选择**

| 问题 | 答案 |
|------|------|
| 首选算法 | 贝叶斯优化（`optuna` Python side） |
| Rust 接口 | HTTP POST `/optimize/bayesian` → 返回最优参数 |
| 回测对接 | 调用现有 `backtest_engine.rs` 的 `run_backtest()` |

**Q2: 并行化方案**

| 问题 | 答案 |
|------|------|
| CPU 利用 | 4 核心并行（配置 `OPTIMIZATION_WORKERS=4`） |
| Rust 侧 | `rayon` 并行回测迭代 |
| Python 侧 | `joblib` 并行目标函数评估 |

**Q3: 边界约束处理**

| 问题 | 答案 |
|------|------|
| 边界定义 | JSON 配置：`{"rsi_period": [5, 50], "atr_period": [5, 100]}` |
| 越界处理 | 截断到边界值，不拒绝 |

---

### ✅ F4 模型版本管理 — 技术方案

**Q1: 数据库表设计**

```sql
-- model_versions
CREATE TABLE model_versions (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    version VARCHAR(16) NOT NULL UNIQUE,
    description TEXT,
    status VARCHAR(16) NOT NULL DEFAULT 'staged',  -- staged/active/deactivated
    metrics_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ai_signals
CREATE TABLE ai_signals (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    symbol VARCHAR(32) NOT NULL,
    direction VARCHAR(8),
    probability FLOAT,
    model_version VARCHAR(16),
    feature_snapshot JSONB
);

-- ab_experiment_logs
CREATE TABLE ab_experiment_logs (
    id BIGSERIAL PRIMARY KEY,
    experiment_id VARCHAR(64) NOT NULL,
    model_version VARCHAR(16) NOT NULL,
    order_id BIGINT,
    signal_strength FLOAT,
    decision_at TIMESTAMPTZ NOT NULL
);
```

**Q2: A/B 流量分配**

| 问题 | 答案 |
|------|------|
| 分配方式 | 内存 `HashMap<version, f64>` + 随机权重路由 |
| 动态更新 | `POST /admin/models/{version}/traffic` 实时生效 |
| 实验日志 | 每个决策打标 `order_id + model_version` |

**Q3: 灰度发布**

| 问题 | 答案 |
|------|------|
| 策略 | 每小时 +10% 流量 |
| 监控 | 亏损阈值 `>2%/小时` → 自动回滚 |
| 状态机 | `staged → active → deactivated` |

---

## ✅ 检查结果

**T1 通过 — 技术方案可执行**

所有子功能已完成详细技术设计：
- F1: `feature_engine.rs` + `model_client.rs` 接口明确
- F2: 订单簿特征提取 + 信号融合公式确定
- F3: 贝叶斯优化接口 + 并行化方案
- F4: 3张新表 + 内存流量分配 + 灰度状态机

> **T1 pass — 继续 T2 架构设计**

## 后续任务

| 任务 | 负责人 | 状态 |
|------|--------|------|
| T2: 架构设计 | ssk | ⏳ 进行中 |
| T3: 技术设计 | ssk | 待开始 |
| T4: 编码实现 | ssk | 待开始 |
