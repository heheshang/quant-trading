# T8 — P1-2.2 Bracket API Documentation

## Overview
Bracket orders are a **P1-2.2 advanced order type** (path A: deferred OCO).
The user-facing creation API reuses the existing `POST /api/v1/orders` endpoint
with `order_type = "bracket"`. A new polling endpoint exposes pending bracket
links so the frontend can attach OCO trigger orders.

## Authentication
All endpoints require `Authorization: Bearer <jwt>` (P1-F1 access token).

---

## POST `/api/v1/orders` (Bracket variant)

Create a bracket parent order (entry limit order with SL/TP metadata).

### Request Body
```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "bracket",
  "price": "50000",
  "quantity": "0.5",
  "stop_loss_price": "49000",
  "take_profit_price": "52000",
  "time_in_force": "GTC"
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `symbol` | string | yes | Must be in `symbol_configs.enabled = true` |
| `side` | string | yes | `"buy"` or `"sell"` |
| `order_type` | string | yes | **Must be `"bracket"`** |
| `price` | string (decimal) | yes | Parent limit price. Must be > 0 |
| `quantity` | string (decimal) | yes | Must be > 0 |
| `stop_loss_price` | string (decimal) | yes | **New (reused from P1-F2)**. Must satisfy side-specific rules |
| `take_profit_price` | string (decimal) | yes | **New (reused from P1-F2)**. Must satisfy side-specific rules |
| `time_in_force` | string | no | `"GTC"`, `"IOC"`, or `"FOK"`. Default `"GTC"` |

### Validation Rules (Path A)
- **Buy bracket**: `sl_price < entry_price < tp_price`
- **Sell bracket**: `tp_price < entry_price < sl_price`
- All prices must be > 0

### Response 201 Created
```json
{
  "code": 0,
  "data": {
    "order_id": "1d9180d8-db11-4a82-9250-154203d51ea7",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "bracket",
    "price": "50000.00000000",
    "stop_price": null,
    "quantity": "0.50000000",
    "filled_quantity": "0.00000000",
    "avg_fill_price": null,
    "status": "pending",
    "mode": "paper",
    "fee": "0.00000000",
    "created_at": "2026-06-02T05:24:57.319054+00:00",
    "updated_at": "2026-06-02T05:24:57.319054+00:00"
  },
  "message": "success"
}
```

### Error Responses
- **400** `{"code": 40001, "message": "Bracket order requires stop_loss_price"}`
- **400** `{"code": 40001, "message": "stop_loss_price (49000) must be < entry_price (50000) for buy bracket"}`
- **400** `{"code": 40001, "message": "Bracket requires a numeric price"}`
- **400** `{"code": 40005, "message": "Symbol not found: BTC/USDT"}` (if symbol disabled)

### Database Side Effects
- Inserts one row in `orders` with `advanced_type = "bracket"`, `order_type = "bracket"`, and `advanced_params = {"oco_status":"pending", "stop_loss_price":..., "take_profit_price":...}`.
- On full fill, the matching engine's `flush_trades` hook inserts a `bracket_links` row (see below).

---

## GET `/api/v1/bracket-links/pending`

Returns all `pending` bracket links for the authenticated user. Frontend polls
this endpoint and, for each link, calls `POST /trigger-orders/oco` to attach the
OCO.

### Request
No body or query params.

### Response 200 OK
```json
{
  "code": 0,
  "data": {
    "user_id": "abc...uuid",
    "count": 1,
    "links": [
      {
        "id": "...",
        "parent_order_id": "1d9180d8-...",
        "user_id": "abc...uuid",
        "symbol": "BTC/USDT",
        "sl_price": 49000.0,
        "tp_price": 52000.0,
        "side": "buy",
        "filled_quantity": 0.5,
        "oco_status": "pending",
        "sl_trigger_id": null,
        "tp_trigger_id": null,
        "created_at": "2026-06-02T05:30:00+00:00",
        "updated_at": "2026-06-02T05:30:00+00:00"
      }
    ]
  },
  "message": "success"
}
```

### Behavior
- Returns up to 200 most recent pending links (`ORDER BY created_at ASC LIMIT 200`).
- Empty array `[]` if no pending links.

### Error Responses
- **401** `{"code": 40101, "message": "Missing or invalid token"}`

---

## Internal: `flush_trades` hook (Backend-only)

When a bracket parent is fully filled:
1. Matching engine's `flush_trades` reads `orders.advanced_type = "bracket"`.
2. Parses `advanced_params` as `BracketParams` (extracts SL/TP).
3. Calls `bracket::record_parent_filled` which:
   - INSERTs a row in `bracket_links` (status `pending`).
   - Increments `BRACKET_PARENT_FILLED_TOTAL{side="..."}` metric.
4. No OCO is auto-created (path A).

---

## Migration Required
```bash
# Apply on first deploy
docker exec -i quant-postgres psql -U quant -d quant_trading < \
  backend/migrations/20260602120000_bracket_links.sql
```

## See Also
- **P1-2.1 Iceberg** — `docs/p1-2-1-iceberg/api/T8_API.md` (sibling feature)
- **P1-F3 Trigger Orders** — `docs/p1-f3-trigger-orders/api/` (OCO endpoint details)
- **T7 Runbook** — `docs/p1-2-2-bracket/operations/T7_Runbook.md` (ops procedures)
