# T3 — 设计文档

## 3.1 模块结构

```
backend/src/
├── middleware/
│   ├── metrics.rs     # 新增：HTTP metrics middleware (7.8 KB)
│   └── mod.rs         # 修改：+ pub mod metrics
└── main.rs            # 修改：create_router 末尾 +.layer(metrics_mw)
```

## 3.2 函数签名

```rust
pub async fn http_metrics_middleware(
    req: Request,
    next: Next,
) -> Response
```

**无 Result**：埋点失败时**仍返回 response**（metrics 不可阻塞业务）。

## 3.3 执行流程

```
1. raw_path = req.uri().path()
2. if raw_path ∈ SKIPPED_PATHS:
     return next.run(req).await        # 0 埋点
3. method = req.method().clone()       # Copy
4. route = req.extensions()
          .get::<MatchedPath>()
          .map(|m| m.as_str().to_string())
          .unwrap_or_else(|| raw_path.to_string())
5. start = Instant::now()
6. response = next.run(req).await
7. elapsed = start.elapsed().as_secs_f64()
8. status = response.status().as_u16().to_string()
9. HTTP_REQUESTS_TOTAL[method, route, status].inc()
10. HTTP_REQUEST_DURATION_SECONDS[method, route].observe(elapsed)
11. return response
```

## 3.4 Cardinality 风险表

| 维度 | 上限 | 实际 |
|---|---|---|
| `method` | 7 (HTTP verbs) | 4-5 (GET/POST/PUT/DELETE/PATCH) |
| `route` | 路由数 | ~50 |
| `status` | 60 (1xx-5xx + 自定义) | 5-10 |

**最大 cardinality**：5 × 50 × 10 = **2500 series** — Prometheus 完全可承受。

## 3.5 错误处理

| 错误 | 行为 |
|---|---|
| `with_label_values` panic (极罕见) | 整个请求 panic → 500（不应该发生）|
| MatchedPath 缺失（404 路径） | fallback 到 raw path |
| skip 路径 | 提前 return，0 埋点开销 |

## 3.6 与现有 TraceLayer 关系

| Layer | 位置 | 作用 |
|---|---|---|
| CORS | 最外 | 处理 preflight + Access-Control-* 头 |
| **HTTP metrics (本任务)** | 内 | 埋点 |
| TraceLayer | 最内 | tracing span + 错误日志 |

**HTTP metrics 不重复 TraceLayer 工作**：TraceLayer 关注的是日志/metrics
链路追踪，HTTP metrics 只关心 counter + histogram 增量。

---

**T3 设计完整，T4 已落地**。
