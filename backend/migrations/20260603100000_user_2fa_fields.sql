-- 20260603100000_user_2fa_fields.sql
--
-- P3-B: 2FA TOTP fields on `users`.
-- Adds three columns backing the RFC 6238 TOTP flow in `services::totp`:
--   * totp_secret  — base32 shared secret (NULL until setup)
--   * totp_enabled — flips to TRUE only after the user proves possession by
--                    submitting a valid 6-digit TOTP code
--   * backup_codes — JSONB array of bcrypt-hashed one-time recovery codes
--                    (10 codes generated at setup, each 8 hex digits)
--
-- All columns are idempotent (`ADD COLUMN IF NOT EXISTS`) so the migration
-- is safe to re-run on a partially-migrated database. The runtime
-- `db::run_migrations` also issues the same DDL for fresh databases; this
-- file exists for tracking in `migrations/` and for SQL-only operators
-- running the SQL against a managed database.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS totp_secret  VARCHAR(64);

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS backup_codes JSONB;
