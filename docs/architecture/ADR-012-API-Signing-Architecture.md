# ADR-012: API 签名认证通用架构

> ID: ADR-012
> 日期: 2026-05-21
> 状态: Accepted
> 关联: ADR-019, PRD-P0-F1

## 背景

当前系统需要支持多个交易所（币安、OKX、Bybit 等）的私有 API 调用。所有带认证的 API（账户信息/划转/挂单）依赖签名机制，但各交易所签名协议各异。P0-F1 要求提供统一的 API 签名认证基础设施，实现安全存储、统一签名管道和频率限制。

本 ADR 定义通用架构，Binance 为第一个实现的交易所（见 ADR-019）。

## 技术方案

### 签名协议

**默认算法：HMAC-SHA256**

所有交易所统一使用 HMAC-SHA256 作为默认签名算法。输出为 hex 小写字符串。

```
signature = HMAC-SHA256(secret_key, query_string)
query_string = "param1=value1&param2=value2&timestamp=1234567890123"
```

**时间戳防重放（±5s 窗口）**

- 所有签名请求必须携带 `timestamp` 参数（Unix 毫秒）
- 服务器校验请求时间与服务器时间偏差不超过 ±5 秒
- 超偏差请求返回 401 并拒绝服务
- 偏差校准通过交易所时间同步端点实现（如 Binance `/api/v3/time`）

**Nonce（可选，防彩虹表攻击）**

- Nonce 由服务端生成，随加密密钥一同存储
- 可选字段，用于在 query string 中引入随机性，防止预计算彩虹表攻击
- 格式：`nonce || ciphertext`（AES-256-GCM 场景下作为 IV）

### 密钥存储

**AES-256-GCM 加密存储**

- 算法：AES-256-GCM（对称加密，认证加密）
- 主密钥：环境变量 `ENCRYPTION_MASTER_KEY`（32 字节，hex 编码）
- 每条 API Secret 独立随机 Nonce（12 字节）
- 存储格式：`base64(nonce || ciphertext || tag)`
- 不支持明文存储

**加密流程**

```
1. 生成 12 字节随机 nonce
2. AES-256-GCM 加密 secret_plaintext
3. 输出 base64(nonce || ciphertext || tag)
```

**解密流程**

```
1. base64 解码获取 nonce + ciphertext
2. AES-256-GCM 解密，验证 tag
3. 返回明文 secret（仅在签名请求时短暂使用）
```

### 频率限制

**签名请求 vs 公开请求区分**

- 公开请求：无签名，共享全局限制
- 签名请求：需认证，独立限制

**每个 API Key 独立计数**

- 使用 Redis 滑动窗口计数器
- Key：`rate_limit:{user_id}:{exchange}`
- Binance 限制：1200 次/分钟（可按交易所调整）
- 超限返回 HTTP 429

### 数据模型

**表：`exchange_api_keys`（PRD 2.1.5）**

| 字段 | 类型 | 描述 |
|------|------|------|
| id | UUID | 主键，API Key 记录 ID |
| user_id | UUID | 所属用户 ID |
| exchange | String | 交易所标识（"binance"） |
| api_key | String | API Key（公开部分） |
| secret_encrypted | String | 加密后的 Secret（nonce:ciphertext） |
| nonce | String | 加密用 Nonce（base64） |
| permissions | String | 权限数组，逗号分隔（read/margin/futures） |
| is_active | Bool | 是否启用 |
| last_used_at | DateTime | 最后使用时间 |
| created_at | DateTime | 创建时间 |

### 实现位置

**`backend/src/services/binance_signer.rs` — 签名算法**

- `sign_request(secret_key, query_string) -> hex_string`
- `build_signed_query(params, timestamp) -> query_string`
- `current_timestamp_ms() -> i64`

**`backend/src/services/exchange/signed_client.rs` — 签名客户端**

- `SignedBinanceClient`：组合 ApiKeyStore + RateLimiter
- `signed_get(endpoint, params, user_id)`：带签名的 GET 请求
- `place_order/cancel_order/get_order`：签名私有 API

**`backend/src/services/exchange/api_keys.rs` — 密钥管理**

- `ApiKeyStore`：API Key CRUD + 加密/解密
- `encrypt_secret()` / `decrypt_secret()`
- `upsert/get_active_key/find_by_user/delete`

**`backend/src/handlers/api_key.rs` — REST API**

- API Key 的增删改查端点
- 权限管理（read/margin/futures）

### 多交易所扩展

此架构设计为可扩展，支持其他交易所：

**扩展接口**

```rust
trait ExchangeSigner {
    fn sign(&self, secret: &str, query_string: &str) -> String;
    fn build_query(&self, params: &[(&str, &str)], timestamp: i64) -> String;
    fn required_headers(&self) -> HashMap<String, String>;
    fn timestamp_tolerance_secs(&self) -> i64;
}
```

**预期实现**

| 交易所 | 签名算法 | 特殊 header | 时间戳精度 |
|--------|----------|-------------|------------|
| Binance | HMAC-SHA256 | X-MBX-APIKEY | 毫秒 |
| OKX | HMAC-SHA256 | OK-ACCESS-KEY | 毫秒 |
| Bybit | HMAC-SHA256 | api_key | 毫秒 |

每个交易所实现独立的 `ExchangeSigner`，共享 `ApiKeyStore`（加密存储）和 `RateLimiter`（频率限制）。

## 决策

1. **默认签名算法为 HMAC-SHA256** — 足够安全，兼容主流交易所
2. **AES-256-GCM 加密存储** — 通过 `ENCRYPTION_MASTER_KEY` 实现无忧存储
3. **±5s 时间戳窗口** — 防止重放攻击，兼容 Binance 硬性要求
4. **每个 API Key 独立频率计数** — Redis 滑动窗口，防止单 Key 耗尽限额
5. **不存储明文 Secret** — 仅签名时短暂解密，使用后立即丢弃

## Consequences

**正面**

- 统一签名架构支持多交易所扩展
- 密钥安全存储，防止数据库泄露
- 频率限制保护交易所账户

**负面**

- 每次签名请求需解密，有轻微性能开销
- 需要安全管理 `ENCRYPTION_MASTER_KEY`（环境变量或密钥管理服务）

**待办**

- [ ] 实现 OKX `ExchangeSigner`
- [ ] 实现 Bybit `ExchangeSigner`
- [ ] 支持硬件 HSM（未来）
