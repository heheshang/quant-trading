-- PAMM (Percent Allocation Management Module) — PRD §6 commercialization-1.
--
-- Five tables:
--   pamm_funds                 — fund definition (manager + fee + status)
--   pamm_investments           — investor's live stake (user, fund, shares)
--   pamm_profit_distributions  — append-only per-period allocation ledger
--   pamm_subscriptions         — pending/active/refunded subscription requests
--   pamm_redemptions           — pending/completed/cancelled redemption requests
--
-- All five use UUID PKs (random, not bigserial) to avoid hot-row
-- contention on concurrent subscribes / distributions.
--
-- Foreign keys:
--   - fund.manager_id           → users.id
--   - investment.fund_id        → pamm_funds.id
--   - investment.user_id        → users.id
--   - distribution.fund_id      → pamm_funds.id
--   - distribution.user_id      → users.id
--   - subscription.fund_id      → pamm_funds.id
--   - subscription.user_id      → users.id
--   - redemption.fund_id        → pamm_funds.id
--   - redemption.user_id        → users.id
--   - fund.strategy_id          → strategies.id (nullable)
--
-- On-delete behaviour:
--   - Deleting a user is rare (audit / GDPR), so we CASCADE — the
--     user's PAMM history goes with them.
--   - Deleting a fund should also clear all dependent rows; CASCADE.
--   - Deleting a strategy should NOT cascade (set NULL on the fund
--     row) so the manager doesn't lose the rest of the fund config.

CREATE TABLE IF NOT EXISTS pamm_funds (
    id                   UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    manager_id           UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                 VARCHAR(120)    NOT NULL,
    description          TEXT,
    base_currency        VARCHAR(16)     NOT NULL,
    management_fee_pct   DECIMAL(10, 6)  NOT NULL DEFAULT 0.02,
    performance_fee_pct  DECIMAL(10, 6)  NOT NULL DEFAULT 0.20,
    high_water_mark      BOOLEAN         NOT NULL DEFAULT TRUE,
    nav                  DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    share_value          DECIMAL(20, 8)  NOT NULL DEFAULT 1.0,
    hwm                  DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    total_shares         DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    strategy_id          UUID            REFERENCES strategies(id) ON DELETE SET NULL,
    status               VARCHAR(16)     NOT NULL DEFAULT 'Active',
    created_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    CONSTRAINT pamm_funds_status_check
        CHECK (status IN ('Active', 'Paused', 'Liquidated')),
    CONSTRAINT pamm_funds_fee_check
        CHECK (management_fee_pct >= 0 AND management_fee_pct <= 1
               AND performance_fee_pct >= 0 AND performance_fee_pct <= 1)
);

CREATE INDEX IF NOT EXISTS idx_pamm_funds_manager_id
    ON pamm_funds (manager_id);
CREATE INDEX IF NOT EXISTS idx_pamm_funds_status
    ON pamm_funds (status);
CREATE INDEX IF NOT EXISTS idx_pamm_funds_strategy_id
    ON pamm_funds (strategy_id) WHERE strategy_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS pamm_investments (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    fund_id             UUID            NOT NULL REFERENCES pamm_funds(id) ON DELETE CASCADE,
    user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    share_pct           DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    shares              DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    initial_investment  DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    current_value       DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    -- One row per (user, fund). Without this, a redemption + re-subscribe
    -- race could create two competing rows that the manager's NAV
    -- rollup would then double-count.
    CONSTRAINT pamm_investments_user_fund_unique UNIQUE (user_id, fund_id)
);

CREATE INDEX IF NOT EXISTS idx_pamm_investments_fund_id
    ON pamm_investments (fund_id);
CREATE INDEX IF NOT EXISTS idx_pamm_investments_user_id
    ON pamm_investments (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS pamm_profit_distributions (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    fund_id             UUID            NOT NULL REFERENCES pamm_funds(id) ON DELETE CASCADE,
    user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start        TIMESTAMPTZ     NOT NULL,
    period_end          TIMESTAMPTZ     NOT NULL,
    profit_amount       DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    hwm                 DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    perf_fee_charged    DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    mgmt_fee_charged    DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    distributed_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pamm_profit_distributions_fund_id
    ON pamm_profit_distributions (fund_id, distributed_at DESC);
CREATE INDEX IF NOT EXISTS idx_pamm_profit_distributions_user_id
    ON pamm_profit_distributions (user_id, distributed_at DESC);
-- Idempotent unique constraint (one distribution per user per period per fund)
-- so a retried distribute call is a no-op rather than a duplicate row.
CREATE UNIQUE INDEX IF NOT EXISTS uq_pamm_distribution_period
    ON pamm_profit_distributions (fund_id, user_id, period_start, period_end);

CREATE TABLE IF NOT EXISTS pamm_subscriptions (
    id          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    fund_id     UUID            NOT NULL REFERENCES pamm_funds(id) ON DELETE CASCADE,
    user_id     UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount      DECIMAL(20, 8)  NOT NULL,
    status      VARCHAR(16)     NOT NULL DEFAULT 'Pending',
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    CONSTRAINT pamm_subscriptions_status_check
        CHECK (status IN ('Pending', 'Active', 'Refunded')),
    CONSTRAINT pamm_subscriptions_amount_check
        CHECK (amount > 0)
);

CREATE INDEX IF NOT EXISTS idx_pamm_subscriptions_fund_id
    ON pamm_subscriptions (fund_id, status);
CREATE INDEX IF NOT EXISTS idx_pamm_subscriptions_user_id
    ON pamm_subscriptions (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS pamm_redemptions (
    id                UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    fund_id           UUID            NOT NULL REFERENCES pamm_funds(id) ON DELETE CASCADE,
    user_id           UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_requested  DECIMAL(20, 8)  NOT NULL,
    amount_paid       DECIMAL(20, 8)  NOT NULL DEFAULT 0,
    status            VARCHAR(16)     NOT NULL DEFAULT 'Pending',
    requested_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    paid_at           TIMESTAMPTZ,
    CONSTRAINT pamm_redemptions_status_check
        CHECK (status IN ('Pending', 'Completed', 'Cancelled')),
    CONSTRAINT pamm_redemptions_amount_check
        CHECK (amount_requested > 0)
);

CREATE INDEX IF NOT EXISTS idx_pamm_redemptions_fund_id
    ON pamm_redemptions (fund_id, status);
CREATE INDEX IF NOT EXISTS idx_pamm_redemptions_user_id
    ON pamm_redemptions (user_id, requested_at DESC);
