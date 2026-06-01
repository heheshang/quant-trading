# T3 — 设计文档

## 3.1 模块结构

```
backend/src/
├── metrics.rs                       # 新增：全局 Registry + 指标定义
├── handlers/
│   ├── metrics_handler.rs           # 新增：GET /metrics handler
│   └── mod.rs                       # 修改：+ pub mod metrics_handler
├── lib.rs                           # 修改：+ pub mod metrics
└── main.rs                          # 修改：+ .route("/metrics", ...)
```

## 3.2 指标清单

### HTTP 层
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `http_requests_total` | IntCounterVec | method/route/status | QPS / 状态码分布 |
| `http_request_duration_seconds` | HistogramVec | method/route | 延迟分位 |

### 订单 / 撮合
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `orders_created_total` | IntCounterVec | side/type/exchange | 下单量 |
| `orders_filled_total` | IntCounterVec | side/exchange | 成交单数 |
| `orders_cancelled_total` | IntCounterVec | reason/exchange | 撤单量 |
| `positions_open` | IntGaugeVec | exchange/symbol | 当前持仓数 |

### 触发单 / 风控
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `trigger_orders_created_total` | IntCounterVec | type | 触发单创建量 |
| `trigger_orders_fired_total` | IntCounterVec | type | 触发单触发量 |
| `risk_rules_tripped_total` | IntCounterVec | rule | 风控触发次数 |
| `rate_limit_denied_total` | IntCounterVec | scope | 限流拒绝次数 |

### WebSocket / 行情
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `ws_connections_active` | IntGaugeVec | client_type | WS 在线连接 |
| `ws_messages_broadcast_total` | IntCounterVec | message_type | 广播消息量 |
| `kline_persist_total` | IntCounterVec | interval | K线持久化数 |

### AI Service
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `ai_predictions_total` | IntCounterVec | symbol | AI 预测请求量 |

### 进程元
| 指标 | 类型 | Labels | 用途 |
|---|---|---|---|
| `build_info` | IntGaugeVec | version | 构建版本（恒为 1） |

## 3.3 API 契约

### `GET /metrics`
- **Response 200**：`Content-Type: text/plain; version=0.0.4; charset=utf-8`
- **Body**：Prometheus text-exposition 格式
- **不需要鉴权**（运维端点，假设已在内网）

### 实现策略
1. **`encode_text()`** 全局辅助 → 编码 + 返回 (body, content_type)
2. **handler** 调 `encode_text()` 包装成 `Response`
3. **业务埋点**留待后续 P0-2/P0-3 任务

## 3.4 错误处理

- Registry gather 失败 → 返回空 body + 200（metrics 不阻塞业务）
- encoder 失败 → 同上（已用 `is_err()` 检测 + `buf.clear()`）
- **不返回 5xx**：metrics endpoint 失败时 Grafana 报警但 backend 仍工作

---

**T3 设计完整，T4 已落地实现**。
