# T0 — P0-5 Prometheus Alert Rules 任务总览

> **任务 ID**：P0-5
> **前置**：[P0-1 端点](../p0-metrics/T0_Task_Overview.md) ·
> **[P0-2 middleware](../p0-http-metrics-mw/T0_Task_Overview.md)** ·
> **[P0-3 业务埋点](../p0-business-metrics/T0_Task_Overview.md)** ·
> **[P0-4 Dashboard](../p0-grafana-dashboard/T0_Task_Overview.md)**
> **日期**：2026-06-01
> **状态**：✅ Done

## 范围

P0-1~4 让 backend 暴露 15 metric family + 15 panel dashboard。**P0-5 把关键 KPI 转化为 Prometheus alert rules**——运维不再需要盯着 dashboard，**异常自动告警**。

## 交付

| 资产 | 路径 | 大小 | 说明 |
|---|---|---|---|
| Alert rules | `ops/prometheus/rules/quant-trading-alerts.yml` | 10.0 KB | 5 groups × 13 alerts |
| Prometheus 集成 | `ops/prometheus/prometheus.yml` (修改) | +1 行 | `rule_files: ["rules/*.yml"]` |

## 13 个告警 × 5 类别

| 类别 | Alert | Severity | 阈值 | For |
|---|---|---|---|---|
| **Risk 业务** | RiskRuleDailyLossTripped | critical | `increase > 0 / 1m` | 0m |
| | RiskRuleMaxDrawdownTripped | critical | `increase > 0 / 1m` | 0m |
| | RiskRuleSingleTradeTripped | warning | `increase > 3 / 5m` | 0m |
| **Abuse 滥用** | RateLimitGlobalSpike | warning | `rate > 5/s` | 5m |
| | RateLimitUserSpike | warning | `rate > 2/s` | 5m |
| | RateLimitSymbolSpike | warning | `rate > 1/s` | 5m |
| **HTTP 健康** | BackendDown | critical | `up == 0` | 1m |
| | HTTP5xxSpike | warning | `5xx ratio > 5% AND rate > 0.5/s` | 2m |
| | HTTPLatencyP95High | warning | `P95 > 2s` | 5m |
| **WS 实时** | WSConnectionsExhaustion | warning | `active > 800` | 5m |
| | WSBroadcastStall | warning | `kline rate == 0` | 10m |
| **AI 模型** | AIPredictionSkewed | info | `long ratio > 80%` | 30m |
| | AIPredictionStalled | warning | `rate == 0` | 15m |

**Severity 分布**：critical=3 / warning=9 / info=1

## 质量门禁

| 阶段 | 结果 |
|---|---|
| YAML parse | ✅ `yaml.safe_load()` ok |
| 表达式 parens / braces 平衡 | ✅ 13/13 |
| Severity / category / summary / description 必备字段 | ✅ 13/13 |
| Metric name 全部 backend 真实存在 | ✅ 7/7 referenced metrics |
| Dashboard ↔ Alert metric 重叠 | ✅ 7 alert metrics 全部 dashboard 可见 |
| Alert-only metric | ✅ 0 (无 dead metric) |
| For duration 合法 (`\d+[smhd]`) | ✅ 13/13 |
| 阈值与 dashboard stat panel 一致 (D2) | ✅ risk>10 / WS>1000 |

## 与 P0-4 集成一致性

- **BackendDown** → `up{job="quant-trading-backend"}` → scrape config 已经定义这个 job
- **HTTP5xxSpike / HTTPLatencyP95High** → 复用 P0-2 HTTP middleware 的 `http_requests_total` / `http_request_duration_seconds_bucket`
- **WS** → 复用 P0-3 业务埋点的 `ws_connections_active` / `ws_messages_broadcast_total`
- **AI** → 复用 P0-3 业务埋点的 `ai_predictions_total`
- **Risk** → 复用 P0-3 业务埋点的 `risk_rules_tripped_total`
- **RateLimit** → 复用 P0-3 业务埋点的 `rate_limit_denied_total`

**没有引入新 metric** — 100% 复用 P0-1~3 的产出。

## 业务价值

**之前**：运维必须 7×24 盯 dashboard 才不漏异常
**现在**：
- **3 critical**：触发即 page on-call（Risk trip / Backend Down）
- **9 warning**：5min 持续异常 → ticket / 业务时间处理
- **1 info**：dashboard 长期信号，无强制响应

## 部署

`prometheus.yml` 已经引用 `rule_files: ["rules/*.yml"]`，**docker-compose 启动后自动加载**（P1+ 任务）。

**AlertManager 路由规则**（P1+ 任务，本任务不做）：
```yaml
# ops/alertmanager/alertmanager.yml
route:
  receiver: 'slack-critical'
  group_by: ['alertname', 'severity']
  routes:
    - match: { severity: 'critical' }
      receiver: 'pagerduty'
    - match: { severity: 'warning' }
      receiver: 'slack-warnings'
    - match: { severity: 'info' }
      receiver: 'slack-info'
```

## 后续任务

| ID | 任务 | 优先级 |
|---|---|---|
| **P1-** | AlertManager + Slack/PagerDuty 路由 | P1 |
| **P1-** | docker-compose 部署 prometheus + grafana + alertmanager | P1 |
| **P1-** | 报警去重 / silencer / maintenance window | P1 |
| **P1-** | 业务报警（策略 P&L 异常 / 撮合 slippage spike）| P2 |
| **P2-** | 报警自适应阈值（ML-based）| P3 |
