# T3 — 设计文档

## 3.1 布局 (24-grid)

```
y=0:  [Row: 📊 Overview]                                   (1×24)
y=1:  [HTTP Request Rate] [HTTP P50/P95/P99]              (8×12 each)
y=9:  [Row: 💼 Order Flow]                                (1×24)
y=10: [Orders Created]    [Orders Cancelled] [Orders Filled] (8×8 each)
y=18: [Row: 🛡️ Risk & Limits]                              (1×24)
y=19: [Risk Stat] [Risk Trips Rate]            [Rate Limit] (4×6, 8×12, 8×6)
y=27: [Row: 🔌 WebSocket Realtime]                          (1×24)
y=28: [WS Active Stat]   [WS Broadcast Rate]              (4×6, 8×18)
y=36: [Row: 🤖 AI & Data Pipeline]                          (1×24)
y=37: [AI Donut] [Kline] [Trigger Created]                 (8×8 each)
y=45: [Trigger Fired] [Build Info]                        (8×8, 4×6)
```

**Total**: 5 rows + 15 data panels (20 元素) — 标准 24-grid 不重叠。

## 3.2 Panel ID 编号

| ID | Panel | 类别 |
|---|---|---|
| 100 | 📊 Overview row | divider |
| 1 | HTTP Request Rate | timeseries |
| 2 | HTTP P50/P95/P99 | timeseries |
| 200 | 💼 Order Flow row | divider |
| 3 | Orders Created | timeseries |
| 4 | Orders Cancelled | timeseries |
| 5 | Orders Filled | timeseries |
| 300 | 🛡️ Risk & Limits row | divider |
| 6 | Risk Stat | stat |
| 7 | Risk Trips Rate | timeseries |
| 8 | Rate Limit Denied | timeseries |
| 400 | 🔌 WebSocket Realtime row | divider |
| 9 | WS Active Stat | stat |
| 10 | WS Broadcast Rate | timeseries |
| 500 | 🤖 AI & Data Pipeline row | divider |
| 11 | AI Distribution | piechart |
| 12 | Kline Throughput | timeseries |
| 13 | Trigger Created | timeseries |
| 14 | Trigger Fired | timeseries |
| 15 | Build Info | stat |

## 3.3 Query 表达式映射表

| Panel | PromQL | 注解 |
|---|---|---|
| 1 HTTP Request Rate | `sum by (route) (rate(http_requests_total[1m]))` | 路由维度 |
| 2 HTTP P95 | `histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[5m])))` | histogram 标准做法 |
| 3 Orders Created | `sum by (side, order_type) (rate(orders_created_total[1m]) * 60)` | ×60 转 per-minute |
| 4 Orders Cancelled | `sum by (actor) (rate(orders_cancelled_total[1m]) * 60)` | actor=user/risk |
| 5 Orders Filled | `sum by (side) (rate(orders_filled_total[1m]) * 60)` | qty/min (不是 count/min) |
| 6 Risk Stat | `sum(risk_rules_tripped_total)` | 绝对数（不是 rate） |
| 7 Risk Trips Rate | `sum by (rule) (rate(risk_rules_tripped_total[5m]))` | 5m 窗口防 0 |
| 8 Rate Limit Denied | `sum by (scope) (rate(rate_limit_denied_total[1m]) * 60)` | per-minute |
| 9 WS Active | `sum(ws_connections_active)` | gauge 不接 rate |
| 10 WS Broadcast Rate | `sum by (kind) (rate(ws_messages_broadcast_total[1m]))` | 6 variants |
| 11 AI Donut | `sum by (direction) (ai_predictions_total)` | absolute count |
| 12 Kline Throughput | `sum by (exchange) (rate(kline_persist_total[1m]) * 60)` | rows/min |
| 13 Trigger Created | `sum by (trigger_type) (rate(trigger_orders_created_total[1m]) * 60)` | type=4 |
| 14 Trigger Fired | `sum by (trigger_type) (rate(trigger_orders_fired_total[1m]) * 60)` | type=4 |
| 15 Build Info | `build_info` | 直接读 gauge |

**Key design**: 大部分 panel 用 `rate() * 60` 把 req/s 转为 per-minute（业务用户更直观）。

## 3.4 关键设计决策

### D1: 不创建 template variables
**理由**: 所有 label 维度已经是封闭枚举（side / type / rule / scope / kind / direction / exchange / trigger_type），**不再需要 user/symbol filter**。`$symbol` / `$user` 会引入 open-ended cardinality（dashboard query 性能 + 视觉噪声）。

### D2: Stat panel 阈值 = 经验值
- Risk Trips: green<1, yellow<10, red>=10 (业务规则：>10 trips/instance 需调查)
- WS Active: green<100, yellow<1000, red>=1000 (资源耗尽信号)

**这些阈值是 P0-5 AlertManager 规则的最小化版本**。P0-5 会复用这些数值。

### D3: Refresh 30s
不是 5s 原因：Prometheus 默认 scrape 15s，30s 是 2x 缓冲。**5s 会触发 alertmanager 误报**（数据不稳）。

### D4: Piechart 用 donut（不是 pie）
- Donut 中心可显示 total（更紧凑）
- `displayLabels: ["name", "percent"]` 直接显示百分比
- 暗色主题下 donut 比 pie 视觉更清晰

### D5: Trigger Created/Fired 都画
**容易混淆**：用户只关心"创建"还是"激活"？**两个都画**（不同 panel）— Created 是用户行为， Fired 是系统响应（取决于价格波动）。

### D6: Build Info 用 stat (不是 timeseries)
Gauge 值是常量（`build_info{version="0.1.0",...} 1`），timeseries 永远是平的 1。**Stat 正确表达**这是 metadata 而不是 metric。

### D7: Datasource UID 唯一
`PBFA97CFB590B2093` 是 Grafana 默认值，**故意保留**（不是改）— 让其他 dashboard 可引用相同 datasource 而不必重配置。

### D8: JSON schemaVersion = 38
Grafana 9.4+ 标准。低于 38 会触发 "panel model out of date" 警告。
