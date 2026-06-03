# T3 — OKX Exchange Frontend-Backend Contract

## Common response wrapper

All endpoints return the standard project envelope:

```json
{
  "code": 0,            // 0 = success, non-0 = error
  "data": { ... },      // business payload
  "message": "success"  // human-readable
}
```

## Error codes (OKX-specific extensions)

| code | HTTP | Meaning |
|---|---|---|
| 0 | 200 | Success |
| 40001 | 400 | Bad request (validation failed) |
| 40101 | 401 | JWT missing/invalid |
| 40301 | 403 | API key not found for user / exchange |
| 50011 | 502 | OKX reports invalid API key (forwarded) |
| 51008 | 400 | OKX reports insufficient balance (forwarded) |
| 51119 | 404 | OKX reports order does not exist (forwarded) |
| 59999 | 502 | Upstream OKX API error (network/timeout/5xx) |

## Endpoints

### 1. `GET /api/v1/exchange/okx/ping`

**Auth:** JWT required.

**Request:** none.

**Response 200:**

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

### 2. `GET /api/v1/exchange/okx/account`

**Auth:** JWT required. Looks up the user's OKX API key from `ApiKeyStore`.

**Request:** none.

**Response 200:**

```json
{
  "code": 0,
  "data": {
    "balances": [
      { "asset": "BTC", "free": "1.5", "locked": "0.1" },
      { "asset": "USDT", "free": "10000", "locked": "0" }
    ]
  },
  "message": "success"
}
```

**Error 403** if user has no OKX API key configured:

```json
{ "code": 40301, "message": "No OKX API key configured for this user" }
```

### 3. `POST /api/v1/exchange/okx/order`

**Auth:** JWT required.

**Request body:**

```json
{
  "symbol": "BTC-USDT",       // OKX format (hyphenated)
  "side": "buy",
  "order_type": "limit",      // "limit" | "market"
  "quantity": "0.1",          // optional for market orders
  "price": "60000",           // optional for market orders
  "time_in_force": "gtc"      // "gtc" | "ioc" | "fok" (limit only)
}
```

**Response 200 (filled):**

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
      { "price": "60000", "qty": "0.1", "commission": "0.0001" }
    ]
  },
  "message": "success"
}
```

**Response 200 (open):**

```json
{
  "code": 0,
  "data": {
    "order_id": "123457",
    "symbol": "BTC-USDT",
    "side": "buy",
    "order_type": "limit",
    "status": "open",
    "executed_qty": "0",
    "fills": []
  },
  "message": "success"
}
```

### 4. `DELETE /api/v1/exchange/okx/order/{orderId}`

**Auth:** JWT required.

**Path params:** `orderId` (OKX ordId string).

**Request body:**

```json
{ "symbol": "BTC-USDT" }
```

**Response 200:**

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

**Error 404** if orderId does not exist on OKX.

### 5. `GET /api/v1/exchange/okx/orders/pending`

**Auth:** JWT required.

**Request:** none.

**Response 200:**

```json
{
  "code": 0,
  "data": {
    "orders": [
      {
        "order_id": "123456",
        "symbol": "BTC-USDT",
        "side": "buy",
        "order_type": "limit",
        "price": "60000",
        "qty": "0.1",
        "status": "live"
      }
    ]
  },
  "message": "success"
}
```

## TypeScript contract (frontend reference)

```typescript
// types/okx.ts
export interface OkxPingResponse {
  server_time: number;
  status: string;
}

export interface OkxBalance {
  asset: string;
  free: string;
  locked: string;
}

export interface OkxAccountResponse {
  balances: OkxBalance[];
}

export type OkxOrderType = 'limit' | 'market';
export type OkxSide = 'buy' | 'sell';
export type OkxTimeInForce = 'gtc' | 'ioc' | 'fok';
export type OkxOrderStatus = 'filled' | 'open' | 'cancelled' | 'rejected';

export interface OkxCreateOrderRequest {
  symbol: string;          // "BTC-USDT"
  side: OkxSide;
  order_type: OkxOrderType;
  quantity?: string;
  price?: string;
  time_in_force?: OkxTimeInForce;
}

export interface OkxOrderResponse {
  order_id: string;
  symbol: string;
  side: OkxSide;
  order_type: OkxOrderType;
  status: OkxOrderStatus;
  executed_qty: string;
  fills: Array<{ price: string; qty: string; commission: string }>;
}

export interface OkxCancelResponse {
  order_id: string;
  status: 'cancelled';
}

export interface OkxPendingOrder {
  order_id: string;
  symbol: string;
  side: OkxSide;
  order_type: OkxOrderType;
  price: string;
  qty: string;
  status: string;
}

export interface OkxPendingOrdersResponse {
  orders: OkxPendingOrder[];
}
```

## Symbol format (frontend rule of thumb)

- **Binance endpoints** (`/exchange/binance/*`): use `BTCUSDT` (concatenated)
- **OKX endpoints** (`/exchange/okx/*`): use `BTC-USDT` (hyphenated)

Do not pass a Binance-format symbol to an OKX endpoint — OKX will return 51119 (instrument not found).
