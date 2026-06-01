# OKX 交易功能对接 - T0/T1/T2/T3/T4/T5

## T0: 需求确认 + 架构设计

### 接入范围
| 功能 | 优先级 | 说明 |
|------|--------|------|
| 账户余额查询 | P0 | `GET /api/v1/exchange/okx/account` |
| 下单 | P0 | `POST /api/v1/exchange/okx/order` |
| 撤单 | P0 | `DELETE /api/v1/exchange/okx/order/{orderId}` |
| 挂单查询 | P1 | `GET /api/v1/exchange/okx/orders/pending` |
| 连通性测试 | P0 | `GET /api/v1/exchange/okx/ping` |

### OKX vs Binance 共存方案
- **前端切换**: 通过 `exchange` 参数 (值: `binance` | `okx`)
- **后端路由**: `/api/v1/exchange/{exchange}/...` (如 `/api/v1/exchange/okx/ping`)
- **客户端单例**: `SignedBinanceClient` + `SignedOkxClient` 并存于 AppState

### Symbol 格式转换
| 交易所 | 格式 | 示例 |
|--------|------|------|
| Binance | BTCUSDT | BTCUSDT, ETHUSDT |
| OKX | BTC-USDT | BTC-USDT, ETH-USDT |

**转换规则**:
```rust
// Binance -> OKX: BTCUSDT -> BTC-USDT (插入连字符)
// OKX -> Binance: BTC-USDT -> BTCUSDT (移除连字符)
```

### API 端点规划
```
GET    /api/v1/exchange/okx/ping           - 连通性测试
GET    /api/v1/exchange/okx/account         - 账户余额
POST   /api/v1/exchange/okx/order           - 下单
DELETE /api/v1/exchange/okx/order/{orderId}  - 撤单
GET    /api/v1/exchange/okx/orders/pending  - 挂单查询
```

### 架构组件
```
main.rs
├── SignedOkxClient (单例)
│   └── services/okx_signed_client.rs (已有)
├── SignedBinanceClient (已有)
└── Exchange Handlers
    └── handlers/exchange.rs
        ├── exchange_ping_okx
        ├── exchange_account_okx
        ├── exchange_create_order_okx
        ├── exchange_cancel_order_okx
        └── exchange_pending_orders_okx
```

---

## T1: PRD Review (review-required)

### API Endpoints 清单

#### 1. GET `/api/v1/exchange/okx/ping`
- **认证**: JWT
- **请求**: 无
- **响应**:
```json
{
  "code": 0,
  "data": {
    "server_time": 1717200000000,
    "status": "ok"
  },
  "message": "success"
}
```

#### 2. GET `/api/v1/exchange/okx/account`
- **认证**: JWT + HMAC (OKX签名)
- **请求**: 无
- **响应**:
```json
{
  "code": 0,
  "data": {
    "balances": [
      {"asset": "BTC", "free": "1.5", "locked": "0.1"},
      {"asset": "USDT", "free": "10000", "locked": "0"}
    ]
  },
  "message": "success"
}
```

#### 3. POST `/api/v1/exchange/okx/order`
- **认证**: JWT + HMAC
- **请求**:
```json
{
  "symbol": "BTC-USDT",       // OKX格式 (非BTCUSDT)
  "side": "buy",
  "type": "limit",
  "quantity": "0.1",          // 非必填，市价单可省略
  "price": "60000",          // 非必填，市价单可省略
  "time_in_force": "gtc"     // 可选: gtc, ioc, fok
}
```
- **响应**:
```json
{
  "code": 0,
  "data": {
    "order_id": "123456",
    "symbol": "BTC-USDT",
    "side": "buy",
    "order_type": "limit",
    "status": "filled",
    "executed_qty": "0.1",
    "fills": [
      {"price": "60000", "qty": "0.1", "commission": "0.0001"}
    ]
  },
  "message": "success"
}
```

#### 4. DELETE `/api/v1/exchange/okx/order/{orderId}`
- **认证**: JWT + HMAC
- **请求 Body**:
```json
{
  "symbol": "BTC-USDT"
}
```
- **响应**:
```json
{
  "code": 0,
  "data": {
    "order_id": "123456",
    "status": "cancelled"
  },
  "message": "success"
}
```

