# T7 — Trailing Stop API Reference

## POST /api/v1/orders

**Purpose:** Create a new trailing stop order.

**Request body:**

```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "trailing_stop",
  "price": "60000.0",
  "quantity": "0.001",
  "advanced_type": "trailing_stop",
  "trailing_distance": "0.5"
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `symbol` | string | yes | Must be in `symbol_configs` and `enabled=true` (e.g. `BTC/USDT`) |
| `side` | `buy`/`sell` | yes | |
| `order_type` | string | yes | Must be `trailing_stop` |
| `price` | decimal string | yes | Initial reference price (peak_price seed) |
| `quantity` | decimal string | yes | Must satisfy `min_quantity` and `min_notional` |
| `advanced_type` | string | yes | `trailing_stop` |
| `trailing_distance` | percent string | yes | e.g. `"0.5"` = 0.5%. Range (0, 10). Stored as fraction 0.005 |

**Response 201:**

```json
{
  "code": 0,
  "data": {
    "order_id": "57b7e81a-e406-4dfa-8038-56184384fe0b",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "trailing_stop",
    "price": "60000.00000000",
    "quantity": "0.00100000",
    "status": "pending"
  }
}
```

**Errors:**
- `40001` — Bad request: missing `trailing_distance`
- `40001` — `trailing_distance` out of (0, 10) range
- `40001` — `trailing_distance` invalid decimal
- `40004` — Symbol not tradable
- `401` — Unauthenticated

## GET /api/v1/orders/{id}

Standard order response. `advanced_params.peak_price` is updated in-place by the background poll loop every 2s. To see the live peak_price, query the DB directly:

```sql
SELECT advanced_params->>'peak_price' FROM orders WHERE id = '<order_id>';
```

## Trigger behavior

- **Buy:** `peak_price` monotonically increases. Triggers when `current_price ≤ peak × (1 − distance)`. Order marked `filled`, no auto-close order (v2 will add).
- **Sell:** `peak_price` monotonically decreases. Triggers when `current_price ≥ peak × (1 + distance)`. Same fill semantics.

## Example session

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"Pass123!"}' | jq -r .data.access_token)

# Create 0.5% trailing buy at $60,000
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{
    "symbol":"BTC/USDT","side":"buy","order_type":"trailing_stop",
    "price":"60000.0","quantity":"0.001",
    "advanced_type":"trailing_stop","trailing_distance":"0.5"
  }'
# → 201 {order_id, status: "pending", ...}

# Wait 3 seconds, then inspect peak_price in DB
sleep 3
psql -c "SELECT advanced_params->>'peak_price' FROM orders WHERE order_type='trailing_stop' AND status='pending';"
# → 78118.12 (current BTC market price)
```
