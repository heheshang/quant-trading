# ADR-012: Binance API 签名认证架构

|| 字段 | 值 |
|---|------|------|
| **ID** | ADR-012 |
| **状态** | T2-评审中 |
| **日期** | 2026-05-19 |
| **决策者** | Tech Lead |
| **影响范围** | 交易所网关、风控、订单执行、前端密钥管理 |
| **关联** | ADR-010 (订单管理), PRD-Missing-Features-20260519 (P0-F1) |

---

## 背景

`binance_rest.rs` 当前仅调用公开端点（`/api/v3/ticker/24hr`、`/api/v3/depth`），无需签名认证。

**P0-F1 目标**：扩展支持需要 HMAC-SHA256 签名的私有端点（账户信息、挂单、撤单等）。

Binance API 签名机制：
- 请求需带 `X-MBX-APIKEY` header
- 所有参数（除 `signature` 本身）需按字母顺序排列并编码为 query string
- `signature = HMAC-SHA256(query_string, secret_key)`，hex 编码
- 时间戳校验：请求时间戳与服务器时间偏差 ≤ 5s
- 频率限制：签名请求 1200次/分钟（公开行情 4800次/分钟）

---

## 决策

### D1: 签名客户端架构 — 装饰器模式

**方案选择**：在现有 `BinanceRestClient` 基础上新增 `SignedBinanceClient`，不修改原有代码。

```
BinanceRestClient (公开行情)
    ↓ 组合
SignedBinanceClient (签名层)
    ↓ 组合
ExchangeService (业务层)
```

**新增服务文件**：`backend/src/services/exchange/`

```
services/exchange/
├── mod.rs
├── signed_client.rs    # HMAC-SHA256 签名逻辑 + 加密存储
├── rate_limiter.rs     # 频率限制（滑动窗口）
├── api_keys.rs         # API Key 存储实体 + CRUD
└── endpoints.rs        # 已签名端点（account/order/rate-limit）
```

**关键设计**：`SignedBinanceClient` 持有 `BinanceRestClient` 实例 + `ApiKeyStore` + `RateLimiter`，对需要签名的请求注入签名。

### D2: API Key 加密存储

**方案选择**：AES-256-GCM 加密 + base64 编码存储

```
用户输入 API Key / Secret
    ↓
生成随机 12-byte nonce
    ↓
AES-256-GCM 加密 (key = master_key, nonce, plaintext)
    ↓
存储: base64(nonce || ciphertext || tag)
```

**master_key 来源**：
- 环境变量 `ENCRYPTION_MASTER_KEY`（生产）
- 开发环境从 `config.yaml` 读取（仅本地有效）

**数据库表**：`exchange_api_keys`

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 所属用户 |
| exchange | String | "binance" |
| api_key | String | 加密存储的 API Key（base64） |
| secret_encrypted | String | AES-256-GCM 加密的 Secret |
| nonce | String | 加密 nonce（base64） |
| permissions | String | 逗号分隔: "read,trade,spot" |
| is_active | Bool | 是否启用 |
| last_used_at | DateTime | 最后使用时间 |
| created_at | DateTime | 创建时间 |

### D3: 频率限制 — 滑动窗口计数器

**方案**：每个用户独立的滑动窗口计数器（Redis 或内存 `DashMap`）

```
Key: ratelimit:signed:{user_id}:{current_minute_window}
Value: 请求计数
TTL: 60s

检查: count >= limit → 429 Too Many Requests
```

**限制规则**：
- 单用户签名请求：1200次/分钟
- 全局签名请求：系统总量上限
- 超出后返回 `429` + 错误码 `RL-001`

### D4: 时间戳校验

**方案**：在签名客户端层校验，±5s 容差

```
请求时间戳 = recvWindow 内的时间戳
服务器时间 = 本地系统时间（从 Binance time API 同步，定期更新）
偏差 = |服务器时间 - 请求时间戳|

偏差 > 5000ms → 返回 401 "Timestamp for this request was not received"
```

