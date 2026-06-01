-- Migration: Create trigger_orders table for P1-F3 (条件触发单)
-- 止损单/止盈单/OCO/TWAP

CREATE TABLE IF NOT EXISTS trigger_orders (
    id              UUID        NOT NULL DEFAULT gen_random_uuid(),
    user_id         UUID        NOT NULL,
    position_id     UUID,
    symbol          VARCHAR(20) NOT NULL,
    trigger_type    VARCHAR(20) NOT NULL, -- stop_loss / take_profit / oco / twap
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    trigger_direction VARCHAR(10) NOT NULL, -- up / down

    -- 触发价格
    trigger_price   DECIMAL(20,8) NOT NULL,

    -- OCO 上下限
    trigger_price_upper DECIMAL(20,8),
    trigger_price_lower DECIMAL(20,8),

    -- 基础价格 (挂单价格)
    base_price      DECIMAL(20,8),

    -- 订单方向 (TWAP需要)
    side            VARCHAR(10) NOT NULL, -- buy / sell

    -- 数量
    quantity         DECIMAL(20,8) NOT NULL,
    filled_quantity  DECIMAL(20,8) NOT NULL DEFAULT 0,

    -- 成交均价
    avg_fill_price  DECIMAL(20,8),

    -- TWAP 参数
    twap_slice_quantity  DECIMAL(20,8) NOT NULL DEFAULT 0,
    twap_interval_secs   INT         NOT NULL DEFAULT 60,
    twap_start_time      TIMESTAMPTZ,
    twap_end_time        TIMESTAMPTZ,
    twap_executed_slices INT         NOT NULL DEFAULT 0,
    twap_max_slices      INT         NOT NULL DEFAULT 100,

    -- OCO 关联
    oco_pair_id     UUID,

    -- 触发后创建的订单
    triggered_order_id UUID,

    -- 触发原因
    trigger_reason  VARCHAR(100),

    -- 时间戳
    triggered_at    TIMESTAMPTZ,
    expire_at       TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at    TIMESTAMPTZ,

    PRIMARY KEY (id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_trigger_orders_user_id ON trigger_orders (user_id);
CREATE INDEX IF NOT EXISTS idx_trigger_orders_symbol ON trigger_orders (symbol);
CREATE INDEX IF NOT EXISTS idx_trigger_orders_status ON trigger_orders (status);
CREATE INDEX IF NOT EXISTS idx_trigger_orders_type ON trigger_orders (trigger_type);
CREATE INDEX IF NOT EXISTS idx_trigger_orders_position_id ON trigger_orders (position_id) WHERE position_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trigger_orders_oco_pair ON trigger_orders (oco_pair_id) WHERE oco_pair_id IS NOT NULL;

-- 约束: trigger_type 枚举值
ALTER TABLE trigger_orders ADD CONSTRAINT chk_trigger_type
    CHECK (trigger_type IN ('stop_loss', 'take_profit', 'oco', 'twap'));

-- 约束: status 枚举值
ALTER TABLE trigger_orders ADD CONSTRAINT chk_trigger_status
    CHECK (status IN ('pending', 'triggered', 'cancelled', 'expired', 'failed'));

-- 约束: trigger_direction 枚举值
ALTER TABLE trigger_orders ADD CONSTRAINT chk_trigger_direction
    CHECK (trigger_direction IN ('up', 'down'));

-- 约束: side 枚举值
ALTER TABLE trigger_orders ADD CONSTRAINT chk_side
    CHECK (side IN ('buy', 'sell'));
