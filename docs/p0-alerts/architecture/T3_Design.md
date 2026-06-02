# T3 — 设计文档

## 3.1 5 Groups 划分

```
quant-trading-alerts.yml
├── risk_rules        (30s interval, 3 alerts)
│   ├── RiskRuleDailyLossTripped       (critical, 0m)
│   ├── RiskRuleMaxDrawdownTripped     (critical, 0m)
│   └── RiskRuleSingleTradeTripped     (warning, 0m)
├── rate_limit        (30s interval, 3 alerts)
│   ├── RateLimitGlobalSpike           (warning, 5m)
│   ├── RateLimitUserSpike             (warning, 5m)
│   └── RateLimitSymbolSpike           (warning, 5m)
├── http_health       (30s interval, 3 alerts)
│   ├── BackendDown                    (critical, 1m)
│   ├── HTTP5xxSpike                   (warning, 2m)
│   └── HTTPLatencyP95High             (warning, 5m)
├── websocket         (30s interval, 2 alerts)
│   ├── WSConnectionsExhaustion        (warning, 5m)
│   └── WSBroadcastStall               (warning, 10m)
└── ai_quality        (1m interval, 2 alerts)
    ├── AIPredictionSkewed             (info, 30m)
    └── AIPredictionStalled            (warning, 15m)
```

**Group interval 选择**：
- `risk_rules` / `rate_limit` / `http_health` / `websocket` — 30s（与 scrape 15s 对齐 × 2，去抖）
- `ai_quality` — 1m（AI 评估慢 + 不需高频）

## 3.2 Alert 表达式映射表

| Alert | PromQL |
|---|---|
| RiskRuleDailyLossTripped | `increase(risk_rules_tripped_total{rule="daily_loss"}[1m]) > 0` |
| RiskRuleMaxDrawdownTripped | `increase(risk_rules_tripped_total{rule="max_drawdown"}[1m]) > 0` |
| RiskRuleSingleTradeTripped | `increase(risk_rules_tripped_total{rule="single_trade"}[5m]) > 3` |
| RateLimitGlobalSpike | `sum(rate(rate_limit_denied_total{scope="global"}[1m])) > 5` |
| RateLimitUserSpike | `sum by (scope) (rate(rate_limit_denied_total{scope="user"}[1m])) > 2` |
| RateLimitSymbolSpike | `sum by (scope) (rate(rate_limit_denied_total{scope="symbol"}[1m])) > 1` |
| BackendDown | `up{job="quant-trading-backend"} == 0` |
| HTTP5xxSpike | `(sum(rate(http_requests_total{status=~"5.."}[2m])) / sum(rate(http_requests_total[2m]))) > 0.05 AND sum(rate(http_requests_total{status=~"5.."}[2m])) > 0.5` |
| HTTPLatencyP95High | `histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) > 2` |
| WSConnectionsExhaustion | `sum(ws_connections_active) > 800` |
| WSBroadcastStall | `sum(rate(ws_messages_broadcast_total{kind="kline"}[10m])) == 0` |
| AIPredictionSkewed | `(sum(ai_predictions_total{direction="long"}) / clamp_min(sum(ai_predictions_total), 1)) > 0.8` |
| AIPredictionStalled | `rate(ai_predictions_total[15m]) == 0` |

## 3.3 关键设计决策 (D1-D8)

### D1: 复用 P0-4 阈值（不变）
- WS exhaustion 800（dashboard stat 用 1000，alert 留 200 缓冲）
- Risk trip 阈值按 `increase > 0`（任意 trip 即告警，不是阈值）
- AI skew 0.8（80% 长边）
- HTTP 5xx 5% + 0.5/s 绝对率

### D2: 双条件复合触发（HTTP5xxSpike）
**理由**：单一比率易在低流量时段假阳性。**双条件**确保真问题：
- 绝对率 > 0.5/s（业务量足够大才有意义）
- 比率 > 5%（相对严重度）

### D3: clamp_min 防除零
```promql
clamp_min(sum(ai_predictions_total), 1)
```
**理由**：分母不能为 0。`clamp_min(x, 1)` 保证分母 ≥ 1。**比 `or vector(1)` 更语义化**。

### D4: increase() vs rate() 选择

| 类型 | 用 | 理由 |
|---|---|---|
| Counter (离散事件计数) | `increase()` over short window | 我们关心"有没有发生"，不是"发生频率" |
| Gauge (瞬时值) | 直接 `<` / `>` | ws_connections_active 是 gauge |
| Rate-based (持续异常) | `rate()` over longer window | HTTP 5xx / latency / WS exhaustion |

**风险 trip** 用 `increase() > 0` 而非 `rate() > 0.1`：因为"1 次 trip 也要知道"，**不是连续 trip 才报警**。

### D5: `for: 0m` 用于业务关键事件
- Risk trip (3 alerts) → 0m：一次就是问题
- BackendDown → 1m（防网络抖动）
- AI stalled → 15m（短期重启不算）

### D6: runbook_url + dashboard 链接
每条 alert 都有：
- `runbook_url: https://wiki.internal/runbooks/...` — 运维标准操作手册（**当前 link 是占位**）
- `dashboard: https://grafana.internal/d/quant-trading-ops-v1` — 一键跳到对应 dashboard

**当前都是占位 URL**——生产部署时需替换为真实内部 wiki 域名。

### D7: 标签统一 schema
| 标签 | 取值 | 路由用途 |
|---|---|---|
| `severity` | critical / warning / info | 严重度路由 |
| `category` | risk / abuse / availability / reliability / performance / capacity / data / model | 业务类别路由 |
| `component` | trading / api / ws / ai | 组件路由 |

**AlertManager 可基于这些标签做多级路由**（P1+ 任务）。

### D8: 不引入 recording rules
所有 query 都直接基于原始 metric 评估。**Recording rules 适合**：
- 高频复杂 query（>10Hz）
- 跨 metric join 复杂
- dashboard 慢 query 优化

**当前 13 alert** 每 30s-1m 评估一次，**复杂度低**，recording rules 不必要。
