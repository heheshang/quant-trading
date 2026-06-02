-- P1-2.2: Bracket order link table
--
-- When a bracket parent order is fully filled, a `bracket_links` row is
-- inserted recording the SL/TP prices + the position info needed by the
-- frontend to create the OCO. OCO creation itself is deferred (path A).

CREATE TABLE IF NOT EXISTS bracket_links (
    id              UUID PRIMARY KEY,
    parent_order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id),
    symbol          VARCHAR(32) NOT NULL,
    sl_price        DOUBLE PRECISION NOT NULL,
    tp_price        DOUBLE PRECISION NOT NULL,
    side            VARCHAR(8) NOT NULL,
    filled_quantity DOUBLE PRECISION NOT NULL,
    oco_status      VARCHAR(16) NOT NULL DEFAULT 'pending',
    sl_trigger_id   UUID,
    tp_trigger_id   UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookup by (user_id, oco_status) for the pending-frontend-poll path.
CREATE INDEX IF NOT EXISTS idx_bracket_links_user_status
    ON bracket_links (user_id, oco_status, created_at);

-- One active link per parent (cascade handles the rest).
CREATE UNIQUE INDEX IF NOT EXISTS uniq_bracket_links_parent
    ON bracket_links (parent_order_id);