#### 5. GET `/api/v1/exchange/okx/orders/pending`
- **认证**: JWT + HMAC
- **请求**: 无
- **响应**:
```json
{
  "code": 0,
  "data": [
    {
      "order_id": "123456",
      "symbol": "BTC-USDT",
      "side": "buy",
      "order_type": "limit",
      "price": "60000",
      "qty": "0.1",
      "status": "live"
    }
  ],
  "message": "success"
}
```

### 数据库模型变更
- **无需数据库变更** — OKX交易通过 `SignedOkxClient` 直连OKX API

### 前后端契约

| 字段 | Binance格式 | OKX格式 | 转换规则 |
|------|-------------|---------|----------|
| symbol | BTCUSDT | BTC-USDT | `BTCUSDT` → `BTC-USDT` |
| timestamp | Unix ms | Unix ms | 相同 |
| price | String | String | 相同 |
| quantity | String | String | 相同 |

**前端请求示例**:
```typescript
// 下单
POST /api/v1/exchange/okx/order
{
  "symbol": "BTC-USDT",  // 注意：是 BTC-USDT，非 BTCUSDT
  "side": "buy",
  "type": "limit",
  "price": "60000",
  "quantity": "0.1"
}
```

---

## T2: 实现

### 2.1: handlers/exchange.rs 新增 OKX handlers

**新增函数**:
- `exchange_ping_okx` - GET `/api/v1/exchange/okx/ping`
- `exchange_account_okx` - GET `/api/v1/exchange/okx/account`
- `exchange_create_order_okx` - POST `/api/v1/exchange/okx/order`
- `exchange_cancel_order_okx` - DELETE `/api/v1/exchange/okx/order/{orderId}`
- `exchange_pending_orders_okx` - GET `/api/v1/exchange/okx/orders/pending`

**复用响应类型**:
- `ExchangePingResponse`
- `ExchangeAccountResponse`
- `ExchangeOrderResponse`
- `ExchangeCancelResponse`

**新增请求类型**:
- `ExchangePendingOrdersResponse` - 挂单查询响应

**Symbol 转换函数** (utils层):
```rust
// BTCUSDT -> BTC-USDT (Binance to OKX)
fn binance_symbol_to_okx(symbol: &str) -> String

// BTC-USDT -> BTCUSDT (OKX to Binance)
fn okx_symbol_to_binance(symbol: &str) -> String
```

### 2.2: main.rs 注册 SignedOkxClient

**修改位置**: `main()` 函数

```rust
// 已有 (line 98-100)
let signed_client = Arc::new(SignedBinanceClient::new(key_store.clone()));

// 新增
let signed_okx_client = Arc::new(SignedOkxClient::new(key_store.clone()));
```

**修改 `create_router()` 函数签名**:
```rust
fn create_router(
    ...
    signed_client: Arc<SignedBinanceClient>,
    signed_okx_client: Arc<SignedOkxClient>,  // 新增
    key_store: Arc<ApiKeyStore>,
    ...
)
```

### 2.3: 路由映射 `/api/v1/exchange/okx/*`

**新增路由块**:
```rust
// Exchange OKX routes
let exchange_okx_routes = Router::new()
    .route("/exchange/okx/ping", get(handlers::exchange::exchange_ping_okx))
    .route("/exchange/okx/account", get(handlers::exchange::exchange_account_okx))
    .route("/exchange/okx/order", post(handlers::exchange::exchange_create_order_okx))
    .route("/exchange/okx/order/{orderId}", delete(handlers::exchange::exchange_cancel_order_okx))
    .route("/exchange/okx/orders/pending", get(handlers::exchange::exchange_pending_orders_okx))
    .layer(Extension(signed_okx_client.clone()))
    .layer(middleware::from_fn(
        quant_trading_backend::middleware::auth::auth_middleware,
    ))
    .with_state(app_state.db.clone());
```

### 2.4: Symbol 格式转换层

**实现位置**: `utils/symbol.rs` (新建)

```rust
/// Binance格式转OKX格式: BTCUSDT -> BTC-USDT
pub fn binance_to_okx(symbol: &str) -> String {
    // BTCUSDT -> [BTC, USDT] -> BTC-USDT
    if symbol.len() >= 4 {
        let base = &symbol[..symbol.len() - 4];
        let quote = &symbol[symbol.len() - 4..];
        format!("{}-{}", base, quote)
    } else {
        symbol.to_string()
    }
}

/// OKX格式转Binance格式: BTC-USDT -> BTCUSDT
pub fn okx_to_binance(symbol: &str) -> String {
    symbol.replace("-", "")
}
```

