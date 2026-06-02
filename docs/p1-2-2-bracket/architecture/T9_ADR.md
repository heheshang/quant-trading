# T9 — P1-2.2 Bracket Architecture Decision Records

## Status
- **Date**: 2026-06-02
- **Author**: ssk
- **Status**: Accepted (path A — deferred OCO)
- **Supersedes**: P1-2 MVP stub (`OrderType::Bracket` accepted but no business logic)

## Context

P1-2.2 is the third advanced order type in the P1-2 series (after P1-2.1 Iceberg).
The product PRD requires bracket orders (entry + SL + TP) for risk-managed
position opening. Three architecture paths were considered for how the OCO
trigger gets created:

1. **Path A (chosen)**: Backend records SL/TP in `bracket_links` on parent fill;
   frontend polls and calls existing `POST /trigger-orders/oco`.
2. **Path B**: Backend inserts `trigger_orders` rows directly on parent fill
   (no frontend involvement, but couples the matching engine to trigger service).
3. **Path C**: Backend auto-attaches OCO on fill by creating a `positions` row
   first (full automation, but requires P0 position-tracking infrastructure).

## Decisions

### D1: Path A (Deferred OCO) chosen
**Why**: Lowest blast radius. The P1-2 series is scoped to "create the order
types" — full position tracking is P0 epic scope. Path A decouples the matching
engine from the trigger service and gives the frontend a clean place to inject
business logic (e.g., "only attach OCO if user has TP-bot enabled").

**Trade-off accepted**: User must keep the frontend open (or poll via a bot) for
the OCO to be created. Documented in T7 runbook (R2 recovery procedure).

### D2: New table `bracket_links` (not `bracket_orders` extension)
**Why**: Decouples metadata lifecycle from `orders` table. `bracket_links` is
immutable history (one row per fill event), while `orders` mutates status
(`pending` → `filled` → `cancelled`).

**Schema**:
```sql
CREATE TABLE bracket_links (
    id              UUID PRIMARY KEY,
    parent_order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id),
    symbol          VARCHAR(32) NOT NULL,
    sl_price        DOUBLE PRECISION NOT NULL,
    tp_price        DOUBLE PRECISION NOT NULL,
    side            VARCHAR(8) NOT NULL,
    filled_quantity DOUBLE PRECISION NOT NULL,
    oco_status      VARCHAR(16) NOT NULL DEFAULT 'pending',
    sl_trigger_id   UUID,
    tp_trigger_id   UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### D3: Reuse P1-F2 SL/TP fields in `CreateOrderRequest`
**Why**: `stop_loss_price`, `take_profit_price`, and trigger mode fields already
exist from P1-F2 (the standalone stop-loss/take-profit feature). Adding new
fields would create a divergent API surface. The `order_type=bracket` field
disambiguates intent.

### D4: Validation in `models/bracket_params.rs` (Pydantic-style wrapper)
**Why**: Same pattern as `models/iceberg_params.rs` (P1-2.1). Keeps validation
logic out of the handler and testable as a unit (8 unit tests in this file).

**Side-specific rules**:
- Buy: `sl_price < entry_price < tp_price`
- Sell: `tp_price < entry_price < sl_price`

### D5: `flush_trades` hook (not a new cron)
**Why**: Reuses the existing 100ms batching mechanism introduced in P1-2.1. No
new channel, no new timer. The hook is colocated with the Iceberg child-fill
hook at `matching_engine.rs:768`.

**Trigger condition**: `is_fully_filled && advanced_type == "bracket"`.

### D6: 1 query per pending link (no JOIN)
**Why**: `bracket_links` and `orders` are joined by `parent_order_id` only for
debugging. The pending-frontend-poll path (`list_pending_for_user`) only needs
the `bracket_links` row; the original `orders` row is already in the user's
order history.

### D7: `RecordParentFilledInput` struct (8-arg function refactored)
**Why**: `clippy::too_many_arguments` lint (max 7). Refactored to a 7-field
struct + `db` arg = 2 total, well under the limit. Improves call site clarity
(`bracket::RecordParentFilledInput { ... }`).

### D8: OCO status enum (string, not Postgres ENUM)
**Why**: String column `oco_status VARCHAR(16) DEFAULT 'pending'` matches the
existing `order_type` column pattern in `orders`. Avoids the ALTER TYPE
migration friction (cf. P1-2.1 migration issues with enum value additions).

**Valid values**: `pending`, `oco_attached`, `failed`, `cancelled`.

## Affected Files

### New
- `backend/src/models/bracket_params.rs` (Pydantic-style schema, 7 unit tests)
- `backend/src/services/bracket.rs` (4 functions + 6 unit tests)
- `backend/src/handlers/bracket.rs` (GET endpoint)
- `backend/migrations/20260602120000_bracket_links.sql` (table DDL)
- `docs/p1-2-2-bracket/{requirements,architecture,api,operations}/T{1,2,7,8,9}*.md`

### Modified
- `backend/src/handlers/order.rs` — Bracket branch in `create_order` (validation + persist)
- `backend/src/services/matching_engine.rs` — Bracket detection in `flush_trades` hook
- `backend/src/metrics.rs` — `BRACKET_PARENT_FILLED_TOTAL` counter
- `backend/src/models/mod.rs` + `services/mod.rs` + `handlers/mod.rs` — mod registration
- `backend/src/main.rs` — router wire (`/api/v1/bracket-links/pending`)

## Migration Strategy
- **D9 (deferred migration)**: Migration applied manually via `docker exec psql`
  in P1-2.2 E2E, same workaround as P1-2.1 (sqlx migration runner skipped our
  files in this environment). Documented in T7 runbook Issue 3.

## Out of Scope (P1-2.2v2 / P0)
- **Position tracking** — required for full path C automation. Tracked in P0 epic.
- **Auto-attach OCO on fill** — requires positions + multi-leg risk check.
- **Trailing stop** — separate P1-2.3 task.
- **Bracket cancellation cascade** — if parent is cancelled before fill, no
  `bracket_links` row is created (correct behavior, but no UI hint to user).

## Validation & Verification
- 419/419 unit tests pass (`cargo test --lib`)
- 0 clippy warnings (`cargo clippy --lib --all-targets --all-features -- -D warnings`)
- E2E verified via curl + psql: bracket parent created with `advanced_type=bracket`,
  `advanced_params={"oco_status":"pending", "stop_loss_price":49000, "take_profit_price":52000}`

## Followup Tasks
- **P1-2.4**: Frontend integration — poll `GET /bracket-links/pending` and call `POST /trigger-orders/oco`
- **P1-2.5**: Metrics dashboard for `BRACKET_PARENT_FILLED_TOTAL`
- **P0-1**: Position tracking (unlocks path C)
- **P1-2.2v2**: Migration to path B or C once positions table exists
