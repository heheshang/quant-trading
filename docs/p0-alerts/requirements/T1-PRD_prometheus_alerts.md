# T1-PRD — Prometheus Alert Rules (P0-5)

> **任务 ID**：P0-5
> **前置**：P0-1~4 metrics + dashboard
> **日期**：2026-06-01

## 1. 背景

P0-4 dashboard 提供了可视化，但**人是不可靠的监控器**。P0-5 把"重要 KPI 偏离"转成 Prometheus alerts，使：

- 关键业务事件（Risk trip / backend down）→ **page on-call**
- 容量风险（WS exhaustion / rate limit spike）→ **预警升级**
- 模型异常（AI 分布偏移 / 停滞）→ **静默观察**

## 2. Gherkin 验收

### Scenario 1: Risk trip 立刻触发 critical
```
Given backend running, 1 user crosses daily_loss_limit
When Prometheus evaluates rules every 30s
Then within 1 evaluation cycle, alert RiskRuleDailyLossTripped is FIRING
And severity=critical, category=risk
And annotation description points to risk_events table
```

### Scenario 2: Backend 不可达 → BackendDown
```
Given Prometheus scrapes localhost:8080 every 15s
When backend crashes / pod evicted
Then after 1 minute, alert BackendDown is FIRING
And severity=critical, category=availability
```

### Scenario 3: Rate limit 滥用 5min 持续
```
Given a misbehaving client retries storm at 100 RPS
When global rate limit denies 10/s for 5min
Then alert RateLimitGlobalSpike is FIRING
And severity=warning, category=abuse
```

### Scenario 4: WS 接近耗尽
```
Given active WS connections climbing to 850
When count > 800 for 5min
Then alert WSConnectionsExhaustion is FIRING
And severity=warning, category=capacity
```

### Scenario 5: AI 预测分布长边偏向
```
Given AI service produces 90% LONG predictions
When long_ratio > 0.8 sustained for 30min
Then alert AIPredictionSkewed is FIRING
And severity=info (advisory only, no action required)
```

## 3. 非功能需求

- **NFR-1 阈值与 dashboard 一致**：risk trip > 10 / WS active > 1000 是 dashboard stat panel 阈值，alert 用同一数值（减一个常量 0/20% 防止边沿触发）
- **NFR-2 For duration 合理**：critical=0m（立即告警）/ warning=2-5m（去抖动）/ info=15-30m（长期信号）
- **NFR-3 Metric 100% 复用**：不允许新加 metric；所有 expr 必须引用 P0-1~3 已声明的 metric
- **NFR-4 必备 annotation**：每个 alert 都有 `summary` + `description` + `runbook_url`（链接到 wiki）+ `dashboard`（链接到 Grafana）
- **NFR-5 标签规范**：每条 alert 都有 `severity` (critical/warning/info) + `category` (risk/abuse/availability/reliability/performance/capacity/data/model) + `component` (trading/api/ws/ai)
- **NFR-6 表达式可读**：使用 `rate()` / `increase()` / `histogram_quantile()` 标准函数，避免 `irate()` 等易混淆
- **NFR-7 复合条件去噪**：HTTP5xxSpike 要求 **绝对率 > 0.5/s AND 比率 > 5%**，双条件避免冷启动假阳性

## 4. 不在范围

- ❌ AlertManager 路由 (P1+ 任务)
- ❌ Slack / PagerDuty 集成 (P1+ 任务)
- ❌ Silencer / maintenance window (P1+ 任务)
- ❌ 跨实例聚合 / HA 复制 (P2+ 任务)
- ❌ 自适应阈值 (P3+ 任务)
- ❌ 业务报警（策略 P&L 异常）(P2+ 任务)
