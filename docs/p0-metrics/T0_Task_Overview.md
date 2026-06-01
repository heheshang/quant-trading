# T0 — P0-1 任务总览

> **任务 ID**：P0-1（业务审计报告 P0 风险 #1）
> **来源**：[2026-06-01_Business_Audit_Report.md](../../design/2026-06-01_Business_Audit_Report.md) § 三、P0 #1
> **开始**：2026-06-01
> **完成**：2026-06-01
> **状态**：✅ Done

## 0.1 任务目标

实现 Prometheus `/metrics` 端点，让运维能监控 backend 关键指标。

## 0.2 文档结构

| 阶段 | 文档 |
|---|---|
| T1 PRD | [requirements/T1-PRD_metrics_endpoint.md](requirements/T1-PRD_metrics_endpoint.md) |
| T2 技术选型 | [architecture/T2_Tech_Selection.md](architecture/T2_Tech_Selection.md) |
| T3 设计 | [architecture/T3_Design.md](architecture/T3_Design.md) |
| T4 单元测试 | [qa/T4_Unit_Test_Report.md](qa/T4_Unit_Test_Report.md) |
| T9 最终评审 | [t9/T9_Final_Review.md](t9/T9_Final_Review.md) |

## 0.3 精简决策

按 rust-software-dev skill v4.1，本任务是 **小功能 P0**，**走精简流程**：
- **跳过**：T0 立项会（无产品决策）
- **跳过**：T4.5（无新 API 契约、无前端）
- **跳过**：T8（无新增管理面 UI）
- **合并**：T5+T4.6 一起跑（cargo test + 实跑 /metrics）
- **保留**：T1/T2/T3/T4/T6/T7/T9

## 0.4 关键指标

| 指标 | 数值 |
|---|---|
| 新增 Rust 代码 | 19,018 字节（2 文件） |
| 单元测试 | 11 个（100% 覆盖） |
| 测试通过率 | 375/375 = 100% |
| Clippy warnings | 0 |
| cargo deny | 4/4 pass |
| 实跑验证 | ✅ /metrics 返回 200 + 正确 content-type |

## 0.5 后续任务

- **P0-2**：HTTP middleware 自动埋点（核心业务点）
- **P0-3**：业务路径埋点（撮合 / 风控 / 触发 / WS）
- **P0-4**：Grafana Dashboard JSON
- **v0.2**：prometheus 0.13 → 0.14 升级（移除 protobuf advisory）
