-- Order management schema tables
-- Mirrors backend/src/db/order.rs SeaORM entities
-- This runs on fresh Docker deployments via docker-entrypoint-initdb.d
-- These tables may also be created by SeaORM run_migrations(), hence IF NOT EXISTS

-- ─── Orders ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    strategy_id UUID,
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type VARCHAR(10) NOT NULL CHECK (order_type IN ('limit', 'market')),
    price DOUBLE PRECISION,
    quantity DOUBLE PRECISION NOT NULL,
    filled_quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    avg_fill_price DOUBLE PRECISION,
    status VARCHAR(20) NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'partial_filled', 'filled', 'cancelled', 'expired', 'rejected')),
    mode VARCHAR(10) NOT NULL DEFAULT 'paper' CHECK (mode IN ('paper', 'live')),
    fee DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    reject_reason TEXT,
    time_in_force VARCHAR(10) NOT NULL DEFAULT 'GTC' CHECK (time_in_force IN ('GTC', 'IOC', 'FOK')),
    expire_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at TIMESTAMPTZ,
    filled_at TIMESTAMPTZ
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_orders_user_id ON orders (user_id);
CREATE INDEX IF NOT EXISTS idx_orders_user_status ON orders (user_id, status);
CREATE INDEX IF NOT EXISTS idx_orders_symbol ON orders (symbol);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders (created_at DESC);

-- ─── Trades ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS trades (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    price DOUBLE PRECISION NOT NULL,
    quantity DOUBLE PRECISION NOT NULL,
    fee DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_maker BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trades_user_id ON trades (user_id);
CREATE INDEX IF NOT EXISTS idx_trades_order_id ON trades (order_id);
CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades (symbol);
CREATE INDEX IF NOT EXISTS idx_trades_created_at ON trades (created_at DESC);

-- ─── Positions ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(5) NOT NULL CHECK (side IN ('long', 'short')),
    quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    available_quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    avg_entry_price DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    unrealized_pnl DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    realized_pnl DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    mode VARCHAR(10) NOT NULL DEFAULT 'paper' CHECK (mode IN ('paper', 'live')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_positions_user_id ON positions (user_id);
CREATE INDEX IF NOT EXISTS idx_positions_user_symbol ON positions (user_id, symbol);

-- ─── Paper Accounts ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS paper_accounts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    balance DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    frozen_balance DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    initial_balance DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_pnl DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_paper_accounts_user_id ON paper_accounts (user_id);

-- ─── Symbol Configs ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS symbol_configs (
    id UUID PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL UNIQUE,
    base_currency VARCHAR(10) NOT NULL,
    quote_currency VARCHAR(10) NOT NULL,
    price_precision SMALLINT NOT NULL DEFAULT 8,
    quantity_precision SMALLINT NOT NULL DEFAULT 8,
    min_quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    max_quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    min_notional DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    fee_rate DOUBLE PRECISION NOT NULL DEFAULT 0.001,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_symbol_configs_symbol ON symbol_configs (symbol);
CREATE INDEX IF NOT EXISTS idx_symbol_configs_enabled ON symbol_configs (enabled);
