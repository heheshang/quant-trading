-- P1-5: Risk rules — 5 new dimensions
-- Extends risk_rules table with: trading_hours, position_concentration,
-- loss_cooldown, event_blacklist.
-- Adds risk_correlations table for correlation_matrix dimension.
--
-- Idempotent: every ALTER uses ADD COLUMN IF NOT EXISTS so this is safe
-- to run against a fresh DB (where SeaORM has already created the base
-- risk_rules table) and against an existing DB from a prior run.

-- 1. trading_hours dimension
--    Time-of-day window when the account is allowed to open new positions.
--    NULL = no restriction (24/7).
ALTER TABLE risk_rules
    ADD COLUMN IF NOT EXISTS trading_hours_start TIME,
    ADD COLUMN IF NOT EXISTS trading_hours_end   TIME,
    -- MM-DD pairs (e.g. ["01-01", "12-25"]). NULL/[] = no holiday restriction.
    ADD COLUMN IF NOT EXISTS trading_holidays    JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 2. position_concentration dimension
--    Pct of total account equity a single symbol (or top-3 basket) may
--    occupy. NULL = no concentration cap.
ALTER TABLE risk_rules
    ADD COLUMN IF NOT EXISTS max_single_symbol_pct DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS max_top3_pct         DOUBLE PRECISION;

-- 3. loss_cooldown dimension
--    After N consecutive losing fills, block new opens for M minutes.
ALTER TABLE risk_rules
    ADD COLUMN IF NOT EXISTS consecutive_loss_limit INTEGER,
    ADD COLUMN IF NOT EXISTS cooldown_minutes       INTEGER;

-- 4. event_blacklist dimension
--    Macro/event windows during which new opens are blocked.
--    Shape: [{"event_name": "CPI", "hours_before": 2, "hours_after": 1}, ...]
--    Defaults (CPI / FOMC / NFP) live in the application layer; this
--    column stores the user override (empty list = use defaults).
ALTER TABLE risk_rules
    ADD COLUMN IF NOT EXISTS event_blacklist JSONB NOT NULL DEFAULT '[]'::jsonb;

-- 5. correlation_matrix dimension
--    Stored in a separate table — one row per unordered symbol pair.
CREATE TABLE IF NOT EXISTS risk_correlations (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol_a        VARCHAR(20)  NOT NULL,
    symbol_b        VARCHAR(20)  NOT NULL,
    -- Pearson r over the trailing 30 daily returns, in [-1, 1].
    correlation     DOUBLE PRECISION NOT NULL CHECK (correlation >= -1.0 AND correlation <= 1.0),
    computed_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT uniq_correlation_pair UNIQUE (user_id, symbol_a, symbol_b),
    CONSTRAINT chk_symbols_ordered   CHECK (symbol_a < symbol_b)
);

CREATE INDEX IF NOT EXISTS idx_risk_correlations_user
    ON risk_correlations (user_id);
CREATE INDEX IF NOT EXISTS idx_risk_correlations_pair
    ON risk_correlations (user_id, symbol_a, symbol_b);
