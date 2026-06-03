-- 20260603120000_create_audit_logs.sql
--
-- P3-4: audit_logs — immutable, append-only audit trail for privileged /
-- mutating actions (per PRD §3.16).
--
-- Why a separate file even though `db::run_migrations` also issues the
-- same DDL: this file is the durable, version-controlled record of the
-- schema. SQL-only operators (managed Postgres, etc.) can apply it
-- without booting the Rust binary, and the changelog/timeline shows
-- when audit_logs landed alongside its five query indexes.
--
-- The runtime path is `db::run_migrations` (creates table from
-- `db::audit_log::Entity` via SeaORM `Schema::create_table_from_entity`
-- + the same five `CREATE INDEX IF NOT EXISTS` statements, all idempotent).
-- This file is the human-readable mirror.
--
-- Columns mirror `backend/src/db/audit_log.rs::Model`.

CREATE TABLE IF NOT EXISTS audit_logs (
    id           UUID         PRIMARY KEY,
    user_id      UUID         NULL,           -- nullable: system actor
    action       VARCHAR(120) NOT NULL,       -- e.g. "user.role.changed"
    target_type  VARCHAR(64)  NOT NULL,       -- e.g. "user", "api_key"
    target_id    VARCHAR(128) NOT NULL,       -- stringified PK
    diff         JSONB        NOT NULL,       -- {before, after}
    ip_address   VARCHAR(64)  NOT NULL,
    user_agent   VARCHAR(512) NULL,
    request_id   VARCHAR(64)  NULL,           -- correlates with P0-3 X-Request-Id
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Index 1: "what did user X do recently" — powers the per-user audit page.
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id_created_at
    ON audit_logs (user_id, created_at DESC);

-- Index 2: "all api_key.created events" — for an action-filtered log feed.
CREATE INDEX IF NOT EXISTS idx_audit_logs_action_created_at
    ON audit_logs (action, created_at DESC);

-- Index 3: "history of resource Y" — full lifecycle of a given target.
CREATE INDEX IF NOT EXISTS idx_audit_logs_target_type_target_id
    ON audit_logs (target_type, target_id);

-- Index 4: global time range / pagination.
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at
    ON audit_logs (created_at DESC);

-- Index 5: partial index that pairs audits with their originating HTTP
-- request (P0-3 X-Request-Id). Partial because the column is nullable
-- for system-initiated writes; the predicate keeps the index small.
CREATE INDEX IF NOT EXISTS idx_audit_logs_request_id
    ON audit_logs (request_id) WHERE request_id IS NOT NULL;
