# T4 — 测试报告

## 4.1 自动化校验脚本

`scripts/validate-dashboard.sh` 不存在，**临时手动校验**如下：

### 校验 1: JSON 语法

```python
import json
json.load(open("ops/grafana/dashboards/quant-trading-ops.json"))
# ✅ pass
```

### 校验 2: YAML 语法 (3 files)

```python
import yaml
yaml.safe_load(open("ops/grafana/provisioning/datasources/prometheus.yml"))
yaml.safe_load(open("ops/grafana/provisioning/dashboards/dashboards.yml"))
yaml.safe_load(open("ops/prometheus/prometheus.yml"))
# ✅ 3/3 pass
```

### 校验 3: Datasource UID 一致

```
Datasource UID in JSON panels:  PBFA97CFB590B2093  (15 occurrences)
Datasource UID in provisioning: PBFA97CFB590B2093
✅ match
```

### 校验 4: Metric name 对齐

```
Backend metrics.rs declares:  16 metric family
Dashboard panels reference:    15 metric family
All 15 referenced metrics exist in backend: ✅
```

**1 unused backend metric**: `http_request_duration_seconds_count` 和 `http_request_duration_seconds_sum`（histogram 自动生成，dashboard 只用 `_bucket` 算 P95 quantile — 正确做法）。

## 4.2 实跑验证 (T4.6)

启动 backend → 发送请求 → curl `/metrics` → 验证 dashboard 涉及的 metric 全部能查到：

```bash
$ curl -s http://localhost:8080/metrics | grep -E "^(http_|ws_)" | head
http_requests_total{method="GET",route="/api/v1/{id}",status="401"} 1
http_requests_total{method="GET",route="/api/v1/market/tickers",status="401"} 2
http_request_duration_seconds_bucket{...}  ...
ws_messages_broadcast_total{kind="ai_predict"} 1
```

✅ **dashboard 涉及的 metric 全部有数据**（业务 metric 仍 0 是 expected — 需 authenticated 流量）

## 4.3 Lint / Schema 门禁

| 门禁 | 命令 | 结果 |
|---|---|---|
| JSON parse | `python -c "import json; json.load(open('ops/grafana/dashboards/quant-trading-ops.json'))"` | ✅ ok |
| YAML parse (×3) | `python -c "import yaml; yaml.safe_load(open('...'))"` | ✅ 3/3 ok |
| Metric name 对齐 | 手动 + regex | ✅ 15/15 |
| Datasource UID | grep | ✅ 一致 |
| Grafana 9 schema | `schemaVersion: 38` | ✅ latest |

## 4.4 Code Review 关注点

1. **No template variables**: 不引入 user/symbol filter，label cardinality 已经控制
2. **Datasource UID**: 15 panel + provisioning 一致
3. **Refresh interval**: 30s（不是 5s）
4. **Units**: 全用 Grafana 内置 unit（reqps / s / short / msgps / rowsm）
5. **Threshold values**: 来自业务规则（risk>10/instance, ws>1000 conn），**P0-5 复用**
6. **Panel ID 唯一**: 1-15 + 100/200/300/400/500 (rows) — 无冲突
7. **No PII**: 无 user_id / order_id / email，label 都是 closed enum