**实现**：后台任务每 5 分钟同步一次 Binance 服务器时间，缓存本地。

### D5: 签名端点映射

| 功能 | Binance 端点 | 签名方式 |
|------|-------------|---------|
| 测试连通性 | GET /api/v3/account | 需要签名 |
| 账户余额 | GET /api/v3/account | 需要签名 |
| 下单 | POST /api/v3/order | 需要签名 |
| 撤单 | DELETE /api/v3/order | 需要签名 |
| 频率限制查询 | GET /api/v3/rateLimit/order | 需要签名 |

**内部 API 端点**（本系统）：

| 端点 | 方法 | 路径 | 认证 |
|------|------|------|------|
| 签名测试 | GET | /api/v1/exchange/ping | JWT |
| 账户信息 | GET | /api/v1/exchange/account | JWT+HMAC |
| 挂单 | POST | /api/v1/exchange/order | JWT+HMAC |
| 撤单 | DELETE | /api/v1/exchange/order/{orderId} | JWT+HMAC |
| 频率限制状态 | GET | /api/v1/exchange/rate-limit | JWT+HMAC |

---

## API 契约

### `GET /api/v1/exchange/ping`
**认证**：JWT
**响应**：
```json
{ "serverTime": 1747680000000, "status": "ok" }
```

### `GET /api/v1/exchange/account`
**认证**：JWT + HMAC
**响应**：
```json
{
  "accountId": "uuid",
  "balances": [
    { "asset": "USDT", "free": "1000.00", "locked": "50.00" },
    { "asset": "BTC", "free": "0.5", "locked": "0.0" }
  ]
}
```

### `POST /api/v1/exchange/order`
**认证**：JWT + HMAC
**请求体**：
```json
{
  "symbol": "BTCUSDT",
  "side": "BUY",
  "type": "MARKET",
  "quantity": "0.1"
}
```
**响应**：
```json
{
  "orderId": "uuid",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "type": "MARKET",
  "status": "FILLED",
  "executedQty": "0.1",
  "fills": [{ "price": "65000", "qty": "0.1", "commission": "0.0001" }]
}
```

### `DELETE /api/v1/exchange/order/{orderId}`
**认证**：JWT + HMAC
**响应**：
```json
{ "orderId": "uuid", "status": "CANCELLED" }
```

---

## 架构健康评分（预估）

| 维度 | 权重 | 得分 | 说明 |
|------|------|------|------|
| 依赖方向 | 30% | 28 | 模块依赖清晰，exchange → binance_rest，无逆向依赖 |
| 循环依赖 | 25% | 25 | 无循环依赖 |
| 模块边界 | 20% | 18 | exchange 模块独立，边界清晰 |
| 复杂度 | 15% | 12 | 签名逻辑简单清晰 |
| 技术债务 | 10% | 8 | 少量新增文件，无破坏性修改 |
| **总分** | | **91** | ≥80 ✅ |

---

## 实施计划

**第 1 天**：
- 创建 `services/exchange/` 模块
- 实现 `SignedBinanceClient`（签名生成 + 加密存储）
- 实现 `ApiKeyStore`（CRUD + AES-256-GCM 加密）
- 创建 `exchange_api_keys` 表迁移

**第 2 天**：
- 实现 `RateLimiter`（滑动窗口）
- 实现 `/exchange/ping`、`/exchange/account` 端点
- 实现时间戳同步任务

**第 3 天**：
- 实现 `/exchange/order`、`/exchange/order/{id}` 端点
- JWT + HMAC 双重认证中间件
- 单元测试 + 集成测试

---

## 参考

- [Binance API 签名文档](https://developers.binance.com/docs/simple-api/exchange-information/endpoint-security)
- [AES-256-GCM Rust 实现](https://docs.rs/aes-gcm/latest/aes_gcm/)
- ADR-010 (订单管理) — 风控前置
