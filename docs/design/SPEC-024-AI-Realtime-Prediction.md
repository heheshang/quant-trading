# SPEC-024: AI 实时预测功能（行情页）

## 1. 概述与目标

**功能：** 在行情页（MarketView.vue）添加 AI 实时预测入口，调用 DeepSeek API 生成 BTCUSDT 1h 交易信号和分析。

**用户故事：**
- 作为交易者，我希望在行情页看到 AI 对当前 K 线的实时预测信号（多空/分析），以便辅助交易决策。
- 我可以手动触发 AI 分析，获取 DeepSeek 生成的交易建议。

---

## 2. 架构设计

```
[前端 MarketView]  ──REST──▶  [Python AI 服务:8002]
                                    │
                              DeepSeek API
                                    │
                           Binance WebSocket K线
                              (实时订阅)
```

**为什么不走 Rust 后端？**
- DeepSeek API 调用延迟高（~秒级），不适合同步阻塞 Rust 主线程
- Python 服务独立部署，更新模型无需重启 Rust 后端
- 已有 `model_client.rs` 定位为"轻量特征模型"，DeepSeek 是"分析型"服务

---

## 3. 目录结构

```
quant-trading/
├── ai-service/                          # Python AI 预测服务（新建）
│   ├── main.py                            # FastAPI 入口
│   ├── config.py                          # 配置（DeepSeek API Key 等）
│   ├── deepseek_client.py                 # DeepSeek API 调用
│   ├── binance_ws.py                      # Binance WebSocket K 线订阅
│   ├── features.py                        # 技术指标特征计算
│   ├── schemas.py                         # Pydantic 请求/响应模型
│   ├── requirements.txt                   # Python 依赖
│   └── .env.example                      # 环境变量示例
```

---

## 4. API 设计

### 4.1 Python AI 服务（port 8002）

#### `GET /health`
健康检查。

**响应：**
```json
{ "status": "ok", "deepseek_connected": true }
```

#### `GET /api/v1/klines/{symbol}?interval=1h&limit=100`
获取历史 K 线（用于特征提取）。

**响应：**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "klines": [
    { "open_time": 1717200000000, "open": "70000", "high": "71000", "low": "69500", "close": "70500", "volume": "1234" }
  ]
}
```

#### `POST /api/v1/predict`
手动触发 AI 预测（使用 DeepSeek）。

**请求：**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "klines": [...]  // 可选，不传则自动拉取最新
}
```

**响应：**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "direction": "long",
  "confidence": 0.75,
  "signal": "strong_buy",
  "analysis": "根据技术指标和宏观分析，BTC 在 1h 级别呈现上升趋势...",
  "price_target": 71500,
  "generated_at": "2026-05-31T12:00:00Z"
}
```

#### `WebSocket /ws/predict/{symbol}`
实时订阅 AI 预测信号（新 K 线触发 + 实时推送）。

**服务端推送：**
```json
{
  "type": "prediction",
  "symbol": "BTCUSDT",
  "interval": "1h",
  "direction": "long",
  "confidence": 0.78,
  "signal": "buy",
  "analysis": "...",
  "price_target": 71200,
  "kline_close_time": 1717200000000,
  "generated_at": "2026-05-31T12:00:00Z"
}
```

### 4.2 前端 → Python 服务（直连）

前端直接调用 Python 服务（port 8002），绕过 Rust 后端：
- 避免 Rust 转发延迟
- 减少 Rust 后端复杂度
- Python 服务独立维护

**CORS 配置：** 允许 `http://localhost:5173` 和 `http://localhost:8081`

---

## 5. 前端设计（MarketView.vue）

### 5.1 新增 AI 预测面板

**位置：** MarketView 右侧栏或底部抽屉

**UI 元素：**
- **实时信号卡片** — 显示当前 AI 信号（多/空/中性）、置信度、分析摘要
- **手动预测按钮** — 「AI 分析」按钮，点击触发 DeepSeek 预测
- **信号历史** — 最近 5 条预测记录（可选）

### 5.2 状态定义

| 信号 | 颜色 | 图标 |
|------|------|------|
| strong_buy | `#10B981` 绿 | ↑↑ |
| buy | `#34D399` 浅绿 | ↑ |
| neutral | `#6B7280` 灰 | — |
| sell | `#F87171` 浅红 | ↓ |
| strong_sell | `#EF4444` 红 | ↓↓ |

### 5.3 DeepSeek Prompt 策略

```
你是一个专业的量化交易分析师。请分析以下 BTCUSDT 1h K线数据，
给出交易方向、置信度（0-1）、目标价格和简明分析。
只返回结构化的 JSON 格式。
```

---

## 6. 技术指标特征

Python 服务在调用 DeepSeek 前，计算以下技术指标作为参考（不直接预测价格）：

| 指标 | 说明 |
|------|------|
| RSI(14) | 相对强弱指数 |
| MACD | 快线、慢线、柱状图 |
| MA(7,25) | 移动平均线 |
| Bollinger Bands | 布林带 |
| Volume MA | 成交量均线 |
| ATR | 平均真实波幅 |

这些指标作为 context 注入 DeepSeek prompt。

---

## 7. 环境变量

```env
# DeepSeek API
DEEPSEEK_API_KEY=sk-xxxxx
DEEPSEEK_BASE_URL=https://api.deepseek.com

# Binance
BINANCE_WS_URL=wss://stream.binance.com:9443/ws

# Server
AI_SERVICE_PORT=8002
```

---

## 8. 实现计划

| 阶段 | 内容 | 依赖 |
|------|------|------|
| **T0** | SPEC.md 评审确认 | — |
| **T1** | Python AI 服务核心（FastAPI + Binance WS + 技术指标） | T0 |
| **T2** | DeepSeek 集成（predict endpoint + WebSocket 推送） | T1 |
| **T3** | 前端 MarketView AI 面板 | T2 |
| **T4** | 端到端测试 + 修复 | T3 |

---

## 9. 验证命令

```bash
# 启动 Python AI 服务
cd ai-service && uvicorn main:app --port 8002 --reload

# 健康检查
curl http://localhost:8002/health

# 手动预测
curl -X POST http://localhost:8002/api/v1/predict \
  -H "Content-Type: application/json" \
  -d '{"symbol":"BTCUSDT","interval":"1h"}'

# WebSocket 测试
wscat -c ws://localhost:8002/ws/predict/BTCUSDT
```

---

## 10. 风险与缓解

| 风险 | 缓解 |
|------|------|
| DeepSeek API 延迟高 | 异步调用，超时 30s，返回降级信号 |
| API Key 暴露 | 仅存在 Python 服务本地，不传前端 |
| Binance WS 断开 | 自动重连 + 断线提示 |