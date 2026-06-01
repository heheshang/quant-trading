# T2 — 技术选型

## 2.1 Middleware 模式

| 方案 | 实现难度 | 与 P0-1 一致性 | 选 |
|---|---|---|---|
| **`axum::middleware::from_fn`** | 低（函数签名） | ✅ 复用 auth middleware pattern | ✅ |
| `tower::Layer` trait | 高 | ❌ 需新抽象 | ❌ |
| 自定义 Service | 中 | ❌ 重复造轮子 | ❌ |

**选 `from_fn`**：与 `auth_middleware` 同款，可读性高、测试简单。

## 2.2 Route Label 来源

| 方案 | Cardinality | 选 |
|---|---|---|
| **`MatchedPath` 提取器** | 路由数（~50）| ✅ |
| 原始 URI 路径 | 无限（用户 ID 变体）| ❌ |
| 自定义正则 | 中 | ❌ |

**选 `MatchedPath`**：axum 0.8 原生 `Request::extensions()` 包含，
fallback 到 raw path（未匹配路由 = 404）。

## 2.3 Skip 列表

| 路径 | 原因 |
|---|---|
| `/metrics` | 抓取自身不计入（防 feedback loop）|
| `/health` | k8s 1s 一次 liveness（QPS 数据污染）|
| `/api/v1/health/ready` | 同上 |

用 `if SKIPPED_PATHS.contains(&raw_path) { return next.run(req).await; }` 提前返回。

## 2.4 挂载位置

```
app.layer(cors)
   .layer(metrics_mw)   ← 本任务
   .layer(TraceLayer::new_for_http())
```

**理由**：
- **CORS 层之外**：CORS preflight 响应也被计数（4xx 可观测）
- **TraceLayer 之外**：TraceLayer 看到的是 metrics_mw 处理后的 Response（status code 真实）
- **auth 之外**：401 也被计数（关键 — 暴力破解攻击可视化）

## 2.5 时间戳来源

`std::time::Instant::now()`（单调时钟，禁用于墙钟）。
Histogram `observe(elapsed_secs_f64())`。

---

**T2 决策已落地 T3 → T4**。
