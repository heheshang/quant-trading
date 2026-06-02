# T7 — P1-2.2 Bracket Order Operations Runbook

## Service Overview
- **What it does**: A bracket order is a parent limit order that, when fully filled,
  records SL/TP prices in `bracket_links` for the frontend to attach as OCO.
- **Path A (deferred)**: Backend does NOT auto-attach OCO on fill (would require
  a position row which is out of P1-2.2 scope). Frontend polls
  `GET /api/v1/bracket-links/pending` and decides when to call
  `POST /api/v1/trigger-orders/oco`.

## Key Files
- `backend/src/models/bracket_params.rs` — Type-safe wrapper around `advanced_params` JSONB
- `backend/src/services/bracket.rs` — `record_parent_filled` (DB insert) + `list_pending_for_user` + `update_parent_oco_status`
- `backend/src/handlers/bracket.rs` — `GET /api/v1/bracket-links/pending`
- `backend/src/handlers/order.rs` — `create_order` Bracket branch (validation + persistence)
- `backend/src/services/matching_engine.rs` — `flush_trades` hook calls `record_parent_filled` on parent fill
- `backend/migrations/20260602120000_bracket_links.sql` — Table schema

## Common Tasks

### Check Pending Bracket Links (PostgreSQL)
```bash
docker exec quant-postgres psql -U quant -d quant_trading -c \
  "SELECT id, parent_order_id, symbol, sl_price, tp_price, side, filled_quantity, oco_status, created_at \
   FROM bracket_links WHERE oco_status = 'pending' ORDER BY created_at DESC LIMIT 50;"
```

### Check Pending Links for a Specific User
```bash
docker exec quant-postgres psql -U quant -d quant_trading -c \
  "SELECT * FROM bracket_links WHERE user_id = '<UUID>' AND oco_status = 'pending';"
```

### Manually Mark a Bracket Link as `oco_attached` (Admin / Recovery)
```bash
docker exec quant-postgres psql -U quant -d quant_trading -c \
  "UPDATE bracket_links SET oco_status = 'oco_attached', updated_at = NOW() \
   WHERE id = '<UUID>';"
```

### Health Check
```bash
curl -s http://localhost:8080/health | jq .
curl -s http://localhost:8080/metrics | grep bracket
```

## Metrics
- `BRACKET_PARENT_FILLED_TOTAL{side="buy|sell"}` — Counter, increments on every parent fill that records a link

## Common Issues

### Issue 1: Bracket order returns 400 "stop_loss_price required"
**Cause**: Bracket requires both `stop_loss_price` and `take_profit_price` in the request body.
**Fix**: Add both fields (e.g., `"stop_loss_price": "49000"`).

### Issue 2: Bracket order returns 400 "stop_loss_price must be < entry_price for buy"
**Cause**: Validation rules (path A design):
- Buy: SL < entry, TP > entry
- Sell: SL > entry, TP < entry
**Fix**: Reverse the prices.

### Issue 3: `bracket_links` table not found
**Cause**: Migration `20260602120000_bracket_links.sql` was not applied.
**Fix**:
```bash
docker exec quant-postgres psql -U quant -d quant_trading -f /path/to/20260602120000_bracket_links.sql
```

## Recovery Procedures

### R1: Backend started but `bracket_links` not auto-created on fill
**Symptom**: After a bracket parent fills, `GET /bracket-links/pending` returns empty.
**Check**:
1. Inspect the `orders.advanced_type` column — should be `bracket`.
2. Check backend logs for `Bracket record_parent_filled failed for {uuid}`.
3. Verify `bracket_links` table exists (`\dt bracket_links`).
**Fix**: Apply migration if missing, restart backend.

### R2: Frontend stuck polling, no OCO created
**Symptom**: `bracket_links` row stuck in `pending` status.
**Cause**: Frontend not calling `POST /trigger-orders/oco` (network issue, JWT expired, etc.).
**Recovery**: Manually create OCO via:
```bash
curl -X POST http://localhost:8080/api/v1/trigger-orders/oco \
  -H "Authorization: Bearer <JWT>" \
  -H "Content-Type: application/json" \
  -d '{
    "position_id": "<UUID>",
    "symbol": "BTC/USDT",
    "side": "sell",
    "quantity": 0.5,
    "stop_loss_price": 49000,
    "take_profit_price": 52000
  }'
```

## Capacity / Limits
- Pending bracket links per user: bounded by `LIMIT 200` in `list_pending_for_user`
- SL/TP price: must be > 0
- Validation: 0 < SL/TP, side-specific (see Issue 2)

## Related Tasks
- P1-F2 — Stop loss / take profit fields already in `CreateOrderRequest` (reused)
- P1-F3 — OCO trigger endpoint (`POST /trigger-orders/oco`) — frontend uses this
- P1-2.1 — Iceberg (sibling feature in `docs/p1-2-1-iceberg/`)
- P1-2.2v2 — Future: auto-attach OCO on fill (requires `positions` table, P0 epic)
