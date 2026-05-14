-- Portfolio Equity History table
-- Mirrors backend/src/db/portfolio.rs SeaORM entity
-- This runs on fresh Docker deployments via docker-entrypoint-initdb.d
--
-- NOTE: The FK to users(id) is NOT created here because the users table
-- is created by the application's SeaORM migration (run_migrations).
-- The FK is added in run_migrations() after both tables exist.

CREATE TABLE IF NOT EXISTS portfolio_equity_history (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    equity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying equity history by user + time range (used by equity_curve endpoint)
CREATE INDEX IF NOT EXISTS idx_portfolio_equity_user_timestamp
    ON portfolio_equity_history (user_id, timestamp DESC);
