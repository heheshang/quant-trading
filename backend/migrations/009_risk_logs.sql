-- Migration: risk_logs table (ADR-013 D2)
-- Created: 2026-05-23

CREATE TABLE IF NOT EXISTS risk_logs (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rule_type   VARCHAR(20) NOT NULL,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    position_value   DECIMAL(20, 8),
    account_equity  DECIMAL(20, 8) NOT NULL,
    threshold       DECIMAL(20, 8) NOT NULL,
    actual_value    DECIMAL(20, 8) NOT NULL,
    action_taken VARCHAR(20) NOT NULL,
    order_id    UUID REFERENCES orders(id) ON DELETE SET NULL,
    notification_sent BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_risk_logs_user_id     ON risk_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_risk_logs_triggered_at ON risk_logs(triggered_at DESC);
CREATE INDEX IF NOT EXISTS idx_risk_logs_rule_type    ON risk_logs(rule_type);
