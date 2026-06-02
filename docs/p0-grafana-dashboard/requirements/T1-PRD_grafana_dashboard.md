# T1-PRD — Grafana Operations Dashboard (P0-4)

> **任务 ID**：P0-4
> **前置**：P0-1~3 metrics
> **日期**：2026-06-01

## 1. 背景

P0-1~3 让 backend 暴露 15 metric family，但 raw `/metrics` text 格式不友好。运维需要：
- 一眼看 KPI（不是 PromQL）
- alert threshold 视觉化（不是 grep）
- 类别分组（HTTP / 业务 / 风险 / 实时 / AI）

## 2. Gherkin 验收

### Scenario 1: Dashboard 加载
```
Given Grafana running with provisioning mounted from ops/grafana/
When admin navigates to Operations / Quant-Trading Backend folder
Then dashboard "Quant-Trading Backend — Operations Dashboard" appears
And uid matches "quant-trading-ops-v1"
And contains 5 row dividers + 15 data panels
```

### Scenario 2: HTTP 流量可视化
```
Given backend serving 100 req/s mixed routes
When user views Overview > HTTP Request Rate panel
Then panel shows 5+ distinct route series
And each series is labeled by route (e.g. "/api/v1/orders/{id}")
And unit is "reqps" (req/s)
```

### Scenario 3: Risk alert 视觉化
```
Given 3 risk trips (1 daily_loss + 1 single_trade + 1 max_drawdown)
When user views Risk & Limits > Risk Trips (stat) panel
Then stat shows "3" with red color (threshold > 10)
And Risk Trips (rate) panel shows 3 distinct series
```

### Scenario 4: WS 健康度
```
Given 7 active frontend WS connections, 1 ai_predict broadcast
When user views WebSocket Realtime > Active Connections panel
Then stat shows "7" with green color
And Broadcast Rate panel shows 1 series "ai_predict"
```

### Scenario 5: AI 预测分布
```
Given 100 predictions: 60 long + 30 short + 10 neutral
When user views AI & Data Pipeline > AI Prediction Distribution panel
Then donut shows 3 slices with percentages 60% / 30% / 10%
```

## 3. 非功能需求

- **NFR-1 Metric 一致性**: 所有 panel query 引用的 metric name 必须在 `backend/src/metrics.rs` 真实存在。**自动化校验脚本**（见 T4）
- **NFR-2 Datasource UID 锁定**: 15 panel 全部引用 uid `PBFA97CFB590B2093`，与 provisioning 文件一致。**Grafana 启动时不会 fallback 到 default datasource**
- **NFR-3 Label cardinality 安全**: 所有 `by (xxx)` 维度使用封闭枚举（side / type / rule / scope / kind / direction / exchange / trigger_type）
- **NFR-4 JSON 严格格式**: Grafana 9+ schema 38（最新 LTS）
- **NFR-5 暗色主题**: 与 quant-trading UI 风格统一（Bloomberg+Linear dark）
- **NFR-6 Refresh interval**: 30s（不是 5s — 减少 Prometheus query 压力）

## 4. 不在范围

- ❌ Alert rules（**P0-5 AlertManager**）
- ❌ Dashboard 模板变量（当前所有 metric 不需要 user/symbol filter — label cardinality 已经控制）
- ❌ 跨实例聚合（**P2+ 任务**）
- ❌ 持久化 dashboard 历史（**运维侧**）
- ❌ Multi-tenant（**P2+ 任务**）
