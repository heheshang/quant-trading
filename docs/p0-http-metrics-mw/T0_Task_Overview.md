# T0 — P0-2 HTTP Metrics Middleware 任务总览

> **任务 ID**：P0-2（业务审计报告 P0 #1 续）
> **前置任务**：[P0-1 /metrics 端点](../p0-metrics/T0_Task_Overview.md)
> **开始**：2026-06-01
> **完成**：2026-06-01
> **状态**：✅ Done

## 0.1 任务目标

P0-1 实现了 `/metrics` 端点 + 15 个 metric family **声明**，
但**业务代码中还没有埋点** → 端点永远返回 0 行。
P0-2 实施**全栈 HTTP middleware 自动埋点**，让 HTTP 维度（QPS / 延迟 / 状态码）真正产生数据。

## 0.2 文档结构

| 阶段 | 文档 |
|---|---|
| T1 PRD | [requirements/T1-PRD_http_metrics_mw.md](requirements/T1-PRD_http_metrics_mw.md) |
| T2 技术选型 | [architecture/T2_Tech_Selection.md](architecture/T2_Tech_Selection.md) |
| T3 设计 | [architecture/T3_Design.md](architecture/T3_Design.md) |
| T4 单元测试 | [qa/T4_Unit_Test_Report.md](qa/T4_Unit_Test_Report.md) |
| T9 最终评审 | [t9/T9_Final_Review.md](t9/T9_Final_Review.md) |

## 0.3 关键决策

- **`axum::middleware::from_fn`** — 复用 P0-1 / auth middleware 同款 pattern，**无新依赖**
- **`MatchedPath` 提取器** — 用路由模式（`/api/v1/orders/{id}`）作 label，**避免 cardinality 爆炸**
- **Skip `/metrics` + `/health`** — 防 feedback loop + 探针污染数据
- **挂载位置**：`.layer(cors) → .layer(metrics_mw) → .layer(TraceLayer)` — 内层在 TraceLayer 外

## 0.4 关键指标

| 指标 | 数值 |
|---|---|
| 新增 Rust 代码 | ~7.8 KB（1 新文件）+ 8 行 main.rs |
| 单元测试 | 4 个（matched route / POST / duration histogram / skip path）|
| 测试通过率 | 379/379 = 100%（前 375 + 新 4）|
| Clippy warnings | 0 |
| cargo deny | 4/4 pass |
| 实跑验证 | ✅ `/api/v1/market/tickers{401}=2`, `/typo/here{404}=1`, `/health` skip 生效 |

## 0.5 后续任务

- **P0-3** 业务路径埋点（撮合 / 风控 / 触发 / WS / kline persist / AI）
- **P0-4** Grafana Dashboard JSON
- **v0.2** prometheus 0.14 升级（移除 protobuf advisory）
