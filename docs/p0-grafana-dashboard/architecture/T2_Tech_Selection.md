# T2 — 技术选型

## 2.1 Dashboard 格式

| 方案 | 版本 | 优 | 劣 | 选 |
|---|---|---|---|---|
| **Grafana 9+ JSON model** | schema 38 | 业界标准 / provisioning 原生支持 | 手动写 JSON 复杂 | ✅ |
| Grafana 8 JSON | schema 36 | 老版本兼容 | 字段不新（legend v2 缺失） | ❌ |
| Grafonnet (jsonnet) | latest | 编程式 | 引入 jsonnet / 工具链 | ❌ 单 dashboard 不值得 |
| Helm chart 模板 | k8s only | 部署友好 | 限定 k8s | ❌ 通用性差 |

**结论**：用 Grafana 9+ JSON model（schema 38），手写。

## 2.2 Provisioning 模式

| 方案 | 加载时机 | 改 dashboard 体验 | 选 |
|---|---|---|---|
| **file provisioning** (`providers[].type=file`) | Grafana 启动 + 30s reload | UI 可改但 30s 覆盖 | ✅ |
| API 写入 DB | 一次性 | 需 API token | ❌ |
| 纯手 import | 手动 | 易忘 | ❌ |

**结论**：file provisioning，30s reload interval（折中 — UI 调试可改，重启不丢）。

## 2.3 Datasource UID 锁定

```yaml
# provisioning/datasources/prometheus.yml
datasources:
  - uid: "PBFA97CFB590B2093"  # 必须在所有 panel 同步
```

**为什么锁定 UID？**
- Grafana 默认 datasource 会被替换（多个 datasource 时）
- Lock 后 provisioning 改 datasource URL 不破坏 dashboard
- 删 datasource 时 panel 会变 "datasource not found" 错误而非静默错查询

## 2.4 Query 风格

| 风格 | 例子 | 选 |
|---|---|---|
| `sum by (label) (rate(metric[1m]))` | 标准 PromQL | ✅ 90% panels |
| `histogram_quantile(0.95, sum by (le) (rate(_bucket[5m])))` | P95 延迟 | ✅ HTTP duration |
| `sum(metric)` | 总和 / 不分组 | ✅ 3 stat panels |
| `topk(5, ...)` | 前 N | ❌ 当前不需要 |
| `increase()` | 累计增长 | ❌ 当前用 `rate()` |

**rate() vs increase()**: 短期窗口用 `rate()`，长期窗口（>5min）用 `increase()`。当前所有 panel 用 1m / 5m 窗口 → `rate()`。

## 2.5 单位 (Unit)

Grafana field config 用预定义 unit：
- `reqps` — HTTP rate
- `s` — seconds (duration)
- `short` — 整数 (订单数)
- `msgps` — 消息/秒
- `rowsm` — rows/min
- `none` — gauge / info

**不要**用 `%`（业务 metric 没有百分比语义，避免误读）。

## 2.6 Stacking 策略

| Metric | Stacking | 理由 |
|---|---|---|
| `orders_created_total` by (side, type) | normal | 总和有意义（总订单）|
| `orders_filled_total` by (side) | normal | 买卖成交量分布 |
| `trigger_orders_created_total` by (type) | normal | 类型分布 |
| `trigger_orders_fired_total` by (type) | normal | 类型分布 |
| `http_requests_total` by (route) | none | 路由独立 |
| `risk_rules_tripped_total` by (rule) | none | 独立规则 |
| `rate_limit_denied_total` by (scope) | none | 独立 scope |
| `ws_messages_broadcast_total` by (kind) | none | 独立类型 |
| `kline_persist_total` by (exchange) | none | 独立交易所 |

**`stacking.mode: "normal"`** 适合「构成」语义（看占比）。**`none`** 适合「独立」语义。
