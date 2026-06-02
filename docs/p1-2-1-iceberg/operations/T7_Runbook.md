# T7 — P1-2.1 Iceberg Order Operations Runbook

## Service Overview
- **What it does**: Splits a large parent order into smaller visible "child slices" to hide true order size in the order book.
- **How it works** (P1-2.1 design):
  1. User POSTs a parent order with `order_type=iceberg` and `visible_quantity` (slice size).
  2. The first child slice (qty = `visible_quantity`) is inserted into the in-memory matching book as a normal `limit` order with metadata `advanced_type=iceberg_child`.
  3. When the child fills, the matching engine's `flush_trades` hook calls `iceberg::append_next_child` which inserts the next slice (replenishment) and updates `filled_children` count in the parent's `advanced_params` JSONB.
  4. Final slice absorbs rounding remainder.
  5. Cancelling the parent cascades to all `active` children (`iceberg::cancel_iceberg_children`).

## Health Check
- **Backend health**: `GET /api/v1/health` → 200 OK
- **Metrics**: `GET /api/v1/metrics`
  - `iceberg_child_orders_total{action="created"|"filled"|"cancelled"|"skipped"}`
- **DB state check** (one-liner):
  ```sql
  SELECT order_type, advanced_type, COUNT(*), SUM(quantity), SUM(filled_quantity)
  FROM orders
  WHERE advanced_type LIKE 'iceberg%' OR order_type = 'iceberg'
  GROUP BY order_type, advanced_type;
  ```

## Common Issues

### Issue 1: `column orders.advanced_type does not exist`
**Symptom**: Backend logs `Database error: error returned from database: column orders.advanced_type does not exist` on any `/api/v1/orders` POST.
**Root cause**: P1-2 migration `20260602000000_advanced_order_types.sql` was not applied.
**Fix** (one-off):
```sql
ALTER TABLE orders ADD COLUMN IF NOT EXISTS advanced_params JSONB;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS advanced_type TEXT;
CREATE INDEX IF NOT EXISTS idx_orders_advanced_type ON orders(advanced_type) WHERE advanced_type IS NOT NULL;
```
The next backend restart will pick up the migration and the columns will exist for new deploys.

### Issue 2: Iceberg parent order is `pending` but no child in book
**Symptom**: `GET /api/v1/orders/{id}` shows status=pending for an iceberg order, but no `advanced_type=iceberg_child` row exists in the DB.
**Root cause**: The first child insertion failed silently (likely insufficient balance, or symbol not tradable, or engine not initialized).
**Debug**:
```sql
SELECT id, order_type, advanced_type, status, reject_reason
FROM orders
WHERE id = '<parent_id>' OR advanced_type = 'iceberg_child';
```
Check `reject_reason` on the parent.

### Issue 3: Stuck in "partially_filled" indefinitely
**Symptom**: Iceberg parent `filled_quantity < quantity` but no new child slices appear.
**Root cause**: Either matching engine is down, or `flush_trades` hook panicked (check logs for `Iceberg replenish failed`).
**Fix**: Restart backend. On restart, `rebuild_order_book` will reload active children from the DB. The first child (already in book) will continue matching. The next slice in the chain will be appended by the `append_next_child` hook the next time a child fills.

## Capacity / Limits
- **Max slices per parent**: 10 (configurable via `params.total_slices` calculation; round up `total / visible`).
- **Min `visible_quantity`**: 1 satoshi equivalent (any positive f64).
- **Max children in book simultaneously**: bounded by `MatchingEngine` order book capacity (in-memory).
- **Replenishment latency**: 100ms (the `flush_trades` batch flush interval).

## Disaster Recovery
- **DB corruption** in `advanced_params` JSONB: re-derive `total_slices` from `quantity / visible_quantity` rounded up; reset `filled_children` to 0; delete orphan children.
- **Engine state drift**: restart backend; `rebuild_order_book` re-loads active children from DB.
- **Migration rollback** (rare, only if P1-2.1 needs revert): there is no down-migration; the columns are nullable, so dropping the columns is safe but loses parent-child relationships.
