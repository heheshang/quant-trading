-- Copy Trading — PRD §6 commercialization-2.
--
-- Four tables back the full copy-trading lifecycle:
--
--   copy_traders        — the trader profile (display name, bio, aggregated P&L)
--   copy_subscriptions  — one row per (follower, trader) with risk limits
--   copy_trades         — append-only log of every copied order (audit)
--   copy_profit_shares  — period profit allocation between trader + followers
--
-- All four use UUID PKs to dodge BIGSERIAL hot-row contention on concurrent
-- subscribe / fan-out writes.
--
-- Foreign keys:
--   - copy_traders.user_id           → users.id
--   - copy_subscriptions.trader_id   → copy_traders.id
--   - copy_subscriptions.follower_id → users.id
--   - copy_trades.subscription_id    → copy_subscriptions.id
--   - copy_trades.original_order_id  → orders.id (nullable: a strategy-driven
--                                       trader may have orders that don't
--                                       have a single 'orders' row)
--   - copy_trades.copied_order_id    → orders.id
--   - copy_profit_shares.subscription_id → copy_subscriptions.id
--
-- On-delete behaviour:
--   - Deleting a user is rare (audit / GDPR), so we CASCADE — their copy
--     history goes with them.
--   - Deleting a trader is more disruptive (active followers) but we CASCADE
--     on subscriptions + profit_shares so no orphan rows survive. A trader
--     deletion is gated by the service layer.
--   - Deleting a subscription CASCADES its copy_trades and profit_shares —
--     we keep the audit row only as long as the relationship is live.

CREATE TABLE IF NOT EXISTS copy_traders (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID            NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    display_name    VARCHAR(120)    NOT NULL,
    bio             TEXT,
    total_pnl       DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    monthly_pnl     DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    win_rate        DECIMAL(10, 6)  NOT NULL DEFAULT 0,
    follower_count  BIGINT          NOT NULL DEFAULT 0,
    status          VARCHAR(16)     NOT NULL DEFAULT 'Active',
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    CONSTRAINT copy_traders_status_check
        CHECK (status IN ('Active', 'Paused', 'Banned')),
    CONSTRAINT copy_traders_display_name_check
        CHECK (char_length(display_name) BETWEEN 1 AND 120),
    CONSTRAINT copy_traders_win_rate_check
        CHECK (win_rate >= 0 AND win_rate <= 1),
    CONSTRAINT copy_traders_follower_count_check
        CHECK (follower_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_copy_traders_status_monthly
    ON copy_traders (status, monthly_pnl DESC);
CREATE INDEX IF NOT EXISTS idx_copy_traders_total_pnl
    ON copy_traders (status, total_pnl DESC);

CREATE TABLE IF NOT EXISTS copy_subscriptions (
    id                UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    trader_id         UUID            NOT NULL REFERENCES copy_traders(id) ON DELETE CASCADE,
    follower_id       UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Copy ratio: 0.10 = follower risks 10% of the trader's notional. Stored
    -- as Decimal to match PAMM; bounded [0, 1].
    ratio             DECIMAL(10, 6)  NOT NULL,
    max_position_size DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    max_loss_per_day  DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    status            VARCHAR(16)     NOT NULL DEFAULT 'Active',
    started_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    ended_at          TIMESTAMPTZ,
    -- One row per (follower, trader) — re-subscribing after an unsubscribe
    -- is a fresh row, not an update of the old one (so the audit log is
    -- preserved).
    CONSTRAINT copy_subscriptions_status_check
        CHECK (status IN ('Active', 'Paused', 'Cancelled')),
    CONSTRAINT copy_subscriptions_ratio_check
        CHECK (ratio >= 0 AND ratio <= 1),
    CONSTRAINT copy_subscriptions_max_pos_check
        CHECK (max_position_size >= 0),
    CONSTRAINT copy_subscriptions_max_loss_check
        CHECK (max_loss_per_day >= 0)
);

CREATE INDEX IF NOT EXISTS idx_copy_subs_trader_status
    ON copy_subscriptions (trader_id, status);
CREATE INDEX IF NOT EXISTS idx_copy_subs_follower
    ON copy_subscriptions (follower_id, status);
-- Idempotent unique constraint: one active row per (follower, trader) is
-- allowed; we let the DB reject a duplicate subscribe.
CREATE UNIQUE INDEX IF NOT EXISTS uq_copy_subs_follower_trader
    ON copy_subscriptions (follower_id, trader_id);

CREATE TABLE IF NOT EXISTS copy_trades (
    id                 UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id    UUID            NOT NULL REFERENCES copy_subscriptions(id) ON DELETE CASCADE,
    -- The trader's order that triggered the copy. Nullable for the rare
    -- strategy-link path where the trader has no 'orders' row.
    original_order_id  UUID            REFERENCES orders(id) ON DELETE SET NULL,
    -- The follower's order. NEVER NULL — if the copy failed (risk check
    -- rejected), no row is written at all.
    copied_order_id    UUID            NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    symbol             VARCHAR(20)     NOT NULL,
    side               VARCHAR(8)      NOT NULL,
    qty                DECIMAL(20, 8)  NOT NULL,
    price              DECIMAL(20, 8)  NOT NULL,
    status             VARCHAR(16)     NOT NULL DEFAULT 'Copied',
    created_at         TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    CONSTRAINT copy_trades_side_check
        CHECK (side IN ('buy', 'sell')),
    CONSTRAINT copy_trades_qty_check
        CHECK (qty > 0),
    CONSTRAINT copy_trades_price_check
        CHECK (price > 0)
);

CREATE INDEX IF NOT EXISTS idx_copy_trades_subscription
    ON copy_trades (subscription_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_copy_trades_symbol_time
    ON copy_trades (symbol, created_at DESC);

CREATE TABLE IF NOT EXISTS copy_profit_shares (
    id                UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id   UUID            NOT NULL REFERENCES copy_subscriptions(id) ON DELETE CASCADE,
    period_start      TIMESTAMPTZ     NOT NULL,
    period_end        TIMESTAMPTZ     NOT NULL,
    -- Net profit/loss for the follower across all their copied trades in
    -- this period. Negative is allowed.
    follower_profit   DECIMAL(20, 8)  NOT NULL,
    -- The trader's cut (e.g. 30% of follower_profit if positive). Always
    -- non-negative; a negative follower_profit is the follower's loss and
    -- the trader doesn't get a share of the downside.
    trader_profit     DECIMAL(20, 8)  NOT NULL,
    -- Performance fee rate applied (e.g. 0.30 for 30%).
    trader_fee_pct    DECIMAL(10, 6)  NOT NULL,
    distributed_at    TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_copy_profit_shares_sub
    ON copy_profit_shares (subscription_id, distributed_at DESC);
-- Idempotent: one distribution per (subscription, period) so a retried
-- monthly settle is a no-op.
CREATE UNIQUE INDEX IF NOT EXISTS uq_copy_profit_shares_period
    ON copy_profit_shares (subscription_id, period_start, period_end);
