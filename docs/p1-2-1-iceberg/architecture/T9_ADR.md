# T9 — P1-2.1 Iceberg Architecture Decision Records

## Status
- **Date**: 2026-06-02
- **Author**: ssk
- **Status**: Accepted
- **Supersedes**: P1-2 MVP stub (`advanced_type`/`advanced_params` columns)

## Context
Iceberg orders are a real exchange feature (Binance, OKX, Bybit, Gate.io all support it) used by market makers to hide true position size. The P1-2 MVP added the schema (`advanced_type` TEXT, `advanced_params` JSONB) but no business logic. This ADR documents the business-logic decisions.

## Decisions

### D1. Child Order Type
**Decision**: Iceberg children are stored as `order_type=limit` with `advanced_type='iceberg_child'` as a discriminator.
**Rationale**: The matching engine treats all limit orders uniformly. Reusing the existing limit code path means zero changes to the matching algorithm.
**Consequence**: A query for `order_type=limit` will include iceberg children. Clients must filter on the **API** level (currently the GET orders endpoint does not return `advanced_type`; see D13).

### D2. Visible Quantity Storage
**Decision**: `visible_quantity` is a request field, **not** a model field. It is read from the request, used to compute `total_slices = ceil(quantity / visible_quantity)`, and stored in `advanced_params` JSONB as `{ visible_quantity, total_quantity, total_slices, filled_children, child_ids }`.
**Rationale**: The request schema is the contract; the JSONB is the runtime state. Decoupling allows `visible_quantity` to differ from the JSONB invariant (e.g. user changes mind before the second slice is appended — not currently supported, but the schema permits it).

### D3. Balance Freeze on the Parent
**Decision**: The full parent `quantity` is frozen on the parent order at creation. **Children do not freeze** additional balance.
**Rationale**: Freezing once at the parent is simpler and matches Binance semantics. The unfreeze on cancel returns the *remaining* quantity.

### D4. Final Slice Rounding
**Decision**: When `total_quantity % visible_quantity != 0`, the **last slice** absorbs the remainder.
**Example**: qty=10, visible=3 → slices = [3, 3, 3, 1] (4 slices, last is 1).
**Rationale**: Matches Binance/OKX behavior. Avoids "lost" 0.00001 of asset due to floating-point rounding.

### D5. Replenishment Latency
**Decision**: A new child slice is inserted into the book **at most 100ms** after the previous child fills.
**Rationale**: The replenishment is piggy-backed on the existing `flush_trades` 100ms batch flush. No new channel, no new task.
**Trade-off**: For HFT latency-sensitive users, 100ms is too long. Acceptable for P1; can revisit in P2 with a dedicated replenishment task.

### D6. Cancellation Cascade
**Decision**: Cancelling the parent cancels **all** `active` and `pending` children. Already-filled children are **not** reversed.
**Rationale**: Standard exchange semantics. Trade history is preserved.

### D7. Append-Only Replenishment
**Decision**: `append_next_child` is **idempotent** and **lazy**. It does not pre-create all 10 slices; it only creates the next one when the previous one fills.
**Rationale**: Memory-efficient for large icebergs. The order book never holds more than one active slice per iceberg parent.

### D8. Public API Hides `advanced_type` (D13)
**Decision**: The `GET /api/v1/orders` response does **not** currently expose `advanced_type` or `advanced_params`. Clients see only the parent (order_type=iceberg) and the children as plain limit orders.
**Rationale**: P1-2.1 ships the backend logic; surfacing iceberg metadata to clients is a frontend change deferred to P1-2.4.
**TODO**: Add `advanced_type` and `parent_id` to the response DTO in a follow-up.

### D9. No New Channel
**Decision**: The replenishment hook is added at the end of `flush_trades` in the matching engine, **not** a new tokio task.
**Rationale**: Avoids spawning a new task per iceberg. The hook is a single `if` branch and runs in O(1) when the child is not iceberg.

### D10. Migration Drift
**Decision**: The P1-2 migration `20260602000000_advanced_order_types.sql` was **not auto-applied** on first deploy because the backend was started before the migration file landed. The migration is `IF NOT EXISTS` so re-deploys are safe.
**Mitigation**: Documented in T7 Issue 1; one-off `ALTER TABLE` applied to the live DB during P1-2.1 verification.

### D11. `MatchingEngine::new` Returns `Arc<Self>`
**Decision**: `MatchingEngine::new()` was changed to return `Arc<Self>` instead of `Self`. The spawn task holds one Arc clone; the caller gets the other.
**Rationale**: Required to allow `flush_trades` to call `iceberg::append_next_child(&Arc<...>, ...)` from inside a `tokio::spawn` block (the spawned task must own its `engine` reference).
**Trade-off**: API breakage for any direct `MatchingEngine::new()` callers. Only one (main.rs) was affected and updated.

### D12. Error Handling in Replenishment Hook
**Decision**: If `append_next_child` fails (DB error, etc.), the matching engine logs the error and **continues** (does not abort the batch). The parent order remains in `partially_filled` state, and the next flush cycle will retry via the next child fill.
**Rationale**: Failures should not poison the entire trade batch.

### D13. Public Response Omits `advanced_type` (open issue)
**Status**: Deferred.
**Action**: Frontend or P1-2.4 work should add `advanced_type` to the `OrderResponse` DTO and update the OpenAPI schema.

## Verification (P1-2.1 done)
- ✅ `cargo build` clean
- ✅ `cargo test --lib`: 406/406 pass (+16 new iceberg tests)
- ✅ `cargo clippy -- -D warnings`: 0 warnings
- ✅ E2E curl: `POST /api/v1/orders` with `order_type=iceberg` → 201, parent + 1st child in DB
- ✅ DB state: parent (`order_type=iceberg, advanced_type=iceberg, qty=10`) + 1st child (`order_type=limit, advanced_type=iceberg_child, qty=3`) created atomically

## Risks
- **R1**: 100ms replenishment latency. Acceptable for paper trading, may be slow for live HFT. See D5.
- **R2**: `advanced_type` schema drift. The Rust enum (`iceberg::advanced_type::ICEBERG_CHILD`) must match the DB string exactly. A typo in either would silently break replenishment. **Mitigation**: covered by unit tests in `iceberg_params` that round-trip the string.
- **R3**: Concurrent cancellation race (parent cancel + child fill in same flush). The current code uses `tokio::select!` semantics in the spawned task; the `update_order_status` call in `flush_trades` may race with `cancel_iceberg_children`. **Mitigation**: covered by `where status = 'pending'` clause in update; if the child was already cancelled, the update is a no-op.
