# T9 — P0-5 Prometheus Alert Rules 最终评审

> **任务 ID**：P0-5
> **日期**：2026-06-01
> **状态**：✅ **PASS — 可合并**

## 9.1 完成情况

| 阶段 | 状态 | 备注 |
|---|---|---|
| T1 PRD | ✅ | 5 Gherkin 场景 + 7 NFR |
| T2 选型 | ✅ | Prometheus native + 13 alerts + 3 severity |
| T3 设计 | ✅ | 5 groups + 13 expressions + 8 关键决策 |
| T4 实现 | ✅ | 1 YAML (10KB) + prometheus.yml 加 1 行 |
| T4 测试 | ✅ | 13/13 静态检查 + 7/7 metric 对齐 |
| T4.6 实跑 | ⚠️ | 受限：本地无 promtool，已用替代验证 |
| T6 门禁 | ✅ | YAML parse + cross-ref ok |
| T7 文档 | ✅ | T0/T1/T2/T3/T4/T9 共 6 文件 |

## 9.2 交付物

| 资产 | 路径 | 大小 |
|---|---|---|
| Alert rules | `ops/prometheus/rules/quant-trading-alerts.yml` | 10.0 KB |
| Prometheus config (修改) | `ops/prometheus/prometheus.yml` | +1 行 (`rule_files: ["rules/*.yml"]`) |
| T0 总览 | `docs/p0-alerts/T0_Task_Overview.md` | 4.5 KB |
| T1 PRD | `docs/p0-alerts/requirements/T1-PRD_prometheus_alerts.md` | 2.7 KB |
| T2 选型 | `docs/p0-alerts/architecture/T2_Tech_Selection.md` | 3.4 KB |
| T3 设计 | `docs/p0-alerts/architecture/T3_Design.md` | 4.6 KB |
| T4 测试 | `docs/p0-alerts/qa/T4_Unit_Test_Report.md` | 2.5 KB |
| T9 评审 | `docs/p0-alerts/t9/T9_Final_Review.md` | (this) |

**Total**: 2 ops 改动 + 6 docs = 8 files

## 9.3 13 Alert 速览

| Group | Alert | Severity | For | Expr |
|---|---|---|---|---|
| risk_rules | RiskRuleDailyLossTripped | **critical** | 0m | `increase(...) > 0` |
| risk_rules | RiskRuleMaxDrawdownTripped | **critical** | 0m | `increase(...) > 0` |
| risk_rules | RiskRuleSingleTradeTripped | warning | 0m | `increase(...) > 3` |
| rate_limit | RateLimitGlobalSpike | warning | 5m | `rate > 5` |
| rate_limit | RateLimitUserSpike | warning | 5m | `rate > 2` |
| rate_limit | RateLimitSymbolSpike | warning | 5m | `rate > 1` |
| http_health | BackendDown | **critical** | 1m | `up == 0` |
| http_health | HTTP5xxSpike | warning | 2m | ratio>5% AND rate>0.5/s |
| http_health | HTTPLatencyP95High | warning | 5m | P95 > 2s |
| websocket | WSConnectionsExhaustion | warning | 5m | sum > 800 |
| websocket | WSBroadcastStall | warning | 10m | kline rate == 0 |
| ai_quality | AIPredictionSkewed | info | 30m | long ratio > 0.8 |
| ai_quality | AIPredictionStalled | warning | 15m | rate == 0 |

## 9.4 关键设计亮点 (D1-D8)

- **D1 复用 P0-4 阈值**：减 200 缓冲（alert 800 / dashboard 1000）
- **D2 双条件复合**：HTTP5xxSpike 用 ratio + 绝对率，避免冷启动假阳性
- **D3 clamp_min 防除零**：比 `or vector(1)` 语义化
- **D4 increase() vs rate()**：counter 用 increase 看"有没有发生"
- **D5 for: 0m 业务关键**：Risk trip 一次就告警
- **D6 runbook + dashboard 链接**：on-call 一键跳转
- **D7 标签统一 schema**：severity / category / component 三维路由
- **D8 不引入 recording rules**：13 alert 30s 评估无性能压力

## 9.5 业务价值

**之前**：7×24 盯 dashboard，靠人发现异常
**现在**：
- **3 critical** 立即 page on-call (Risk 2 + Backend 1)
- **9 warning** 自动 ticket (5min 持续)
- **1 info** 静默观察 (30min 长期信号)
- 业务影响 = 钱的事件 **必然告警**

## 9.6 已知限制

1. **未跑 promtool**：本地无 Prometheus binary
2. **未接 AlertManager**：rules 已定义但 notification 缺
3. **占位 URL**：runbook_url / dashboard 需替换为生产域名
4. **未做 alert fatigue 测试**：去抖时长需回放验证
5. **单实例 Prometheus**：HA 需 P2+

## 9.7 后续任务

| ID | 任务 | 优先级 |
|---|---|---|
| **P1-** | AlertManager + Slack/PagerDuty 路由 | P1 |
| **P1-** | docker-compose 部署 prometheus + grafana + alertmanager | P1 |
| **P1-** | silencer / maintenance window | P1 |
| **P1-** | CI 跑 promtool 静态校验 | P1 |
| **P2-** | 业务报警（策略 P&L 异常 / 撮合 slippage spike）| P2 |
| **P2-** | alert fatigue 历史回放调优 | P2 |
| **P2-** | HA（Thanos / Mimir）| P3 |

## 9.8 Commit 计划

```
ops(alerts): add Prometheus alert rules for 13 KPI (P0-5)

Add 13 alerts across 5 groups covering:
- risk_rules (3): daily_loss, max_drawdown (critical), single_trade
- rate_limit (3): global, user, symbol spikes
- http_health (3): backend down (critical), 5xx spike, P95 latency
- websocket (2): connection exhaustion, broadcast stall
- ai_quality (2): prediction skewed (info), stalled

All alerts reference existing P0-1~3 metrics (no new metric
declarations). Thresholds aligned with P0-4 dashboard stat
panels (with 200-buffer for capacity alerts).

Files:
- ops/prometheus/rules/quant-trading-alerts.yml (5 groups, 13 alerts)
- ops/prometheus/prometheus.yml (add rule_files reference)

Closes: P0-5
```

## 9.9 P0 阶段全景 (5/5)

| ID | 任务 | Commit | 状态 |
|---|---|---|---|
| P0-1 | /metrics 端点 | `8d8c586` | ✅ |
| P0-2 | HTTP middleware | `030e3f4` | ✅ |
| P0-3 | 业务埋点 | `19fc5e8` | ✅ |
| P0-4 | Grafana Dashboard | `aeca5d2` | ✅ |
| **P0-5** | **Alert Rules** | **TBD** | **✅ 文档完成，待 commit** |

**P0 阶段 5/5 完成**。完整可观测性栈：
- 16 backend metric
- 13 alert rules
- 15 dashboard panels
- 1 文档体系
