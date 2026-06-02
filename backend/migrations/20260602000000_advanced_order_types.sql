-- ============ P1-2: Advanced Order Types ============
-- Add 3 new variants to order_type enum: iceberg, bracket, trailing_stop
-- Add advanced_params JSONB column for parameter storage

-- 1. Extend order_type enum with 3 new values.
-- ALTER TYPE ... ADD VALUE cannot run inside a transaction block
-- when the type is used elsewhere, so use IF NOT EXISTS where supported
-- (PG 12+). For older PG, we conditionally use a DO block.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_enum
        WHERE enumlabel = 'iceberg'
          AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'order_type')
    ) THEN
        ALTER TYPE order_type ADD VALUE 'iceberg';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_enum
        WHERE enumlabel = 'bracket'
          AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'order_type')
    ) THEN
        ALTER TYPE order_type ADD VALUE 'bracket';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_enum
        WHERE enumlabel = 'trailing_stop'
          AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'order_type')
    ) THEN
        ALTER TYPE order_type ADD VALUE 'trailing_stop';
    END IF;
END$$;

-- 2. Add advanced_params JSONB column to orders for advanced-type metadata.
-- Schema (validated at application layer):
--   iceberg:        { visible_quantity: f64 }
--   bracket:        { entry_price: f64, stop_loss: f64, take_profit: f64, parent_status: "open"|"sl_triggered"|"tp_triggered" }
--   trailing_stop:  { entry_price: f64, trail_amount: f64, current_stop: f64, highest_price: f64, lowest_price: f64 }
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS advanced_params JSONB;

-- 3. Add advanced_type TEXT discriminator (limit/market/iceberg/bracket/trailing_stop).
-- Redundant with order_type but stored separately for fast filtering.
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS advanced_type TEXT;

-- 4. Index for fast advanced-type queries (used by trailing stop service).
CREATE INDEX IF NOT EXISTS idx_orders_advanced_type ON orders(advanced_type) WHERE advanced_type IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_orders_advanced_status ON orders(advanced_type, status) WHERE advanced_type = 'trailing_stop';
