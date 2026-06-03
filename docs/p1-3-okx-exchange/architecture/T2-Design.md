# T2 — OKX Exchange Integration Design

## Architecture overview

```
HTTP Client (Frontend)
    ↓ JWT
Axum Router
    ↓ auth_middleware
    ↓ Extension(Arc<SignedOkxClient>)
handlers/exchange.rs
    ↓ user_id, request
SignedOkxClient (services/okx_signed_client.rs)
    ↓ HMAC-SHA256 signed request
ApiKeyStore (decrypt user keys)
    ↓ api_key, secret, passphrase
OKX v5 REST API
    ↓ JSON response
Handler → ApiResponse JSON
```

## Component design

### `SignedOkxClient` (services/okx_signed_client.rs)

Single struct holding the `ApiKeyStore` reference. All public methods are async and take `user_id: Uuid` to look up the user's OKX API keys.

```rust
pub struct SignedOkxClient {
    key_store: Arc<ApiKeyStore>,
    base_url: String,           // https://www.okx.com
    http: reqwest::Client,      // shared, with timeout config
}

impl SignedOkxClient {
    pub fn new(key_store: Arc<ApiKeyStore>) -> Self
    pub async fn ping(&self) -> Result<PingResponse, AppError>
    pub async fn get_account(&self, user_id: Uuid) -> Result<AccountInfo, AppError>
    pub async fn place_order(&self, user_id: Uuid, new_order: NewOrder) -> Result<OrderResponse, AppError>
    pub async fn cancel_order(&self, order_id: &str, symbol: &str, user_id: Uuid) -> Result<CancelOrderResponse, AppError>
    pub async fn get_pending_orders(&self, user_id: Uuid) -> Result<Vec<PendingOrder>, AppError>
}
```

### HMAC signing scheme

OKX uses a specific signing scheme (different from Binance):

```
timestamp = ISO 8601 UTC, e.g. "2026-06-02T11:00:00.000Z"
message = timestamp + method + requestPath + body
signature = Base64(HMAC-SHA256(secret, message))
```

Headers:
```
OK-ACCESS-KEY: <api_key>
OK-ACCESS-SIGN: <signature>
OK-ACCESS-TIMESTAMP: <timestamp>
OK-ACCESS-PASSPHRASE: <passphrase>
Content-Type: application/json
```

### Response types

OKX returns a wrapper:

```json
{
  "code": "0",
  "msg": "",
  "data": [ ... ]
}
```

The client unwraps `data` and parses the inner type. Non-zero `code` returns `AppError::ExternalApi`.

### Handler ↔ Service mapping

| HTTP Route | Handler | Client method |
|---|---|---|
| `GET /exchange/okx/ping` | `exchange_okx_ping` | `ping()` |
| `GET /exchange/okx/account` | `exchange_okx_account` | `get_account(user_id)` |
| `POST /exchange/okx/order` | `exchange_okx_create_order` | `place_order(user_id, new_order)` |
| `DELETE /exchange/okx/order/{orderId}` | `exchange_okx_cancel_order` | `cancel_order(order_id, symbol, user_id)` |
| `GET /exchange/okx/orders/pending` | `exchange_okx_pending_orders` | `get_pending_orders(user_id)` |

### Symbol format

**OKX uses hyphenated format**: `BTC-USDT`, `ETH-USDT`, etc.

The frontend sends OKX format for OKX endpoints (no conversion at the API boundary). A `binance_to_okx_symbol()` helper is reserved (dead_code with `#[allow]`) for future cross-exchange adapter use.

### Error handling

| OKX code | Meaning | AppError mapping |
|---|---|---|
| `0` | Success | OK |
| `50011` | Invalid API key | `AppError::Unauthorized` |
| `50113` | Timestamp expired | `AppError::BadRequest` |
| `51008` | Insufficient balance | `AppError::BadRequest("Insufficient balance")` |
| `51119` | Order does not exist | `AppError::NotFound` |
| Other non-zero | Generic OKX error | `AppError::ExternalApi(code, msg)` |

### State persistence

- **No new DB tables** — OKX uses the existing `api_keys` table with `exchange = 'okx'` discriminator
- **No new migrations** — all state lives in OKX's own systems

### Concurrency

- `SignedOkxClient` is `Send + Sync` (uses `Arc<ApiKeyStore>` internally)
- One instance is shared across all requests via `Arc<SignedOkxClient>` in `AppState`
- `reqwest::Client` is internally `Arc`-shared, allowing concurrent requests
- Per-user request rate limiting is enforced by the existing `OrderRateLimiter` (extensible to exchange calls)

### Security

- API keys are decrypted on-demand from `ApiKeyStore` (AES-256-GCM at rest)
- Decrypted keys live only in local stack frames; never persisted in plaintext
- HMAC signing happens inside the client; secret bytes never leave the function
- Logs mask sensitive fields (`*** ***` pattern)
- TLS 1.3 enforced by reqwest default
