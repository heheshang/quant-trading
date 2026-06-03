-- P2-1: Telegram Bot 通知通道 — users 表加 telegram_chat_id 字段
-- P2-1: Telegram Bot notification channel — add telegram_chat_id column to users.
--
-- 中文说明：
--   该列保存"用户已绑定的 Telegram chat_id"，是 per-user 通知寻址的关键。
--   存 NULL = 未绑定 → send() 时跳过；存非空字符串 = 已绑定 → 用于 sendMessage。
--   字段格式：Telegram chat_id 为 64-bit 整数，VARCHAR 留出宽松余量（最多 32 位 ASCII）。
--
-- English description:
--   Stores the user's bound Telegram chat_id, the key for per-user notification
--   addressing. NULL = unbound → send() skips; non-empty = bound → used in
--   sendMessage. Telegram chat_id is a 64-bit int; VARCHAR(32) gives ASCII slack.
--
-- Idempotent: ADD COLUMN IF NOT EXISTS so this migration is safe to re-run.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS telegram_chat_id VARCHAR(32);

-- 可选索引：用 chat_id 反查 user（多对一时便于排查）
-- Optional index: lookup user by chat_id (one-to-many scenarios during ops).
CREATE INDEX IF NOT EXISTS idx_users_telegram_chat_id
    ON users (telegram_chat_id)
    WHERE telegram_chat_id IS NOT NULL;