**使用场景**:
- `exchange_create_order_okx`: 前端传入 Binance格式 → 转为 OKX格式 调用 `SignedOkxClient`
- `exchange_cancel_order_okx`: 前端传入 Binance格式 → 转为 OKX格式 调用 `SignedOkxClient`

---

## T3: 前后端契约审查

### API 格式 (Request/Response)

#### 公共响应格式
```json
{
  "code": 0,           // 0=成功，非0=失败
  "data": { ... },     // 业务数据
  "message": "success"  // 错误信息
}
```

#### 错误码定义
| code | 说明 |
|------|------|
| 0 | 成功 |
| 400 | 请求参数错误 |
| 401 | 认证失败 |
| 403 | 无权限 |
| 500 | 服务器内部错误 |

### 前端调用示例

```typescript
// 1. Ping 测试
GET /api/v1/exchange/okx/ping
Authorization: Bearer <jwt_token>

// 2. 获取账户余额
GET /api/v1/exchange/okx/account
Authorization: Bearer <jwt_token>

// 3. 下单 (symbol使用Binance格式，前端无需知道转换)
POST /api/v1/exchange/okx/order
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "symbol": "BTCUSDT",   // 前端用Binance格式
  "side": "buy",
  "type": "limit",
  "price": "60000",
  "quantity": "0.1"
}

// 4. 撤单
DELETE /api/v1/exchange/okx/order/123456
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "symbol": "BTCUSDT"   // 前端用Binance格式
}

// 5. 查询挂单
GET /api/v1/exchange/okx/orders/pending
Authorization: Bearer <jwt_token>
```

### 前端切换交易所示例

```typescript
// 切换到 OKX
const exchange = 'okx';
const symbol = 'BTCUSDT';  // 前端始终用 Binance 格式

// API调用
await api.post(`/exchange/${exchange}/order`, {
  symbol,  // 后端自动转换
  side: 'buy',
  type: 'limit',
  price: '60000',
  quantity: '0.1'
});
```

---

## T4: 单元测试

### 4.1: Symbol 转换测试
```rust
#[cfg(test)]
mod symbol_tests {
    use crate::utils::symbol::{binance_to_okx, okx_to_binance};

    #[test]
    fn test_binance_to_okx() {
        assert_eq!(binance_to_okx("BTCUSDT"), "BTC-USDT");
        assert_eq!(binance_to_okx("ETHUSDT"), "ETH-USDT");
        assert_eq!(binance_to_okx("BNBUSDT"), "BNB-USDT");
    }

    #[test]
    fn test_okx_to_binance() {
        assert_eq!(okx_to_binance("BTC-USDT"), "BTCUSDT");
        assert_eq!(okx_to_binance("ETH-USDT"), "ETHUSDT");
    }
}
```

### 4.2: Mock SignedOkxClient
```rust
#[cfg(test)]
mod mock_tests {
    use crate::services::okx_signed_client::{
        SignedOkxClient, NewOrder, AccountInfo, Balance,
        OrderResponse, CancelOrderResponse, PendingOrder,
    };

    // Mock SignedOkxClient for testing handlers
    pub struct MockOkxClient {
        pub account_response: Option<AccountInfo>,
        pub order_response: Option<OrderResponse>,
        pub cancel_response: Option<CancelOrderResponse>,
        pub pending_response: Option<Vec<PendingOrder>>,
    }

    impl MockOkxClient {
        pub fn new() -> Self { ... }
    }

    #[tokio::test]
    async fn test_get_account() {
        let client = MockOkxClient::new();
        let result = client.get_account(user_id).await;
        assert!(result.is_ok());
    }
}
```

### 4.3: Order Creation 测试
```rust
#[cfg(test)]
mod order_tests {
    #[tokio::test]
    async fn test_create_order_request_validation() {
        // 测试缺少必填字段
        let req = CreateExchangeOrderRequest {
            symbol: "".into(),
            side: "buy".into(),
            order_type: "limit".into(),
            quantity: None,
            price: Some("60000".into()),
            time_in_force: None,
            quote_order_qty: None,
        };
        // 应返回 BadRequest("symbol is required")
    }
}
```

