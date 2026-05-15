-- ============ 策略管理模块 — 添加 symbol/timeframe/strategy_type ============
-- 基于 ADR-STRATEGY-MGMT D1/D2/D6
-- 要求: user_strategies 表必须有 symbol + timeframe + strategy_type

-- 1. 添加新列（可空过渡期）
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS symbol VARCHAR(20) NOT NULL DEFAULT 'BTCUSDT';
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS timeframe VARCHAR(10) NOT NULL DEFAULT '1h';
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS strategy_type VARCHAR(50) NOT NULL DEFAULT 'trend_following';

-- 2. 重建约束（NOT NULL 在有数据后生效）
ALTER TABLE strategies ALTER COLUMN symbol DROP DEFAULT;
ALTER TABLE strategies ALTER COLUMN symbol SET NOT NULL;
ALTER TABLE strategies ALTER COLUMN timeframe DROP DEFAULT;
ALTER TABLE strategies ALTER COLUMN timeframe SET NOT NULL;
ALTER TABLE strategies ALTER COLUMN strategy_type DROP DEFAULT;
ALTER TABLE strategies ALTER COLUMN strategy_type SET NOT NULL;

-- 3. 索引
CREATE INDEX IF NOT EXISTS idx_strategies_user_id ON strategies(user_id);
CREATE INDEX IF NOT EXISTS idx_strategies_status ON strategies(status);
CREATE INDEX IF NOT EXISTS idx_strategies_symbol ON strategies(symbol);

-- 4. 回滚
-- ALTER TABLE strategies DROP COLUMN IF EXISTS symbol;
-- ALTER TABLE strategies DROP COLUMN IF EXISTS timeframe;
-- ALTER TABLE strategies DROP COLUMN IF EXISTS strategy_type;
