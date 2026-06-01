-- ============ P1-F2: 实盘止盈止损 — 数据库迁移 ============
-- PRD: P1-F2 实盘止盈止损
-- ADR: P1-F2 实现

-- 1. 创建止盈止损触发类型枚举
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'alert_type') THEN
    CREATE TYPE alert_type AS ENUM ('take_profit', 'stop_loss', 'trailing_stop');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'alert_status') THEN
    CREATE TYPE alert_status AS ENUM ('active', 'triggered', 'cancelled', 'paused');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'trigger_mode') THEN
    CREATE TYPE trigger_mode AS ENUM ('market', 'limit');
  END IF;
END$$;

-- 2. 创建 position_alerts 表
CREATE TABLE IF NOT EXISTS position_alerts (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    position_id         UUID            NOT NULL,  -- 关联持仓ID (无外键以避免循环)
    symbol              VARCHAR(20)     NOT NULL,
    alert_type          alert_type      NOT NULL,  -- 'take_profit' | 'stop_loss' | 'trailing_stop'
    status              alert_status    NOT NULL DEFAULT 'active',
    trigger_price       DOUBLE PRECISION NOT NULL, -- 触发价格
    limit_price         DOUBLE PRECISION,          -- 限价触发时的目标价格（可选）
    trigger_mode        trigger_mode    NOT NULL DEFAULT 'market', -- 市价/限价触发
    trailing_distance   DOUBLE PRECISION,          -- 追踪止损距离（百分比）
    trailing_activated  BOOLEAN         NOT NULL DEFAULT false,      -- 追踪是否已激活
    activated_price     DOUBLE PRECISION,          -- 追踪止损激活时的最高价/最低价
    triggered_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    cancelled_at        TIMESTAMPTZ,
    triggered_order_id  UUID,                      -- 触发后创建的平仓订单ID
    note                TEXT
);

-- 3. 索引
CREATE INDEX IF NOT EXISTS idx_position_alerts_user_id ON position_alerts(user_id);
CREATE INDEX IF NOT EXISTS idx_position_alerts_position_id ON position_alerts(position_id);
CREATE INDEX IF NOT EXISTS idx_position_alerts_symbol ON position_alerts(symbol);
CREATE INDEX IF NOT EXISTS idx_position_alerts_status ON position_alerts(status);
CREATE INDEX IF NOT EXISTS idx_position_alerts_user_status ON position_alerts(user_id, status);

-- 4. position_alerts 表增加 position_id 的普通索引（用于查用户持仓对应的alerts）
CREATE INDEX IF NOT EXISTS idx_position_alerts_lookup ON position_alerts(user_id, symbol, status);
