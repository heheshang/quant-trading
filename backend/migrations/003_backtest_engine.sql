-- ============ 策略回测引擎 — 数据库迁移 ============
-- 基于 ADR-007, data-model.md (2.3节)
-- 注意: backtest_results 表已在 data-model.md 中定义
-- 此迁移补充缺少的字段

-- 1. 如果 backtest_status 类型枚举不存在，创建它
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'backtest_status') THEN
    CREATE TYPE backtest_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
  END IF;
END$$;

-- 2. 创建 backtest_results 表（如果不存在）
CREATE TABLE IF NOT EXISTS backtest_results (
    id          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    strategy_id UUID            NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    user_id     UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    config      JSONB           NOT NULL,
    status      backtest_status NOT NULL DEFAULT 'pending',
    progress    SMALLINT        NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    start_time  TIMESTAMPTZ,
    end_time    TIMESTAMPTZ,
    duration_ms BIGINT,
    metrics     JSONB,
    trades      JSONB,
    equity_curve JSONB,
    error       TEXT,
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- 3. 索引
CREATE INDEX IF NOT EXISTS idx_backtest_strategy_id ON backtest_results(strategy_id);
CREATE INDEX IF NOT EXISTS idx_backtest_user_id ON backtest_results(user_id);
CREATE INDEX IF NOT EXISTS idx_backtest_created_at ON backtest_results(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_backtest_status ON backtest_results(status);

-- 4. kline_data 表索引（如果尚未创建）
CREATE INDEX IF NOT EXISTS idx_kline_lookup ON kline_data(symbol, interval, open_time DESC);
