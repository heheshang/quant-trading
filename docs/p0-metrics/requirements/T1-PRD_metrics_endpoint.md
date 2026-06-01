# T1-PRD — `/metrics` Prometheus 端点

> **任务 ID**：P0-1
> **作者**：Claude (rust-software-dev T1)
> **日期**：2026-06-01
> **关联报告**：[2026-06-01_Business_Audit_Report.md](../../design/2026-06-01_Business_Audit_Report.md)

## 1. 背景

`2026-06-01_Business_Audit_Report.md` 报告 P0 风险 #1：**无 `/metrics` 端点 — 运维无法监控**。
当前 backend 仅暴露 `/health`（业务可达性探针），缺少：
- 请求 QPS / 错误率
- 接口延迟分位（P50 / P95 / P99）
- 撮合引擎订单量、成交率
- WebSocket 在线连接数
- AI 预测调用量
- 风控规则触发次数

## 2. 目标

实现 `GET /metrics` 端点，**Prometheus text-exposition 格式**，
供 Grafana / AlertManager 抓取，覆盖 HTTP、订单、撮合、WS、风控、AI 六大业务域。

## 3. 用户故事

| 角色 | 场景 | 期望 |
|---|---|---|
| 运维 | Grafana 添加 backend dashboard | 200ms 内拉到所有指标 |
| 运维 | 订单失败率超阈值告警 | AlertManager 5s 内触发 |
| 业务 | 量化策略观察下单延迟 | P99 延迟分位 < 100ms |
| 开发 | backpressure 时观察 WS 连接 | 实时 WS 连接数图 |

## 4. 验收标准（Gherkin）

```gherkin
Scenario: Prometheus 抓取 /metrics
  Given backend 启动并通过 /health 检查
  When Prometheus GET http://backend:8080/metrics
  Then 响应 200 + Content-Type: text/plain; version=0.0.4
  And  响应 body 包含以下 metric families（至少 12 个）：
    | http_requests_total |
    | http_request_duration_seconds |
    | orders_created_total |
    | orders_filled_total |
    | orders_cancelled_total |
    | positions_open |
    | trigger_orders_created_total |
    | trigger_orders_fired_total |
    | risk_rules_tripped_total |
    | rate_limit_denied_total |
    | ws_connections_active |
    | ws_messages_broadcast_total |
    | kline_persist_total |
    | ai_predictions_total |
    | build_info |
```

## 5. 范围

### 包含
- 全局 Registry + 15 个指标定义
- `GET /metrics` handler
- Cargo.toml + 路由挂载 + 模块声明
- 单元测试 11 个
- 质量门禁通过

### 不包含（后续任务）
- 业务埋点实际调用（P0-2 HTTP middleware + P0-3 业务路径）
- Grafana Dashboard JSON（P0-4）
- 多用户 / 限流 / 鉴权
- Pushgateway 集成

## 6. 风险与依赖

| 风险 | 等级 | 缓解 |
|---|---|---|
| `prometheus` crate 拉入 `protobuf 2.28` (CVE-2024-0437) | 中 | 漏洞路径不触发，deny.toml ignore 显式记录 |
| 暴露 `/metrics` 到公网 | 中 | 假设已在内网，文档说明 |
| 业务埋点遗漏 | 低 | 本任务只声明 metric，后续 P0-2/3 任务埋点 |

## 7. 时间线

| 阶段 | 计划 | 实际 |
|---|---|---|
| T1 PRD | 0.5h | 0.25h |
| T2 选型 | 0.5h | 0.25h |
| T3 设计 | 1h | 0.5h |
| T4 实现 | 2h | 1h |
| T4 测试 | 1h | 0.5h |
| T5+T4.6 验证 | 1h | 0.5h |
| T6 门禁 | 0.5h | 0.25h |
| T7-T9 文档 | 0.5h | 0.5h |
| **合计** | **7h** | **3.75h** |

---

**T1 完成，进入 T2**。
