-- 套利模块三表：arbitrage_pairs / arbitrage_positions / arbitrage_signals
-- 2026-05-21

-- 套利对配置表
CREATE TABLE IF NOT EXISTS arbitrage_pairs (
    id SERIAL PRIMARY KEY,
    pair_type VARCHAR(32) NOT NULL,          -- calendar_spread, cross_pair, spot_futures
    symbol_a VARCHAR(64) NOT NULL,
    symbol_b VARCHAR(64) NOT NULL,
    exchange VARCHAR(32) NOT NULL DEFAULT 'binance',
    status VARCHAR(16) NOT NULL DEFAULT 'active',  -- active, inactive
    spread_entry_threshold DECIMAL(12, 6) NOT NULL DEFAULT 0.02,
    spread_exit_threshold DECIMAL(12, 6) NOT NULL DEFAULT 0.005,
    max_position_size DECIMAL(14, 4) NOT NULL DEFAULT 10000,
    calculation_mode VARCHAR(16) NOT NULL DEFAULT 'percentage',  -- ratio, percentage, zscore
    correlation_threshold DECIMAL(8, 4),      -- 跨品种用，如 0.9000
    z_score_entry DECIMAL(8, 4),              -- 如 2.0000
    z_score_exit DECIMAL(8, 4),               -- 如 0.5000
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 套利持仓表
CREATE TABLE IF NOT EXISTS arbitrage_positions (
    id SERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES arbitrage_pairs(id) ON DELETE CASCADE,
    direction VARCHAR(16) NOT NULL,           -- long_spread (A多B空), short_spread (A空B多)
    size_a DECIMAL(14, 4) NOT NULL DEFAULT 0,
    size_b DECIMAL(14, 4) NOT NULL DEFAULT 0,
    entry_spread DECIMAL(16, 8) NOT NULL,
    current_spread DECIMAL(16, 8),
    unrealized_pnl DECIMAL(14, 4) DEFAULT 0,
    status VARCHAR(16) NOT NULL DEFAULT 'open',  -- open, closed, liquidated
    opened_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMP WITH TIME ZONE
);

-- 套利信号记录表
CREATE TABLE IF NOT EXISTS arbitrage_signals (
    id SERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES arbitrage_pairs(id) ON DELETE CASCADE,
    signal_type VARCHAR(16) NOT NULL,         -- entry_long, entry_short, exit, stop_loss
    spread DECIMAL(16, 8),
    z_score DECIMAL(10, 4),
    confidence DECIMAL(6, 4),                  -- 0.0000 - 1.0000
    executed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_arbitrage_pairs_status ON arbitrage_pairs(status);
CREATE INDEX IF NOT EXISTS idx_arbitrage_positions_pair_id ON arbitrage_positions(pair_id);
CREATE INDEX IF NOT EXISTS idx_arbitrage_positions_status ON arbitrage_positions(status);
CREATE INDEX IF NOT EXISTS idx_arbitrage_signals_pair_id ON arbitrage_signals(pair_id);
CREATE INDEX IF NOT EXISTS idx_arbitrage_signals_created ON arbitrage_signals(created_at);
