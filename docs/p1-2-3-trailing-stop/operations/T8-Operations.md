# T8 — Trailing Stop Operations Runbook

## Startup

The trailing stop poll loop is auto-spawned in `main.rs:624` at backend startup. No manual action required.

**Verify it's running:**

```bash
# Hit /health
curl http://localhost:8080/health
# → 200 OK

# Check backend logs for poll activity
journalctl -u quant-trading-backend -f | grep "trailing_stop"
# (no "spawn_poll_loop started" log line — the loop is silent when no orders)
```

**Verify poll loop is processing orders:**

```bash
# Create a test order
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Authorization: Bearer *** -H 'Content-Type: application/json' \
  -d '{"symbol":"BTC/USDT","side":"buy","order_type":"trailing_stop",
       "price":"60000","quantity":"0.001",
       "advanced_type":"trailing_stop","trailing_distance":"0.5"}'

# Wait 5 seconds, then query DB
psql -c "SELECT advanced_params->>'peak_price' FROM orders WHERE order_type='trailing_stop' AND status='pending' ORDER BY created_at DESC LIMIT 1;"
# → Should match current BTC market price (~$78,000 range), not 60000
```

## Configuration

| Env var | Default | Purpose |
|---|---|---|
| `REDIS_URL` | (required) | Redis connection string with auth. Tickers cached under `ticker:{SYMBOL}` hashes (e.g. `ticker:BTCUSDT`). |
| `DATABASE_URL` | (required) | Postgres connection. Schema must have `orders.advanced_params JSONB` and `orders.advanced_type VARCHAR`. |

The `orders.order_type` column was widened from `VARCHAR(10)` to `VARCHAR(20)` to accommodate the 13-character `trailing_stop` value:

```sql
ALTER TABLE orders ALTER COLUMN order_type TYPE VARCHAR(20);
```

## Monitoring

**Key metrics to watch (Prometheus endpoint at `/metrics`):**

- `trailing_stop_orders_pending` — count of active trailing stop orders (gauge)
- `trailing_stop_poll_duration_seconds` — histogram of poll loop cycle time
- `trailing_stop_poll_errors_total` — counter of poll loop errors

**Alerts (recommended, not yet configured):**

- `trailing_stop_poll_errors_total` increasing rapidly → investigate Redis connectivity
- Backend `/health` failing for >1 min → trailing stops are NOT being ticked

## Failure modes

| Symptom | Likely cause | Mitigation |
|---|---|---|
| `peak_price` stuck at creation price | Redis `ticker:BTCUSDT` not updating | Check `ws_hub` is connected to Binance WS; check `redis_cache.set_ticker` is being called |
| 401 on poll price fetch | Old code path used `GET /api/v1/market/tickers` (auth required) | Ensure `main.rs:617+` uses `RedisCache::get_ticker` not HTTP loopback |
| `peak_price` not monotonically moving toward favorable direction | Bug in `update_peak` (frozen in time) | Restart backend; investigate tick logic |

## Cleanup / Disable

To disable trailing stops temporarily without redeploying:

1. Cancel all pending trailing stop orders via existing cancel endpoint
2. Comment out `spawn_poll_loop` in `main.rs:617-641` and rebuild
3. Existing orders remain `pending` but won't tick (can be cancelled manually)

## Backups

- Order state lives in `orders` table — already covered by existing Postgres backup
- `advanced_params` JSONB contains `peak_price` + `trailing_distance` + `side` — safe to lose (orders can be cancelled)
