# Release Note v0.9.0 — Phase 5 压测工具

- Version: 0.9.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级包含 **Phase 5 压测工具**，为上线前性能验证提供标准化工具。

## 新增功能

### stress-test binary
独立压测工具 binary，支持：

**CLI 参数：**
- `--scenario`：场景名称（health/order_write/order_read/portfolio）
- `--concurrency`：并发数（默认 10）
- `--duration`：持续时间秒（默认 30）
- `--warmup`：预热时间秒（默认 5）
- `--host`：目标地址（默认 http://localhost:8080）
- `--output`：JSON 报告输出路径

**阈值门禁：**
- QPS ≥ 500 → PASS/FAIL
- P99 ≤ 200ms → PASS/FAIL
- 错误率 ≤ 1% → PASS/FAIL

**报告输出示例：**
```json
{
  "scenario": "health",
  "qps": 1364.93,
  "avg_latency_ms": 3.8,
  "p50_ms": 13,
  "p99_ms": 27,
  "success_count": 6983,
  "error_count": 0,
  "error_rate": 0.0,
  "pass": true
}
```

## 新增 ADR

- **ADR-018**: 压测工具架构设计

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| 后端单元测试 | 254 | ✅ |
| 压测 binary 单元测试 | 11 | ✅ |
| 前端单元测试 | 708 | ✅ |

## 部署信息

- **Binary**: `cargo run --bin stress-test`
- **依赖**: reqwest, tokio, clap, url
- **不需要数据库或外部服务**

## 已知限制

- WebSocket 压测（ws_broadcast 场景）尚未实现
- order_read/order_write 需要认证 token
- 不支持分布式压测（单机为限）

## 下一步

- WS 压测实现（ws_broadcast 场景）
- HTML 报告生成（Chart.js 曲线图）
- CI 集成压测 job
