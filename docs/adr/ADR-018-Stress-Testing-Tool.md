# ADR-018: 压测工具架构设计

> 版本: v1.0  
> 状态: Accepted  
> Date: 2026-05-21

## 背景

Phase 5 需要压测工具验证系统在高并发下的性能。上线前必须确认单机 QPS≥500、P99延迟≤200ms、并发WS≥1000。

## 决策

### 工具形态
**采用独立 `stress-test` binary，不混入主后端服务。**

理由：
- 压测是独立工具，不需要随主服务部署
- 独立 binary 方便 CI 集成和本地调试
- 不会对主服务造成干扰

### 压测引擎选型
**Rust + tokio + reqwest + tokio-tungstenite**

理由：
- 与主后端技术栈一致，开发维护成本低
- tokio 异步并发能力强，适合高压力测试
- reqwest 支持 HTTP/1.1 keep-alive，复用连接
- tungstenite 支持 WebSocket 压测

### 报告格式
**JSON + HTML 双输出**

- JSON：方便 CI 自动化解析和阈值判断
- HTML：人类可读，包含 Chart.js 曲线图

### 测试场景
| 场景 | 端点 | 方法 | 目的 |
|------|------|------|------|
| order_write | `/api/v1/orders` | POST | 写入吞吐测试 |
| order_read | `/api/v1/orders` | GET | 读取延迟测试 |
| portfolio | `/api/v1/portfolio/positions` | GET | 复杂查询延迟测试 |
| ws_broadcast | ws `/ws` | 订阅 kline | 广播延迟测试 |

## 目录结构

```
backend/
  src/bin/stress_test/
    main.rs          # CLI 入口，clap 解析参数
    http_client.rs   # HTTP 压测客户端
    ws_client.rs     # WebSocket 压测客户端
    reporter.rs      # 报告生成器
    scenarios.rs     # 预设场景配置
```

## CLI 参数设计

```bash
cargo run --bin stress-test -- \
  --scenario order_write \
  --concurrency 50 \
  --duration 60 \
  --warmup 10 \
  --output report.json
```

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--scenario` | 必填 | 场景名称 |
| `--concurrency` | 10 | 并发数 |
| `--duration` | 30 | 持续时间（秒） |
| `--warmup` | 5 | 预热时间（秒） |
| `--output` | stdout | 输出文件 |

## 阈值判断

| 指标 | 阈值 | 判定 |
|------|------|------|
| QPS | ≥ 500 | PASS/FAIL |
| P99 延迟 | ≤ 200ms | PASS/FAIL |
| 错误率 | ≤ 1% | PASS/FAIL |
| WS 并发 | ≥ 1000 | PASS/FAIL |

CI 中压测结果不达标则 pipeline 失败。

## 替代方案

| 方案 | 优点 | 缺点 |
|------|------|------|
| 独立 binary（选） | 解耦、CI 友好 | 需要单独编译 |
| wrk2 外部工具 | 功能成熟 | 需安装外部依赖 |
| 混入主服务 | 部署简单 | 耦合、不安全 |

## 结论

采用独立 `stress-test` binary 方案，Rust + tokio 技术栈，支持 HTTP 和 WebSocket 压测，JSON+HTML 双报告输出。
