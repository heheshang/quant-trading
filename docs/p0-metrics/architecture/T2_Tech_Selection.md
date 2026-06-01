# T2 — 技术选型

## 2.1 指标采集方案

### 选型：Pull 模型（Prometheus 主动 scrape）

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **Prometheus 文本格式** | 行业标准、Grafana/AlertManager 开箱即用、与 K8s 生态一致 | 需额外搭 Prometheus server | ✅ |
| OpenTelemetry OTLP | 厂商中立、可推送 | 生态分散、需要 collector | ❌ |
| 自研 JSON | 简单 | 与生态脱节 | ❌ |

**选 Prometheus**。理由：
1. 与 K8s `kube-state-metrics` / `node-exporter` 同语言生态
2. 文本格式无 SDK 依赖，方便 `curl` 调试
3. Grafana 模板复用度高

## 2.2 Rust 库

| 候选 | 版本 | License | 维护 | 选 |
|---|---|---|---|---|
| `prometheus` | 0.13 | MIT/Apache-2.0 | 活跃 | ✅ |
| `prometheus-client` | 0.22 | MIT/Apache-2.0 | 官方 | ❌（API 不够成熟） |
| `metrics` + exporter | 0.23 | MIT/Apache-2.0 | 活跃 | ❌（依赖更多） |

**选 `prometheus` 0.13**。理由：
- `Registry` 模式成熟
- 文本格式编码器稳定
- 已在 deny allow list

**已知 trade-off**：
- `prometheus 0.13` 拉入 `protobuf 2.28`（CVE RUSTSEC-2024-0437）。
  不启用 protobuf 特性则不触发漏洞路径。
  升级到 0.14 是 API breaking change，**当前 deny.toml ignore**。

## 2.3 路由挂载

`/metrics` 是运维端点（不走 `/api/v1` 业务前缀），与 `/health`、`/ws` 同级，**直接挂在根 `Router`**。

不通过 `AppState`（无业务依赖，仅全局 `LazyLock<Registry>`）。

## 2.4 初始化策略

**Lazy 静态注册**：`std::sync::LazyLock<>` 包裹每个 `IntCounterVec` /
`HistogramVec` / `IntGaugeVec`，**首次访问时初始化**。`init_build_info()`
在 main 启动时调用一次以注册 build_info 标签。

---

**T2 决策已落地 T3 → T4**。
