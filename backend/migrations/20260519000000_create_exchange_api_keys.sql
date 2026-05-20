-- ADR-012: API Key 加密存储表
-- 创建 exchange_api_keys 表用于存储用户的交易所 API Key

CREATE TABLE IF NOT EXISTS exchange_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    exchange VARCHAR(32) NOT NULL DEFAULT 'binance',
    api_key TEXT NOT NULL,
    secret_encrypted TEXT NOT NULL,
    nonce TEXT NOT NULL,
    permissions VARCHAR(128) NOT NULL DEFAULT 'read,trade',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, exchange)
);

CREATE INDEX IF NOT EXISTS idx_exchange_api_keys_user ON exchange_api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_exchange_api_keys_exchange ON exchange_api_keys(exchange);
