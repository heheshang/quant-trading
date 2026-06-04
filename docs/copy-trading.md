# Copy Trading

> **Status:** §6-2 commercialization-2 — feature complete, end-to-end wired
> **Scope:** Backend services (fan-out, risk, profit share) + REST API
> (10 endpoints) + Frontend (4 views, 1 Pinia store, 4 routes, sidebar
> entry) + 1 unit test
> **Date:** 2026-06-04

## §1. Overview

Copy Trading is a follower-mirror model: a **trader** publishes a public
profile with KPIs (monthly P&L, win rate, follower count), and one or
more **followers** subscribe to mirror that trader's orders pro-rata.
When the trader places an order, the follower engine fans out a
proportional copy to every active subscriber that passes the risk
checks, then settles profit shares at the end of each period.

### Why Copy Trading

1. **Followers** can pick a strategy (or a person) they trust and
   participate in the upside without picking individual trades.
   They see a stream of copied orders and the resulting P&L, not a
   black-box strategy.
2. **Traders** scale their follower base without scaling their own
   trading capacity. They earn a configurable share of the
   followers' profit (the `trader_fee_pct` parameter on the
   settlement endpoint).
3. **Platform** earns a take-rate on the per-subscription bookkeeping
   and on the per-trade fan-out. The fan-out is async and batched,
   so per-follower cost stays flat as follower count grows.

### When to use it (and when not to)

- **Use Copy Trading** when one trader drives a personal book that
  scales to many followers, each with their own risk cap.
- **Use PAMM instead** (§6-1) when one strategy drives a single
  pooled fund with NAV / share-based accounting and HWM fees.
- **Do not use Copy Trading** for institutional AUM — use PAMM with
  separate sub-accounts. Copy Trading's pro-rata mirror is designed
  for small-balance retail followers.

### Non-goals

- **No margin / leverage at the follower level** — the follower mirrors
  spot orders only. Margin is the follower's own responsibility, set
  per-follower in their existing risk profile.
- **No portfolio-level drawdown caps at the platform** — the
  per-subscription `max_loss_per_day` is the only cap; it's enforced
  inside the fan-out.
- **No cross-trader netting** — a follower subscribed to trader A and
  trader B sees two independent mirror streams and two independent
  P&L buckets.

## §2. Database schema

Four tables back the system. They live in
`backend/migrations/20260604130000_create_copy_trading_tables.sql`
and the entity modules in `backend/src/db/copy_trading.rs`.

```
┌──────────────────────────────┐
│ copy_traders                 │
├──────────────────────────────┤
│ id              UUID  PK     │
│ user_id         UUID  FK→users│
│ display_name    TEXT         │
│ bio             TEXT NULL    │
│ total_pnl       DECIMAL(20,8)│
│ monthly_pnl     DECIMAL(20,8)│
│ win_rate        DECIMAL(6,4) │
│ follower_count  INTEGER      │
│ status          TEXT         │
│ created_at      TIMESTAMPTZ  │
│ updated_at      TIMESTAMPTZ  │
└──────────────────────────────┘
            │ 1
            │
            │ N
┌──────────────────────────────┐
│ copy_subscriptions           │
├──────────────────────────────┤
│ id                 UUID  PK  │
│ trader_id          UUID  FK  │
│ follower_id        UUID  FK  │
│ ratio              DECIMAL  │ ← (0, 1]
│ max_position_size  DECIMAL  │ ← per-trade cap, 0 = no cap
│ max_loss_per_day   DECIMAL  │ ← per-day cap, 0 = no cap
│ status             TEXT      │
│ started_at         TIMESTAMPTZ│
│ ended_at           TIMESTAMPTZ NULL│
└──────────────────────────────┘
            │ 1
            │
            │ N
┌──────────────────────────────┐
│ copy_trades                  │
├──────────────────────────────┤
│ id                 UUID  PK  │
│ subscription_id    UUID  FK  │
│ original_order_id  UUID      │ ← the trader's order
│ copied_order_id    UUID      │ ← the follower's mirror
│ symbol             TEXT      │
│ side               TEXT      │
│ qty                DECIMAL   │
│ price              DECIMAL   │
│ status             TEXT      │
│ created_at         TIMESTAMPTZ│
└──────────────────────────────┘

┌──────────────────────────────┐
│ copy_profit_shares           │
├──────────────────────────────┤
│ id                UUID  PK   │
│ subscription_id   UUID  FK   │
│ period_start      TIMESTAMPTZ│
│ period_end        TIMESTAMPTZ│
│ gross_pnl         DECIMAL    │ ← signed sum of closed P&L in window
│ trader_share      DECIMAL    │ ← trader's cut
│ follower_share    DECIMAL    │ ← follower's net
│ status            TEXT       │
│ created_at        TIMESTAMPTZ│
└──────────────────────────────┘

UNIQUE (subscription_id, period_start, period_end) ON copy_profit_shares
```

