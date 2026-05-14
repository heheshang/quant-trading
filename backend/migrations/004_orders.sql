-- ============ 交易执行模块 — 数据库迁移 ============
-- 基于 ADR-TRADING-EXECUTION D1-D8

-- 1. 创建订单状态枚举类型
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_side') THEN
    CREATE TYPE order_side AS ENUM ('buy', 'sell');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_type') THEN
    CREATE TYPE order_type AS ENUM ('limit', 'market');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'order_status') THEN
    CREATE TYPE order_status AS ENUM ('pending', 'partial_filled', 'filled', 'cancelled', 'expired', 'rejected');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'trade_mode') THEN
    CREATE TYPE trade_mode AS ENUM ('paper', 'live');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'time_in_force') THEN
    CREATE TYPE time_in_force AS ENUM ('GTC', 'IOC', 'FOK');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'position_side') THEN
    CREATE TYPE position_side AS ENUM ('long', 'short');
  END IF;
END$$;

-- 2. 创建 orders 表
CREATE TABLE IF NOT EXISTS orders (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    strategy_id     UUID,
    symbol          VARCHAR(20)     NOT NULL,
    side            order_side      NOT NULL,
    order_type      order_type      NOT NULL,
    price           DOUBLE PRECISION,
    quantity        DOUBLE PRECISION NOT NULL CHECK (quantity > 0),
    filled_quantity DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (filled_quantity >= 0),
    avg_fill_price  DOUBLE PRECISION,
    status          order_status    NOT NULL DEFAULT 'pending',
    mode            trade_mode      NOT NULL DEFAULT 'paper',
    fee             DOUBLE PRECISION NOT NULL DEFAULT 0,
    reject_reason   TEXT,
    time_in_force   time_in_force   NOT NULL DEFAULT 'GTC',
    expire_at       TIMESTAMPTZ,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    cancelled_at    TIMESTAMPTZ,
    filled_at       TIMESTAMPTZ,
    CONSTRAINT chk_price_for_limit CHECK (
        order_type != 'limit' OR price IS NOT NULL
    ),
    CONSTRAINT chk_price_positive CHECK (
        price IS NULL OR price > 0
    )
);

-- 3. 创建 trades 表
CREATE TABLE IF NOT EXISTS trades (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id        UUID            NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol          VARCHAR(20)     NOT NULL,
    side            order_side      NOT NULL,
    price           DOUBLE PRECISION NOT NULL CHECK (price > 0),
    quantity        DOUBLE PRECISION NOT NULL CHECK (quantity > 0),
    fee             DOUBLE PRECISION NOT NULL DEFAULT 0,
    is_maker        BOOLEAN         NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- 4. 创建 positions 表
CREATE TABLE IF NOT EXISTS positions (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol              VARCHAR(20)     NOT NULL,
    side                position_side   NOT NULL,
    quantity            DOUBLE PRECISION NOT NULL DEFAULT 0,
    available_quantity  DOUBLE PRECISION NOT NULL DEFAULT 0,
    avg_entry_price     DOUBLE PRECISION NOT NULL DEFAULT 0,
    unrealized_pnl      DOUBLE PRECISION NOT NULL DEFAULT 0,
    realized_pnl        DOUBLE PRECISION NOT NULL DEFAULT 0,
    mode                trade_mode      NOT NULL DEFAULT 'paper',
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, symbol)
);

-- 5. 创建 paper_accounts 表
CREATE TABLE IF NOT EXISTS paper_accounts (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID            NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    balance         DOUBLE PRECISION NOT NULL DEFAULT 0,
    frozen_balance  DOUBLE PRECISION NOT NULL DEFAULT 0,
    initial_balance DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_pnl       DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- 6. 创建 symbol_configs 表
CREATE TABLE IF NOT EXISTS symbol_configs (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol              VARCHAR(20)     NOT NULL UNIQUE,
    base_currency       VARCHAR(10)     NOT NULL,
    quote_currency      VARCHAR(10)     NOT NULL,
    price_precision     SMALLINT        NOT NULL DEFAULT 8,
    quantity_precision  SMALLINT        NOT NULL DEFAULT 8,
    min_quantity        DOUBLE PRECISION NOT NULL DEFAULT 0.001,
    max_quantity        DOUBLE PRECISION NOT NULL DEFAULT 1000000,
    min_notional        DOUBLE PRECISION NOT NULL DEFAULT 10,
    fee_rate            DOUBLE PRECISION NOT NULL DEFAULT 0.001,
    enabled             BOOLEAN         NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- 7. 索引
CREATE INDEX IF NOT EXISTS idx_orders_user_id ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_symbol ON orders(symbol);
CREATE INDEX IF NOT EXISTS idx_orders_user_status ON orders(user_id, status);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_trades_order_id ON trades(order_id);
CREATE INDEX IF NOT EXISTS idx_trades_user_id ON trades(user_id);
CREATE INDEX IF NOT EXISTS idx_trades_symbol ON trades(symbol);
CREATE INDEX IF NOT EXISTS idx_trades_created_at ON trades(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_positions_user_id ON positions(user_id);
CREATE INDEX IF NOT EXISTS idx_positions_user_symbol ON positions(user_id, symbol);

CREATE INDEX IF NOT EXISTS idx_paper_accounts_user_id ON paper_accounts(user_id);

-- 8. 初始化默认交易对配置
INSERT INTO symbol_configs (id, symbol, base_currency, quote_currency, price_precision, quantity_precision, min_quantity, max_quantity, min_notional, fee_rate, enabled, created_at)
VALUES
    (gen_random_uuid(), 'BTCUSDT', 'BTC', 'USDT', 2, 6, 0.0001, 1000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'ETHUSDT', 'ETH', 'USDT', 2, 5, 0.001, 10000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'BNBUSDT', 'BNB', 'USDT', 2, 4, 0.01, 50000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'SOLUSDT', 'SOL', 'USDT', 3, 3, 0.1, 100000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'DOGEUSDT', 'DOGE', 'USDT', 5, 0, 1, 10000000, 10, 0.001, true, NOW()),
    (gen_random_uuid(), 'XRPUSDT', 'XRP', 'USDT', 4, 1, 10, 1000000, 10, 0.001, true, NOW())
ON CONFLICT (symbol) DO NOTHING;
