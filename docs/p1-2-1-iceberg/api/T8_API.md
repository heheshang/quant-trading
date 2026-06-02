# T8 — P1-2.1 Iceberg API Documentation

## Overview
Iceberg orders are a **P1-2.1 advanced order type**. The user-facing API uses the same `/api/v1/orders` endpoint as limit/market orders; only the request body and the resulting child-order semantics differ.

## Create an Iceberg Order

### Request
```http
POST /api/v1/orders
Authorization: Bearer <jwt_access_token>
Content-Type: application/json
```

```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "iceberg",
  "price": "50000.00",
  "quantity": "10.0",
  "visible_quantity": "3.0",
  "time_in_force": "GTC"
}
```

### Required Fields
| Field | Type | Description |
|---|---|---|
| `symbol` | string | Trading pair, e.g. `BTC/USDT`. Must be in `/api/v1/symbols` list. |
| `side` | string | `buy` or `sell`. |
| `order_type` | string | **Must be exactly `"iceberg"`** (case-sensitive). |
| `price` | string (decimal) | Limit price per unit. |
| `quantity` | string (decimal) | **Total** quantity you want to fill across all slices. |
| `visible_quantity` | string (decimal) | Per-slice visible quantity. Must be `> 0` and `<= quantity`. |

### Optional Fields
| Field | Type | Description |
|---|---|---|
| `time_in_force` | string | `GTC` (default) / `IOC` / `FOK`. FOK is not valid for iceberg. |
| `strategy_id` | uuid | Optional strategy linkage. |

### Response (201 Created)
```json
{
  "code": 0,
  "data": {
    "order_id": "803ba807-256d-4796-bfce-440ce0858dc4",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "iceberg",
    "price": "50000.00000000",
    "quantity": "10.00000000",
    "filled_quantity": "0.00000000",
    "status": "pending",
    "created_at": "2026-06-02T04:29:05Z"
  },
  "message": "success"
}
```

### Errors
| HTTP | `code` | Scenario |
|---|---|---|
| 400 | 40004 | `Symbol not tradable: <symbol>` |
| 400 | 40010 | `visible_quantity must be > 0 and <= quantity` |
| 400 | 40010 | `quantity must be > 0` |
| 400 | 40010 | `Iceberg requires limit price` (price=null) |
| 401 | 40101 | Missing/invalid JWT |
| 402 | 40201 | Insufficient paper balance |
| 500 | 50002 | DB error (check T7 Issue 1) |

## Order Lifecycle

### Status Transitions
```
pending  ──►  active  ──►  filled
   │             │  ╲
   │             ▼   ▼
   └────► cancelled  partially_filled ──► filled
```

### Child Slice Behavior
- **First child** (`visible_quantity`) is inserted into the matching engine immediately on order creation.
- **Subsequent children** are appended (one at a time) by the `flush_trades` hook when the previous child fills. Appending is **lazy** — you do not see all 10 child rows at creation time, only the first.
- **Final slice** absorbs rounding remainder (e.g. qty=10, visible=3 → 4 slices of 3+3+3+1).

### Querying Children
```http
GET /api/v1/orders?symbol=BTC/USDT
```
Returns both the parent (`order_type=iceberg`) and any active children (`order_type=limit`, `advanced_type=iceberg_child` in the DB; the public API does not currently expose `advanced_type` — see **T9 Decision 13**).

## Cancel an Iceberg Order

### Request
```http
DELETE /api/v1/orders/{parent_id}
Authorization: Bearer <jwt_access_token>
```

### Response
```json
{
  "code": 0,
  "data": {
    "order_id": "803ba807-256d-4796-bfce-440ce0858dc4",
    "status": "cancelled",
    "cancelled_at": "2026-06-02T04:30:00Z"
  },
  "message": "success"
}
```

### Side Effects
- All `active` and `pending` children are cancelled.
- The parent is marked `cancelled`.
- The parent's balance (frozen at creation) is unfrozen.
- **Children that have already filled are NOT reversed** (trade history preserved).

## Examples

### Create + Wait + List (curl)
```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"Test1234!"}' | jq -r .data.access_token)

curl -s -X POST http://localhost:8080/api/v1/orders \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "symbol":"BTC/USDT",
    "side":"buy",
    "order_type":"iceberg",
    "price":"50000.00",
    "quantity":"10.0",
    "visible_quantity":"3.0"
  }'
```

## Versioning
- **Added in**: v0.3.0 (P1-2.1, June 2026)
- **Stability**: stable for create/cancel. The `advanced_type` field is a **backend-internal** discriminator; do not build client logic that depends on it.
