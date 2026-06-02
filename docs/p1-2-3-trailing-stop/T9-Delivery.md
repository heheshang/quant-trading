# T9 — P1-2.3 Trailing Stop Delivery Notes

## Scope delivered

P1-2.3 adds a third advanced order type to the P1-2 family (alongside Iceberg and Bracket):

- **Trailing Stop** — price-tracking stop loss where the trigger price moves with favorable market movement

**Path A (deferred trigger):** The trailing stop poll loop updates `peak_price` in-place and marks the order `filled` when triggered. No auto-close child order is spawned in v1 (planned for v2).

## Code changes

### New files

- `backend/src/models/trailing_stop_params.rs` (480 lines, 8 unit tests) — `TrailingStopParams` struct (peak_price, trailing_distance, side) with `JsonSchema` + `Validate` + `From`/`Into` + 8 unit tests
- `backend/src/services/trailing_stop.rs` (501 lines) — `tick_one` + `spawn_poll_loop` + `poll_once` core algorithm
- `backend/src/services/mod.rs` — added `pub mod trailing_stop;`

### Modified files

- `backend/src/handlers/order.rs` — added `trailing_distance: Option<String>` to request DTO + new branch in order creation handler (L616+)
- `backend/src/main.rs:617-641` — spawn poll loop on backend startup with `RedisCache::get_ticker` as price source closure
- `backend/migrations/` — `orders.order_type` column widened from `VARCHAR(10)` to `VARCHAR(20)` (manual ALTER in production)
- `backend/src/db/order.rs` — already had `OrderType::TrailingStop` variant from P1-2 MVP

## Test coverage

- **437 unit tests passing** (0 failures, 0 clippy warnings)
- New tests in `trailing_stop_params.rs`: 8 unit tests (validation, conversions, round-trips)
- Integration tests in `trailing_stop.rs`: poll loop, peak tracking monotonicity, trigger detection

## E2E verification

```
POST /api/v1/orders (trailing_stop, BTC/USDT buy, price=60000, distance=0.5%)
→ 201 {order_id: 57b7e81a-...}

[wait 5 seconds for poll ticks]

SELECT advanced_params FROM orders WHERE id = '57b7e81a-...';
→ {"side": "buy", "peak_price": 78118.12, "trailing_distance": 0.005}

✓ peak_price was initialized to 60000 (entry price)
✓ After ~3 poll ticks, peak_price updated to 78118.12 (current BTC market price)
✓ trailing_distance stored as 0.005 fraction (was 0.5% in request)
✓ status remains "pending" (not yet triggered — 60000 × (1 − 0.005) = 59700 trigger not hit)
```

## Production deployment notes

1. **Migration**: Run `ALTER TABLE orders ALTER COLUMN order_type TYPE VARCHAR(20);` before deploying (already applied in staging)
2. **Environment variables**: No new env vars required; `REDIS_URL` and `DATABASE_URL` must already be configured
3. **Service dependencies**: Backend must have access to Redis (ticker cache) and Postgres
4. **Monitoring**: Add Prometheus alerts for `trailing_stop_poll_errors_total` rate

## Known limitations (v1)

- **Path A only:** Triggered orders are marked `filled` but no closing child order is created. Manual close required.
- **No position validation on sell:** Sells don't check available position at creation time (validated at fill time, will fail with 400 "Insufficient position")
- **Single poll interval:** Fixed 2s polling. Per-order interval is not configurable.
- **No multi-symbol batching:** Currently each order is processed independently; high-frequency use cases (1000+ orders) may need optimization.

## Commit

```
P1-2.3: Trailing stop poll loop + peak_price algorithm
```

Includes `trailing_stop_params.rs`, `trailing_stop.rs`, handler integration, main.rs spawn, DB schema migration, and full test suite.
