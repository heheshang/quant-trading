# PRD: Phase 5 压测工具

> 版本: v1.0  
> 状态: Draft  
> 作者: PM  
> Date: 2026-05-21

---

## 1. 背景

Phase 1-4 完成了核心交易功能（订单/风控/策略/组合），但未对系统各组件进行压测。上线前必须验证：
- 后端 API 在高并发下的吞吐量和延迟
- 数据库连接池上限
- WebSocket 连接数上限
- nginx 反向代理吞吐能力

本 PRD 定义压测工具的功能范围和验收标准。

---

## 2. 目标

| 目标 | 指标 |
|------|------|
| API 吞吐量 | 单机 QPS ≥ 500（/api/v1/order 等高频端点） |
| P99 延迟 | < 200ms（单机） |
| 并发 WS 连接 | ≥ 1000 |
| 压测报告 | 自动生成，包含 QPS/延迟/错误率/曲线图 |

---

## 3. 功能范围

### F1: HTTP API 压测
- 支持 GET/POST 多端点组合压测
- 可配置并发数、持续时间、预热时间
- 支持 token 认证（自动刷新）
- 输出：QPS、平均延迟、P50/P90/P99/P999、错误率

### F2: WebSocket 压测
- 支持模拟多 WS 客户端同时连接
- 验证 ws_hub 连接数上限
- 验证广播消息延迟

### F3: 压测报告
- JSON + HTML 格式导出
- 包含时间序列折线图（延迟/QPS）
- 支持与历史报告对比

### F4: 测试场景预设
- `/api/v1/orders` 写入压测
- `/api/v1/portfolio/positions` 读取压测
- `/api/v1/market/kline` WebSocket 订阅压测

---

## 4. 不做什么

- 不做分布式压测（单机为限）
- 不做真实交易下单压测（paper trade only）
- 不做全链路压测（只压后端 HTTP/WS 层）

---

## 5. 数据来源

- 测试数据：数据库已有测试账户（paper_accounts）
- K线数据：从 Binance 历史数据生成（不依赖实时 WS）

---

## 6. 验收标准

| ID | 标准 |
|----|------|
| AC1 | `cargo run --bin stress-test -- --scenario order_write` 成功执行，输出 QPS ≥ 500 |
| AC2 | P99 延迟 ≤ 200ms（100并发持续60s） |
| AC3 | 生成 `stress_report.html`，包含 QPS 曲线和延迟分布 |
| AC4 | 支持 `--concurrency 50 --duration 60 --warmup 10` 参数 |

---

## 7. 技术方案

- **工具形态**：`cargo run --bin stress-test`（独立 binary）
- **压测引擎**：Rust + tokio（参考 goat lords/duck byte）
- **HTTP 压测**：`reqwest` 并发客户端
- **WS 压测**：`tokio-tungstenite` 多连接
- **报告生成**：serde_json + 简单 HTML 模板