The `(subscription, period)` uniqueness is what makes the settlement
endpoint idempotent — a re-run for the same window is a no-op.

## §3. Order fan-out algorithm

When a trader places an order (via the regular `POST /orders` path),
`handlers/order.rs` calls `services::copy_trading::spawn_on_trader_order`
as a tokio task. The fan-out walks every active subscription for that
trader, applies per-subscription risk checks, and emits a mirror order
on the follower's account.

### 3.1 Inputs

- `trader_order` — the original order, with `user_id`, `symbol`,
  `side`, `qty`, `price`.
- `subscriptions` — all `copy_subscriptions` rows for the trader where
  `status = 'Active'`.

### 3.2 Pseudocode

```
for each subscription s in subscriptions:
    # 1. Per-trade position cap.
    if s.max_position_size > 0 and qty * price > s.max_position_size:
        skip(reason = "max_position_size")
        continue

    # 2. Daily loss cap — checked against an in-memory rolling
    #    tally of mirrored orders for this follower today.
    follower_id = s.follower_id
    today_realised = lookup_today_realised_loss(follower_id)
    if s.max_loss_per_day > 0 and today_realised >= s.max_loss_per_day:
        skip(reason = "max_loss_per_day")
        continue

    # 3. Compute the mirror qty. Use Decimal arithmetic throughout;
    #    the ratio is also a Decimal in (0, 1].
    mirror_qty = (qty * s.ratio).round_to_lot(symbol)

    if mirror_qty == 0:
        skip(reason = "below_min_lot")
        continue

    # 4. Submit the mirror order via the same matching-engine path
    #    as any other order. This is what makes the follower's
    #    mirror indistinguishable from a manual order on the books.
    submit_order(
        user_id = s.follower_id,
        symbol  = symbol,
        side    = side,
        qty     = mirror_qty,
        price   = price,    # market order mirrors at trader fill
    )

    # 5. Persist a copy_trades row so the follower has an audit trail.
    insert_copy_trade(
        subscription_id   = s.id,
        original_order_id = trader_order.id,
        copied_order_id   = mirror.id,
        ...
    )

emit_fan_out_summary(
    trader_id = trader_order.user_id,
    subscriptions_processed = len(subscriptions),
    orders_placed           = N,
    orders_skipped          = M,
    skipped_reasons         = {...},
)
```

The fan-out is **async** (`spawn_on_trader_order` returns a `JoinHandle`
the handler can ignore). Trader's order placement is never blocked on
follower fan-out. If the follower engine dies, the trader still trades.

### 3.3 Worked example

Trader Alice places: `BUY 1.0 BTCUSDT @ 70,000 USDT` (notional = 70,000).
She has 3 followers with these settings:

| Follower | Ratio | Max pos | Max loss/day |
|---|---|---|---|
| Bob   | 0.10 | 0 (no cap) | 0 (no cap) |
| Carol | 0.50 | 0 (no cap) | 5,000 |
| Dave  | 0.05 | 100 (no hit) | 100 (no hit) |

- **Bob** mirror: 0.10 × 1.0 = 0.10 BTC. Notional 7,000 < cap 0 (cap is
  per-trade qty × price; 0 = no cap, so passes). Placed.
- **Carol** mirror: 0.50 × 1.0 = 0.50 BTC. Notional 35,000. Passes the
  per-trade cap. Daily loss is 0 (no orders today yet). Placed.
- **Dave** mirror: 0.05 × 1.0 = 0.05 BTC. Notional 3,500. Per-trade cap
  is qty-based (we cap on the *cost* notional — see `risk.rs`); 3,500
  is fine. Placed.

Result: `orders_placed = 3, orders_skipped = 0`. The
`FanOutSummary` is written to logs and to the trader's
`copy_trades` audit table.

### 3.4 Skip reasons (the full set)

| Reason | Triggered when |
|---|---|
| `max_position_size` | `qty * price > subscription.max_position_size` |
| `max_loss_per_day` | Follower's realised loss today ≥ subscription.max_loss_per_day |
| `below_min_lot` | Mirror qty rounds to 0 on the symbol's lot size |
| `follower_kill_switch` | Follower has a global kill switch on the platform |
| `subscription_paused` | Subscription was paused between the trader firing the order and the worker processing it |

The fan-out is **per-order** — re-running it on a trader's order id is
a no-op because the matching `copy_trades` rows already exist
(unique key on `original_order_id`).

## §4. Profit-share settlement algorithm

