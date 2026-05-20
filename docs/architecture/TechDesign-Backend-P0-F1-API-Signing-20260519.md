# TechDesign-Backend-P0-F1-API-Signing-20260519

## 后端技术设计方案：P0-F1 API 签名加密

---

## 1. 文件结构

```
backend/src/
├── services/
│   ├── binance_rest.rs          # [不修改] 现有公开行情客户端
│   └── exchange/
│       ├── mod.rs               # [追加导出] signed_client, api_keys, rate_limiter
│       ├── binance_connector.rs # [不修改] WS connector
│       ├── signed_client.rs     # [新增] HMAC-SHA256 签名客户端
│       ├── api_keys.rs          # [新增] API Key 实体 + AES-256-GCM 加密存储
│       ├── rate_limiter.rs      # [新增] 滑动窗口频率限制
│       ├── endpoints.rs         # [新增] 已签名端点（account/order/ping）
│       ├── errors.rs            # [追加] 签名相关错误类型
│       ├── types.rs             # [追加] 签名请求/响应类型
│       └── ws_hub.rs            # [不修改]
├── handlers/
│   └── exchange.rs              # [新增] /api/v1/exchange/* handler
├── db/
│   ├── mod.rs                   # [追加] api_keys Entity
│   └── migrations/
│       └── xxx_create_exchange_api_keys.sql  # [新增]
└── config.rs                    # [追加] ENCRYPTION_MASTER_KEY 配置
```

---

## 2. 新增服务详解

### 2.1 `api_keys.rs` — API Key 存储

```rust
// Entity: exchange_api_keys
pub struct ApiKey {
    id: Uuid,
    user_id: Uuid,
    exchange: String,         // "binance"
    api_key: String,          // 加密后 base64
    secret_encrypted: String, // AES-256-GCM 加密后 base64
    nonce: String,            // 12-byte nonce base64
    permissions: String,      // "read,trade,spot"
    is_active: bool,
    last_used_at: Option<DateTime>,
    created_at: DateTime,
}

// AES-256-GCM 加密函数
fn encrypt_secret(master_key: &[u8; 32], secret: &str) -> Result<(nonce, ciphertext_and_tag), AppError>
fn decrypt_secret(master_key: &[u8; 32], nonce: &[u8], ct: &[u8]) -> Result<String, AppError>
```

### 2.2 `signed_client.rs` — 签名客户端

```rust
pub struct SignedBinanceClient {
    inner: BinanceRestClient,    // 组合，非继承
    key_store: ApiKeyStore,
    rate_limiter: RateLimiter,
    time_offset: i64,            // 服务器时间偏移（毫秒）
    master_key: [u8; 32],        // 解密用
}

impl SignedBinanceClient {
    /// 生成 HMAC-SHA256 签名
    fn sign_request(&self, query_string: &str, secret: &str) -> String {
        let signature = hmac_sha256::hmac_sha256(secret.as_bytes(), query_string.as_bytes());
        hex::encode(signature)
    }

    /// 构建签名请求：排序参数 → query_string → 签名 → 追加 signature
    fn build_signed_url(&self, params: &[(String, String)], secret: &str) -> String {
        // 1. 按字母顺序排序
        // 2. key1=value1&key2=value2
        // 3. HMAC-SHA256(secret, query_string) → signature
        // 4. query_string + "&signature=" + hex(sig)
    }

    /// 验证时间戳偏差
    fn validate_timestamp(&self, recv_window: i64) -> Result<(), AppError> {
        let偏差 = (current_time_millis() + self.time_offset) - recv_window;
        if偏差.abs() > 5000 {
            return Err(AppError::Unauthorized("Timestamp for this request was not received".into()));
        }
        Ok(())
    }
}
```

### 2.3 `rate_limiter.rs` — 滑动窗口频率限制

```rust
use std::collections::VecDeque;
use tokio::sync::Mutex;

pub struct SlidingWindowLimiter {
    window_secs: u64,
    max_requests: u64,
    requests: Mutex<VecDeque<i64>>, // 时间戳队列
}

impl SlidingWindowLimiter {
    pub async fn check_and_record(&self) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - (self.window_secs * 1000) as i64;
        let mut reqs = self.requests.lock().await;
        // 移除窗口外请求
        reqs.retain(|&ts| ts > window_start);
        if reqs.len() >= self.max_requests as usize {
            return Err(AppError::TooManyRequests("RL-001: Order rate limit exceeded".into()));
        }
        reqs.push_back(now);
        Ok(())
    }
}
```

---

## 3. Handler 设计

### `handlers/exchange.rs`

| 函数 | 路径 | 方法 | 认证 |
|------|------|------|------|
| `exchange_ping` | /api/v1/exchange/ping | GET | JWT |
| `exchange_account` | /api/v1/exchange/account | GET | JWT+HMAC |
| `exchange_create_order` | /api/v1/exchange/order | POST | JWT+HMAC |
| `exchange_cancel_order` | /api/v1/exchange/order/{orderId} | DELETE | JWT+HMAC |
| `exchange_rate_limit` | /api/v1/exchange/rate-limit | GET | JWT+HMAC |

**认证流程**：
```
请求 → JWT 验证（From Request） → extract user_id
     → 查询 exchange_api_keys（user_id + exchange="binance"）
     → 解密 secret（master_key）
     → 频率限制检查（user_id）
     → 时间戳校验（recv_window）
     → 构造签名请求 → Binance API
     → 更新 last_used_at
```

---

## 4. 数据库 Schema

```sql
CREATE TABLE exchange_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    exchange VARCHAR(32) NOT NULL DEFAULT 'binance',
    api_key TEXT NOT NULL,          -- base64(AES-256-GCM 加密)
    secret_encrypted TEXT NOT NULL, -- base64(AES-256-GCM 加密)
    nonce TEXT NOT NULL,            -- base64(12-byte nonce)
    permissions VARCHAR(128) NOT NULL DEFAULT 'read,trade',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, exchange)
);

CREATE INDEX idx_exchange_api_keys_user ON exchange_api_keys(user_id);
```

---

## 5. 状态机

API Key 状态：`active` ↔ `inactive`（软删除）

```
active ──[用户禁用]──→ inactive
inactive ──[用户启用]──→ active
```

---

## 6. 错误码规范

| 错误码 | 含义 |
|--------|------|
| 40101 | API Key 无效（签名校验失败）|
| 40102 | 时间戳过期 |
| 40103 | API Key 已禁用 |
| 42901 | 频率超限 |
| 40001 | 参数校验失败 |

---

## 7. 依赖项

新增 `Cargo.toml` 依赖：
```toml
aes-gcm = "0.10"        # AES-256-GCM 加密
hmac = "0.12"           # HMAC
sha2 = "0.10"           # SHA-256（HMAC-SHA256 底层）
hex = "0.4"             # hex 编码
```
