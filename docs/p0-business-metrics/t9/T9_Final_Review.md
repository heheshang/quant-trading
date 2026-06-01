# T9 — P0-3 Business Metrics Instrumentation 最终评审

> **任务 ID**：P0-3
> **日期**：2026-06-01
> **状态**：✅ **PASS — 可合并**

## 9.1 完成情况

| 阶段 | 状态 | 备注 |
|---|---|---|
| T1 PRD | ✅ | 5 Gherkin 场景 + 5 NFR |
| T2 选型 | ✅ | 复用 P0-1 全局 Registry + 22 call sites |
| T3 设计 | ✅ | 埋点矩阵 + 文件改动清单 + 5 关键决策 |
| T4 实现 | ✅ | 8 个文件，+140 行 |
| T4 测试 | ✅ | 379/379 pass (无 regression) |
| T4.6 实跑 | ✅ | `/metrics` 输出新行 |
| T6 门禁 | ✅ | check / test / clippy / deny 全 ok |
| T7 文档 | ✅ | T0/T1/T2/T3/T4/T9 共 6 文件 |

## 9.2 业务价值

**之前**：监控只看 HTTP status / response time
**现在**：业务全景可观测
- 订单流：创建 / 取消 / 成交 (side × type × exchange)
- 触发单：4 种类型的创建 + 激活
- 风控：3 种规则的 trip 次数（报警基础）
- 限流：3 个 scope 的拒绝次数（滥用检测）
- WS：6 种消息类型的广播量 + 活跃连接数
- 数据管道：K线持久化吞吐
- AI：3 种预测方向分布（模型偏差监控）

## 9.3 技术亮点

1. **22 call sites 全部 1-2 行**：埋点不增加代码复杂度
2. **Cardinality 严格控制**：封闭 label 集合，最大 ~432 series
3. **3 个 inline match**：避免跨模块 helper 反向依赖
4. **D1-D5 关键决策**：OCO `inc_by(2)` / AI 方向降级 / 复用 P0-1 helper / match > Display

## 9.4 已知限制

1. **业务 metric 需真实流量才能验证**：单元测试覆盖了 metric registry 操作，但业务调用路径**只在生产 / 集成测试**才能完整验证
2. **未跨实例聚合**：当前单实例部署，Prometheus server scrape 即可
3. **WS broadcast 总数无去重**：每个用户订阅都收到同一 broadcast（业务设计如此）

## 9.5 后续任务

| ID | 任务 | 优先级 |
|---|---|---|
| **P0-4** | Grafana Dashboard JSON (5+ 面板) | P0 |
| **P0-5** | Prometheus AlertManager 规则 (3+ 报警) | P1 |
| **P1-** | 业务 metric 单元测试 (mock inc 验证) | P2 |
| **P1-** | prometheus 0.14 升级 (移除 protobuf advisory) | P2 |

## 9.6 Commit 计划

```
feat(backend): instrument business metrics across 12 metric families (P0-3)

Add 22 call sites across 8 files to make the /metrics endpoint
output real data. All labels use bounded enums to keep
cardinality safe (< 500 series total).

Closes: P0-3
```
