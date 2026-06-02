# T0 — P0-4 Grafana Operations Dashboard 任务总览

> **任务 ID**：P0-4
> **前置**：[P0-1 端点](../p0-metrics/T0_Task_Overview.md) ·
> **[P0-2 middleware](../p0-http-metrics-mw/T0_Task_Overview.md)** ·
> **[P0-3 业务埋点](../p0-business-metrics/T0_Task_Overview.md)**
> **日期**：2026-06-01
> **状态**：✅ Done

## 范围

P0-1~3 让 backend 暴露 15 metric family 真实数据。P0-4 把这些数据**可视化为运维 dashboard**，使：

- **SRE / 运维** 一眼看到 HTTP 流量、错误率、延迟分布
- **业务** 一眼看到订单流、风险触发、触发单激活
- **AI / 数据** 一眼看到预测分布、K线吞吐、WS 广播健康

## 交付

| 资产 | 路径 | 大小 | 说明 |
|---|---|---|---|
| Dashboard JSON | `ops/grafana/dashboards/quant-trading-ops.json` | 16.6 KB | 5 rows × 15 panels |
| Datasource provisioning | `ops/grafana/provisioning/datasources/prometheus.yml` | 0.6 KB | Prometheus UID 锁定 |
| Dashboard provisioning | `ops/grafana/provisioning/dashboards/dashboards.yml` | 0.7 KB | 自动从 `/var/lib/grafana/dashboards` 加载 |
| Prometheus scrape config | `ops/prometheus/prometheus.yml` | 1.3 KB | 抓取 `localhost:8080/metrics` |

## Dashboard 结构

| Row | Panel | 类型 | 关键 metric |
|---|---|---|---|
| 📊 Overview | HTTP Request Rate | timeseries | `rate(http_requests_total[1m])` |
| | HTTP P50/P95/P99 | timeseries | `histogram_quantile(http_request_duration_seconds_bucket)` |
| 💼 Order Flow | Orders Created (side × type) | timeseries stacked | `rate(orders_created_total[1m])` |
| | Orders Cancelled (actor) | timeseries | `rate(orders_cancelled_total[1m])` |
| | Orders Filled (side) | timeseries stacked | `rate(orders_filled_total[1m])` |
| 🛡️ Risk & Limits | Risk Trips (stat) | stat | `sum(risk_rules_tripped_total)` |
| | Risk Trips (rate by rule) | timeseries | `rate(risk_rules_tripped_total[5m])` |
| | Rate Limit Denied (by scope) | timeseries | `rate(rate_limit_denied_total[1m])` |
| 🔌 WebSocket Realtime | Active WS Connections (stat) | stat | `sum(ws_connections_active)` |
| | WS Broadcast Rate (by kind) | timeseries | `rate(ws_messages_broadcast_total[1m])` |
| 🤖 AI & Data Pipeline | AI Prediction Distribution (donut) | piechart | `sum by (direction)(ai_predictions_total)` |
| | Kline Persist Throughput | timeseries | `rate(kline_persist_total[1m])` |
| | Trigger Orders Created | timeseries stacked | `rate(trigger_orders_created_total[1m])` |
| | Trigger Orders Fired | timeseries stacked | `rate(trigger_orders_fired_total[1m])` |
| | Build Info | stat | `build_info` |

**Total**: 20 panels (5 row dividers + 15 data panels)
**Layout**: 24-grid Grafana standard

## 质量门禁

| 阶段 | 结果 |
|---|---|
| JSON schema 解析 | ✅ `json.load()` ok |
| YAML schema 解析 | ✅ `yaml.safe_load()` 3/3 ok |
| Metric name 对齐 | ✅ 15/15 dashboard query 引用 → backend `metrics.rs` 真实存在 |
| Datasource UID 一致 | ✅ `PBFA97CFB590B2093` 在所有 15 panel + provisioning file 同步 |
| Live curl 验证 | ✅ `http_requests_total` 5 行 + `ws_messages_broadcast_total{kind="ai_predict"}` 1 行 |

## 部署

`docker-compose.yml` 增加（**P1+ 任务**）：
```yaml
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./ops/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    ports: ["9090:9090"]
  grafana:
    image: grafana/grafana:latest
    depends_on: [prometheus]
    volumes:
      - ./ops/grafana/provisioning:/etc/grafana/provisioning:ro
      - ./ops/grafana/dashboards:/var/lib/grafana/dashboards:ro
    ports: ["3000:3000"]
```

## 业务价值

**之前**：运维看 Prometheus raw TSDB，需要熟悉 PromQL
**现在**：5 类别面板覆盖核心 KPI，alert threshold 预设（绿黄红）
- HTTP 流量异常 → 1 眼发现
- 风险规则 spike → 1 眼发现
- WS 广播堆积 → 1 眼发现
- AI 预测分布异常（单边倾向）→ 1 眼发现
