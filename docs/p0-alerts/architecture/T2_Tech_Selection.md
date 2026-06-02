# T2 — 技术选型

## 2.1 规则格式

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **Prometheus native rules** (`*.yml` + `groups[].rules[]`) | 业界标准 / Prometheus / Grafana 原生 | 写起来啰嗦 | ✅ |
| Grafana managed alerting | UI 友好 | 锁定 Grafana 11+ | ❌ 当前 9.x |
| Datadog / NewRelic SaaS | 强大 | 第三方依赖 | ❌ 内部部署 |
| AlertManager only（无 rules） | 简单 | 需手写 alert 触发器 | ❌ |

**结论**：用 Prometheus native `groups: [name, interval, rules: [...]]`。

## 2.2 Alert vs Recording rules

| 类型 | 用途 | 当前用 |
|---|---|---|
| **Alert rule** | 评估 PromQL → 触发 alert | ✅ 13 个 |
| Recording rule | 预计算 PromQL → 新 metric（性能优化）| ❌ 当前不需要 |

P0-5 全部用 alert rules。**Recording rules 适合**高频/复杂 query 的预计算（**P2+ 任务**）。

## 2.3 严重度体系

| Severity | 响应时间 | 通知渠道 | 当前数量 |
|---|---|---|---|
| **critical** | 5min 内 page on-call | PagerDuty / 电话 | 3 (Risk 2 + Backend 1) |
| **warning** | 1 工作日内处理 | Slack #alerts | 9 (Risk 1 + RateLimit 3 + HTTP 2 + WS 2 + AI 1) |
| **info** | 观察，不强制响应 | Slack #alerts-info 或邮件 | 1 (AI 分布偏移) |

**为什么 critical=3？** 业务核心事件——Risk trip 直接关系到钱，Backend down 直接关系到可用性。其他 9 个都是 capacity / abuse / model 这类**有缓冲**的异常。

## 2.4 For duration 设计

| 事件类型 | For | 理由 |
|---|---|---|
| 立即业务事件 | 0m | daily_loss trip 一次就是 1 次损失，无须等待 |
| 容量风险 | 5m | 短暂 spike 是正常（listing / 行情剧烈波动） |
| HTTP 错误率 | 2m | 平衡冷启动 vs 持续错误 |
| HTTP P95 | 5m | GC / 短暂慢查询可接受 |
| WS exhaustion | 5m | 同上 |
| AI 分布 | 30m | 短期不均很正常 |
| AI 停滞 | 15m | 服务挂掉但 scrape 仍成功（业务 down 而非系统 down） |

**原则**：critical 立即 / warning 等 2-10m 去抖 / info 等 15-30m 看长期。

## 2.5 复合条件 (双 trigger)

### HTTP5xxSpike 双条件

```promql
(
  sum(rate(http_requests_total{status=~"5.."}[2m]))
  /
  sum(rate(http_requests_total[2m]))
) > 0.05
and
sum(rate(http_requests_total{status=~"5.."}[2m])) > 0.5
```

**为什么双条件？**
- 单独 `ratio > 5%`：1 个 5xx 在 10 个 req 里 = 10% ratio，**但只 0.17/s 绝对率** → **假阳性**
- 单独 `rate > 0.5/s`：1000 req 全成功 + 1 个 5xx 也不算严重
- 双条件：5% 比率 + 0.5/s 绝对率 → **真问题**才触发

### BackendDown 隐含双条件
`up == 0` for 1m → 隐含 **连续 4 次 scrape (15s × 4) 都失败**。单次失败不触发（防网络抖动）。

## 2.6 标签 vs 注解

| 类型 | 例子 | 用途 |
|---|---|---|
| **labels** | `severity`, `category`, `component` | routing / filtering / 抑制规则 |
| **annotations** | `summary`, `description`, `runbook_url`, `dashboard` | 通知文案 / 富链接 |

**所有 alert 共享 label schema**（`severity` / `category` / `component`），让 AlertManager routing 规则可以：
```yaml
routes:
  - match: { severity: "critical" }
    receiver: pagerduty
  - match: { category: "model" }
    receiver: slack-ml
```

## 2.7 复用 vs 新增 metric

P0-5 严格 100% 复用 P0-1~3 已声明 metric：
- 7 referenced metrics: `up`, `http_requests_total`, `http_request_duration_seconds_bucket`, `ws_connections_active`, `ws_messages_broadcast_total`, `risk_rules_tripped_total`, `rate_limit_denied_total`, `ai_predictions_total`

**没有**新加 metric 触发点（避免无谓 schema 变化）。如果发现需要新 metric，**先回到 P0-3 业务埋点流程**添加埋点，再写 alert。
