# Contract-Review-AI-Quant

AI量化模块接口契约定义

## 1. Rust Backend → AI Model Service

### POST /predict

预测接口

**Request:**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "features": [
    [open, high, low, close, volume, ...],
    [open, high, low, close, volume, ...],
    ...
  ],
  "model_version": "v1.0"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对 |
| interval | string | 是 | K线周期，如 "1h", "4h", "1d" |
| features | array[array[float]] | 是 | 特征向量数组，每组对应一根K线 |
| model_version | string | 否 | 模型版本，默认最新 |

**Response:**
```json
{
  "direction": "long",
  "confidence": 0.75,
  "price_target": 68500.0,
  "model_version": "v1.0",
  "timestamp": "2026-05-22T19:00:00Z"
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| direction | string | "long" / "short" / "neutral" |
| confidence | float | 置信度 0.0 ~ 1.0 |
| price_target | float | 目标价格（可选） |
| model_version | string | 实际使用模型版本 |
| timestamp | string | 预测时间戳 |

**Error Response (5xx):**
```json
{
  "error": "model_service_unavailable",
  "message": "AI model service is temporarily unavailable",
  "fallback": "use_rule_based_signal"
}
```

---

### GET /health

服务健康检查

**Response:**
```json
{
  "status": "healthy",
  "model_loaded": true,
  "model_version": "v1.0",
  "uptime_seconds": 3600
}
```

---

### GET /models

列出可用模型

**Response:**
```json
{
  "models": [
    {
      "version": "v1.0",
      "created_at": "2026-05-01T00:00:00Z",
      "accuracy": 0.72,
      "status": "active"
    },
    {
      "version": "v0.9",
      "created_at": "2026-04-01T00:00:00Z",
      "accuracy": 0.68,
      "status": "deprecated"
    }
  ]
}
```

---

## 2. Frontend → Rust Backend

### GET /api/v1/ai/models

列出可用的AI模型

**Response:**
```json
{
  "models": [
    {
      "version": "v1.0",
      "name": "BTC Trend Predictor",
      "accuracy": 0.72,
      "status": "active"
    }
  ],
  "current_model": "v1.0"
}
```

---

### GET /api/v1/ai/predictions/{symbol}

获取指定交易对的最新AI预测

**Path Parameters:**
| 参数 | 类型 | 说明 |
|------|------|------|
| symbol | string | 交易对，如 "BTCUSDT" |

**Query Parameters:**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| interval | string | "1h" | K线周期 |

**Response:**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "prediction": {
    "direction": "long",
    "confidence": 0.75,
    "price_target": 68500.0,
    "model_version": "v1.0",
    "generated_at": "2026-05-22T19:00:00Z"
  },
  "rule_based_signal": {
    "direction": "long",
    "confidence": 0.60
  },
  "fused_signal": {
    "direction": "long",
    "confidence": 0.68
  }
}
```

---

### POST /api/v1/ai/backtest

AI信号回测

**Request:**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "start_time": "2026-01-01T00:00:00Z",
  "end_time": "2026-05-22T00:00:00Z",
  "initial_balance": 10000.0,
  "model_version": "v1.0"
}
```

**Response:**
```json
{
  "backtest_id": "bt_20260522_001",
  "symbol": "BTCUSDT",
  "interval": "1h",
  "period": {
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-05-22T00:00:00Z"
  },
  "results": {
    "total_trades": 45,
    "winning_trades": 28,
    "win_rate": 0.622,
    "total_pnl": 1250.50,
    "total_pnl_percent": 12.505,
    "max_drawdown": 0.08,
    "sharpe_ratio": 1.45
  },
  "model_version": "v1.0"
}
```

---

## 3. 内部信号融合规则

### signal_fusion.rs 融合逻辑

```
final_signal = weighted_average(ai_signal, rule_signal)
```

| AI置信度 | 规则置信度 | 融合权重(AI) | 融合权重(规则) |
|----------|------------|--------------|----------------|
| > 0.8 | any | 0.7 | 0.3 |
| 0.6-0.8 | > 0.7 | 0.6 | 0.4 |
| 0.6-0.8 | 0.5-0.7 | 0.5 | 0.5 |
| < 0.6 | any | 0.4 | 0.6 |

### 冲突处理
- 当AI信号与规则信号方向相反时，以规则信号为主
- AI置信度 < 0.4 时，完全使用规则信号
