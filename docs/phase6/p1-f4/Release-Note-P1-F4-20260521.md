# Release Note v1.0.0 — P1-F4 下单频率限制

- Version: 1.0.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级实现 P1-F4 下单频率限制，采用多维度滑动窗口算法，支持用户/交易对/全局三级限制。

## 核心实现

### OrderRateLimiter

```rust
pub struct OrderRateLimiter {
    user_limits: Arc<RwLock<HashMap<Uuid, SlidingWindow>>>,
    symbol_limits: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    global_limit: Arc<RwLock<SlidingWindow>>,
}
```

### SlidingWindow 算法

- 60秒滑动窗口
- 毫秒精度时间戳
- 超限返回 `AppError::TooManyRequests("RL-001: Order rate limit exceeded")`

### 限制维度

| 维度 | 默认值 | 说明 |
|------|--------|------|
| 用户 | 10单/分钟 | 每个用户独立计算 |
| 交易对 | 20单/分钟 | 每个 symbol 独立计算 |
| 全局 | 100单/分钟 | 所有用户共享 |

### 响应头

```
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 7
X-RateLimit-Reset: 1716240000000
```

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| order_rate_limiter 单元测试 | 8 | ✅ |
