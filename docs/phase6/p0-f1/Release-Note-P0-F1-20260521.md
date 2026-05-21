# Release Note v0.9.0 — P0-F1 Binance API 签名加密

- Version: 0.9.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级包含 **P0-F1 Binance API 签名加密**模块，为所有私有 Binance API 端点提供 HMAC-SHA256 签名能力。

## 新增功能

### binance_signer 模块

```rust
use crate::services::binance_signer::{sign_request, build_signed_query, current_timestamp_ms};

// 签名
let sig = sign_request("secret_key", "symbol=BTCUSDT&timestamp=123456");
// -> "a1b2c3..." (64 char hex)

// 构建签名查询
let query = build_signed_query(&[("symbol", "BTCUSDT")], 1641381404172);
// -> "symbol=BTCUSDT&timestamp=1641381404172"

// 时间戳
let ts = current_timestamp_ms();
```

## 签名协议

1. 按字母排序所有参数
2. 拼接 `key=value&key=value...`
3. 追加 `&timestamp={ms}`
4. HMAC-SHA256(secret, query_string) -> hex 小写

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| binance_signer 单元测试 | 6 | ✅ |

## 已知限制

- 加密存储（AES-256-GCM）尚未实现（Phase 2）
- 频率限制（Redis 计数器）尚未实现（Phase 2）
- `/exchange/ping` 等端点尚未创建 handler（Phase 2）
