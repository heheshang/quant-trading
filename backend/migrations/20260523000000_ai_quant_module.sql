-- Migration: P3-F3 AI量化模块 - 数据库表
-- Created: 2026-05-22

-- 模型版本管理表
CREATE TABLE IF NOT EXISTS model_versions (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    version VARCHAR(16) NOT NULL UNIQUE,
    description TEXT,
    status VARCHAR(16) NOT NULL DEFAULT 'staged',  -- staged/active/deactivated
    metrics_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_model_versions_status ON model_versions(status);
CREATE INDEX idx_model_versions_version ON model_versions(version);

-- AI信号记录表
CREATE TABLE IF NOT EXISTS ai_signals (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    symbol VARCHAR(32) NOT NULL,
    timeframe VARCHAR(8) NOT NULL DEFAULT '1m',
    direction VARCHAR(8),   -- up/down/neutral
    probability FLOAT,      -- 0.0 ~ 1.0
    sentiment_score FLOAT,  -- -1.0 ~ 1.0
    final_score FLOAT,      -- 融合后得分
    model_version VARCHAR(16),
    feature_snapshot JSONB,  -- 特征快照
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_signals_symbol_time ON ai_signals(symbol, timestamp DESC);
CREATE INDEX idx_ai_signals_model_version ON ai_signals(model_version);
CREATE INDEX idx_ai_signals_direction ON ai_signals(direction);

-- A/B实验日志表
CREATE TABLE IF NOT EXISTS ab_experiment_logs (
    id BIGSERIAL PRIMARY KEY,
    experiment_id VARCHAR(64) NOT NULL,
    model_version VARCHAR(16) NOT NULL,
    order_id BIGINT,
    signal_strength FLOAT,
    ai_probability FLOAT,
    sentiment_score FLOAT,
    final_decision VARCHAR(8),
    decision_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_ab_experiment_experiment_id ON ab_experiment_logs(experiment_id);
CREATE INDEX idx_ab_experiment_model_version ON ab_experiment_logs(model_version);
CREATE INDEX idx_ab_experiment_decision_at ON ab_experiment_logs(decision_at DESC);