### 4.4: Handler 集成测试
```rust
#[cfg(test)]
mod handler_tests {
    use axum::{
        body::Body,
        routing::delete, get, post,
        Router,
    };

    #[tokio::test]
    async fn test_exchange_ping_okx() {
        let app = Router::new()
            .route("/exchange/okx/ping", get(exchange_ping_okx))
            .layer(Extension(mock_client));

        let response = app.oneshot(Request::builder()
            .uri("/exchange/okx/ping")
            .method("GET")
            .extension(AuthenticatedUser { user_id: uuid })
            .body(Body::empty())
        ).await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

---

## T5: 部署验证

### 5.1: 本地 docker compose 测试

**启动服务**:
```bash
cd /home/ssk/workspace/quant-trading
docker compose up -d
```

**验证步骤**:
1. 启动后端: `docker compose up -d backend` (或 `make dev` 本地)
2. 检查日志: `docker compose logs -f backend | grep -i okx`
3. 健康检查: `curl http://localhost:8080/api/v1/exchange/okx/ping -H "Authorization: Bearer <token>"`

### 5.2: 手动测试下单/撤单流程

**前置条件**:
- 拥有 OKX 测试账户 API Key
- API Key 已通过 `/api/v1/api-keys` 接口录入系统

**测试用例**:

| # | 操作 | 预期结果 |
|---|------|----------|
| 1 | GET /exchange/okx/ping | `{"code":0,"data":{"status":"ok",...}}` |
| 2 | GET /exchange/okx/account | 返回余额列表 |
| 3 | POST /exchange/okx/order (限价单) | 返回 order_id, status=live |
| 4 | GET /exchange/okx/orders/pending | 返回刚才下的挂单 |
| 5 | DELETE /exchange/okx/order/{id} | 返回 status=cancelled |
| 6 | POST /exchange/okx/order (市价单) | 返回 order_id, status=filled |

**测试脚本**:
```bash
# 1. 获取 token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"xxx"}' | jq -r '.data.access_token')

# 2. Ping 测试
curl -X GET http://localhost:8080/api/v1/exchange/okx/ping \
  -H "Authorization: Bearer $TOKEN"

# 3. 账户余额
curl -X GET http://localhost:8080/api/v1/exchange/okx/account \
  -H "Authorization: Bearer $TOKEN"

# 4. 下单 (symbol 使用 Binance 格式)
curl -X POST http://localhost:8080/api/v1/exchange/okx/order \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"symbol":"BTCUSDT","side":"buy","type":"limit","price":"60000","quantity":"0.01"}'

# 5. 撤单
curl -X DELETE http://localhost:8080/api/v1/exchange/okx/order/123456 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"symbol":"BTCUSDT"}'
```

---

## 任务清单 (Checklist)

- [ ] **T0**: 需求确认 + 架构设计
  - [ ] 确认 OKX API 端点列表
  - [ ] 确认 Symbol 转换规则
  - [ ] 创建 `utils/symbol.rs`

- [ ] **T1**: PRD Review (review-required)
  - [ ] API endpoints 审查
  - [ ] 数据库模型审查
  - [ ] 前后端契约审查

- [ ] **T2**: 实现
  - [ ] `handlers/exchange.rs` 新增 OKX handlers
  - [ ] `main.rs` 注册 `SignedOkxClient`
  - [ ] 路由映射 `/api/v1/exchange/okx/*`
  - [ ] Symbol 格式转换层

- [ ] **T3**: 前后端契约审查
  - [ ] API 格式文档
  - [ ] 前端调用示例

- [ ] **T4**: 单元测试
  - [ ] Symbol 转换测试
  - [ ] Mock `SignedOkxClient` 测试
  - [ ] Order creation 测试

- [ ] **T5**: 部署验证
  - [ ] 本地 docker compose 测试
  - [ ] 手动测试下单/撤单流程

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `utils/symbol.rs` | 新建 | Symbol 转换工具函数 |
| `handlers/exchange.rs` | 修改 | 新增 OKX handlers |
| `main.rs` | 修改 | 注册 SignedOkxClient + 路由 |
| `Cargo.toml` | 无需修改 | 依赖已存在 |

## 依赖检查

```bash
cd /home/ssk/workspace/quant-trading/backend
cargo check  # 验证 SignedOkxClient 可编译
```