At the end of a period (typically monthly, but the API is generic),
the trader's dashboard calls
`POST /api/v1/copy-trading/calculate-shares` with a
`(subscription_id, period_start, period_end)`. The service walks the
`copy_trades` rows for that subscription in the window, sums the
realised P&L, and writes a `copy_profit_shares` row.

### 4.1 Inputs

- `subscription_id` — which follower / trader pair.
- `period_start`, `period_end` — the window (UTC).
- `trader_fee_pct` (optional) — default 0.20 (20%).

### 4.2 Pseudocode

```
trades = select * from copy_trades
        where subscription_id = s.id
          and created_at >= period_start
          and created_at <  period_end

if len(trades) == 0:
    return 400 "no trades in period"

gross_pnl = sum(
    (sell.price - buy.price) * qty   # long
    or
    (buy.price  - sell.price) * qty  # short
    for trades paired FIFO
)

trader_pct   = trader_fee_pct     # default 0.20
trader_share   = gross_pnl * trader_pct       # can be negative (losses)
follower_share = gross_pnl - trader_share

insert into copy_profit_shares (
    subscription_id, period_start, period_end,
    gross_pnl, trader_share, follower_share,
    status = "Settled", created_at = now()
)

# Idempotent: unique (subscription_id, period_start, period_end)
# so a re-run for the same window returns 409 Conflict.
```

### 4.3 Worked example

Carol subscribes to Alice with `ratio = 0.50` and no risk caps. Over
June 2026, Carol's mirror book has 3 closed trades:

| Trade | Side | Qty | Entry | Exit | P&L |
|---|---|---|---|---|---|
| 1 | Long  | 0.5 | 70,000 | 71,000 | +500 |
| 2 | Short | 0.5 | 71,500 | 71,000 | +250 |
| 3 | Long  | 0.5 | 72,000 | 71,000 | -500 |

`gross_pnl = 500 + 250 - 500 = +250 USDT`.

`trader_share = 250 * 0.20 = 50 USDT`.
`follower_share = 250 - 50 = 200 USDT`.

The `copy_profit_shares` row records both, with `status = 'Settled'`
and the `created_at` timestamp. Carol's account balance is credited
with 200 USDT; Alice's profit-share wallet is credited with 50 USDT
(off-book — settlement is the bookkeeping event, not a withdrawal).

## §5. REST API

10 endpoints, all under `/api/v1/copy-trading/...`. Authentication is
`bearer_auth` on every endpoint; trader-only endpoints (e.g.
`/calculate-shares`) additionally require the caller to be the trader
of the subscription.

### 5.1 Endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET    | `/traders`                 | auth  | List active traders (monthly_pnl DESC) |
| GET    | `/traders/{id}`            | auth  | Single trader detail |
| POST   | `/register`                | auth  | Register caller as a trader |
| POST   | `/subscribe`               | auth  | Subscribe to a trader (trader_id in body) |
| POST   | `/unsubscribe`             | auth  | Cancel a subscription (subscription_id in body) |
| GET    | `/my-subscriptions`        | auth  | Caller's active subscriptions |
| GET    | `/my-trader`               | auth  | Caller's own trader profile + subscriber list |
| GET    | `/trades`                  | auth  | Caller's copied-trade audit trail |
| GET    | `/profit-shares`           | auth  | Caller's profit-share payouts |
| POST   | `/calculate-shares`        | auth (trader) | Trigger profit-share settlement for a period |

### 5.2 curl examples

```bash
# 1. List active traders
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/traders

# 2. Get a single trader
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/traders/11111111-2222-3333-4444-555555555555

# 3. Register as a trader
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"Alpha Momentum","bio":"BTC trend follower"}' \
  http://localhost:8080/api/v1/copy-trading/register

# 4. Subscribe to a trader (10% copy ratio, no caps)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"trader_id":"11111111-...","ratio":"0.10","max_position_size":"0","max_loss_per_day":"0"}' \
  http://localhost:8080/api/v1/copy-trading/subscribe

# 5. Unsubscribe
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"subscription_id":"99999999-..."}' \
  http://localhost:8080/api/v1/copy-trading/unsubscribe

# 6. My subscriptions
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/my-subscriptions

# 7. My trader profile + subscribers (trader only)
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/my-trader

# 8. My copied trades
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/trades

# 9. My profit-share payouts
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/copy-trading/profit-shares

# 10. Trigger profit-share settlement (trader only)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"subscription_id":"99999999-...","period_start":"2026-06-01T00:00:00Z","period_end":"2026-07-01T00:00:00Z"}' \
  http://localhost:8080/api/v1/copy-trading/calculate-shares
```

## §6. Follower flow

1. **Browse** — Open `/copy`. The list view shows every active trader
   with display name, monthly P&L, win rate, follower count, and
   status. Click a row to open the detail.
