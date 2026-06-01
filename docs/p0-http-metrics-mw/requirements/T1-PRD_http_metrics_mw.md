# T1-PRD — HTTP Metrics Middleware

> **任务 ID**：P0-2
> **前置**：P0-1 `/metrics` 端点
> **日期**：2026-06-01

## 1. 背景

P0-1 实现了 `/metrics` 端点 + 15 个 metric 声明。但 P0-1 实跑时
`/metrics` body 永远是 0 行 — 因为业务代码从未调用 `inc()` /
`observe()`。

**问题**：运维 dashboard 接入 Prometheus 后，看到的全是空图表。

## 2. 目标

实施 **axum middleware**，对**每一个**进入 router 的 HTTP 请求自动埋点：
- `http_requests_total{method, route, status}` — QPS + 状态码分布
- `http_request_duration_seconds{method, route}` — 延迟分位

## 3. 用户故事

| 角色 | 场景 | 期望 |
|---|---|---|
| 运维 | 看 backend QPS 趋势图 | Grafana 1h 内 60 个数据点 |
| 运维 | 设置 5xx > 1% 告警 | 告警基于 `http_requests_total{status=~"5.."}` |
| 开发 | 优化慢接口 | `http_request_duration_seconds_bucket{le="0.5"}` 计算 P50/P95/P99 |
| 业务 | 量化下单延迟 | `/api/v1/order/*` P99 < 100ms 看板 |

## 4. 验收标准（Gherkin）

```gherkin
Scenario: HTTP middleware 计数
  Given backend 启动
  When 客户端 GET /api/v1/market/tickers (未授权)
  Then HTTP 401 + http_requests_total{method="GET",route="/api/v1/market/tickers",status="401"} += 1
  And  http_request_duration_seconds_count{method="GET",route="/api/v1/market/tickers"} += 1

Scenario: route label 用 matched path
  Given backend 启动
  When 客户端 GET /api/v1/order/cancel/abc-123-uuid
  Then route label = "/api/v1/order/cancel/{id}"（不是字面 UUID）

Scenario: 跳过 /metrics 自身
  Given backend 启动 + Prometheus 15s 抓取
  When 抓取 /metrics 1000 次
  Then http_requests_total{route="/metrics"} = 0（无自计数）

Scenario: 跳过 /health
  Given k8s liveness 1s 一次探测
  When 1 小时内 3600 次探测
  Then http_requests_total{route="/health"} = 0（探针不污染 QPS）
```

## 5. 范围

### 包含
- `src/middleware/metrics.rs` 新文件
- `src/middleware/mod.rs` 声明模块
- `main.rs` 在 `create_router` 末尾挂载
- 单元测试 4 个
- 文档 4 份

### 不包含
- 业务路径埋点（P0-3）
- Grafana Dashboard（P0-4）
- 自定义 histogram bucket（用 prometheus crate 默认）
- Request body 大小 / 响应 body 大小指标

## 6. 风险与依赖

| 风险 | 等级 | 缓解 |
|---|---|---|
| `route` label cardinality 爆炸 | 中 | 用 `MatchedPath` 模式，**绝不**用 raw URI |
| 探针污染 QPS 数据 | 中 | skip `/health` + `/api/v1/health/ready` |
| 自计数 feedback loop | 中 | skip `/metrics` |
| middleware 性能开销 | 低 | 仅 1 次 `Instant::now()` + 2 次 `with_label_values`（lazy O(1)）|

## 7. 时间线

| 阶段 | 计划 | 实际 |
|---|---|---|
| T2 选型 | 0.25h | 0.1h |
| T3 设计 | 0.5h | 0.25h |
| T4 实现 | 1.5h | 0.5h |
| T4 测试 | 1h | 0.5h |
| T4.6 实跑 | 0.5h | 0.25h |
| T6 门禁 | 0.25h | 0.1h |
| T7-T9 文档 | 0.5h | 0.25h |
| **合计** | **4.5h** | **2h** |

---

**T1 完成，进入 T2**。
