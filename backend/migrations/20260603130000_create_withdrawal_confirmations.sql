-- 20260603130000_create_withdrawal_confirmations.sql
--
-- P3-6: withdrawal_confirmations — secure two-step withdrawal flow.
--
-- Why a separate file even though `db::run_migrations` also issues the
-- same DDL: this file is the durable, version-controlled record of the
-- schema. SQL-only operators (managed Postgres, etc.) can apply it
-- without booting the Rust binary, and the changelog/timeline shows
-- when withdrawal_confirmations landed alongside its three indexes.
--
-- The runtime path is `db::run_migrations` (creates table from
-- `db::withdrawal_confirmation::Entity` via SeaORM `Schema::create_table_from_entity`
-- + the same three `CREATE INDEX IF NOT EXISTS` statements, all idempotent).
-- This file is the human-readable mirror.
--
-- Columns mirror `backend/src/db/withdrawal_confirmation.rs::Model`.

CREATE TABLE IF NOT EXISTS withdrawal_confirmations (
    id                   UUID            PRIMARY KEY,
    user_id              UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount               DECIMAL(20, 8)  NOT NULL,
    asset                VARCHAR(16)     NOT NULL,
    dest_address         VARCHAR(256)    NOT NULL,
    confirm_code_hash    VARCHAR(64)     NOT NULL,        -- hex(SHA-256(code))
    expires_at           TIMESTAMPTZ     NOT NULL,
    confirmed_at         TIMESTAMPTZ,
    status               VARCHAR(16)     NOT NULL DEFAULT 'Pending',  -- Pending/Confirmed/Expired/Cancelled
    mock_withdrawal_id   UUID,
    created_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- Index 1: "this user's withdrawal history" — powers GET /api/v1/withdrawals
CREATE INDEX IF NOT EXISTS idx_withdrawal_confirmations_user_id_created_at
    ON withdrawal_confirmations (user_id, created_at DESC);

-- Index 2: "expire_overdue worker hot path" — finds Pending rows past their
-- expires_at in one pass. Note: status is VARCHAR(16), not an enum, so the
-- index can still be a btree on the literal.
CREATE INDEX IF NOT EXISTS idx_withdrawal_confirmations_status_expires_at
    ON withdrawal_confirmations (status, expires_at);