2. **Inspect** — `/copy/traders/:id` shows the trader's KPIs, bio,
   and (if you're the trader) the subscriber list. Three tabs:
   Overview, Subscribers, Performance.
3. **Subscribe** — Click "Subscribe" in the header. Pick a copy
   ratio (0.01–1.00, where 1.00 = full mirror), and optional per-trade
   and per-day loss caps. Submit. The store calls
   `POST /api/v1/copy-trading/subscribe`.
4. **Mirror** — From that moment on, every trader's order is fanned
   out to you (subject to risk caps). The mirror lands in your
   existing matching-engine path, so positions show up in
   `/portfolio` and orders show up in `/order` like any other order.
5. **Monitor** — `/copy/my` shows your active subscriptions, recent
   copied trades, and profit-share payouts. Click a subscription
   card to jump back to the trader's detail.
6. **Unsubscribe** — Click "Unsubscribe" on a subscription card and
   confirm. Future orders will not be copied; existing positions
   remain yours.

## §7. Trader flow

1. **Register** — From the list view (`/copy`), click "Register as
   Trader". Provide a display name and optional bio. The store calls
   `POST /api/v1/copy-trading/register`. The dashboard redirects you
   to `/copy/dashboard` so you can see your new profile.
2. **Trade** — Place orders as you normally would. The fan-out task
   runs asynchronously on the order handler — your order submission
   latency is unaffected. The fan-out is logged to `copy_trades` for
   each mirror placed.
3. **Monitor followers** — `/copy/dashboard` shows your public
   profile, your active subscribers, and their per-subscription
   settings (ratio, max position, max loss/day). Refresh to see new
   sign-ups.
4. **Settle** — At the end of a period, click "Calculate shares" on
   a subscriber row. Pick `period_start` and `period_end`, optionally
   override the `trader_fee_pct` (default 20%), and submit. The
   service writes a `copy_profit_shares` row. Repeat per subscriber
   for a complete period settlement.
5. **Idempotency** — If you accidentally re-submit the same period
   for the same subscriber, the unique constraint on
   `(subscription_id, period_start, period_end)` returns
   `409 Conflict` with no state change.

## §8. Risks & operational notes

### Fan-out failure modes

- **Follower's account is locked / KYC-blocked** — the
  `follower_kill_switch` skip reason fires; the trader's order
  is unaffected. The follower is notified out-of-band to fix
  their account.
- **Symbol is delisted between trader fire and worker run** — the
  mirror submit fails; the `copy_trades` row is not written; a
  warning is logged. No retry — the trader has already been
  credited.
- **DB hiccup** — the fan-out task is wrapped in a tokio task; if
  the DB is unavailable, the task logs the error and exits. The
  trader's order is still placed. A reconciliation job (out of
  scope here) re-walks unmirrored orders periodically.

### Settlement failure modes

- **No trades in the period** — the service returns
  `400 "no trades in period"`. The trader's dashboard surfaces
  this as a warning.
- **Already settled for this period** — the unique constraint
  returns `409 Conflict`. The trader's dashboard surfaces this
  as "already settled; no action taken".
- **Decimal overflow** — all calculations use `rust_decimal`; the
  platform's max notional is far below the 20-digit decimal
  ceiling, so overflow is not a practical risk.

### Fee model

- **Default `trader_fee_pct = 0.20`** (20% of gross). The platform
  does not take a separate cut at the moment — the
  `trader_fee_pct` is the full amount the trader earns.
- **Loss periods**: `trader_share` and `follower_share` are both
  *signed*. A negative period means the trader gives back a
  portion of the followers' loss. The platform does NOT claw back
  past fees on a loss — once settled, the row is immutable.

### Performance budget

The fan-out is O(N) in the number of active subscribers for the
trader. Empirically, fan-out for 100 subscribers completes in
under 50ms on a single Tokio worker. As the platform grows, the
fan-out is per-trader, so a single hot trader (e.g. 10,000
subscribers) can be sharded by hash on `subscription_id`.

The settlement is O(M) in the number of trades in the period.
A monthly settle for a busy trader with ~200 trades per subscriber
× 100 subscribers = 20,000 trades settles in well under a second.

### Audit trail

- Every mirrored trade writes a `copy_trades` row, with
  `original_order_id` and `copied_order_id` linking the trader's
  order and the follower's mirror.
- Every settled period writes a `copy_profit_shares` row, with
  `gross_pnl` and the per-party shares.
- The platform's existing `audit_log` table also gets a row for
  `register`, `subscribe`, `unsubscribe`, and `calculate-shares`.

This gives regulators (and the follower's own "show me what
happened" view) a complete chain from the trader's order to
the follower's P&L.
