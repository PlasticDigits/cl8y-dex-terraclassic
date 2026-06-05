-- GitLab #279 (Phase 1a): materialized current-state limit book, distinct from the append-only
-- limit_order_placements/_cancellations/_fills logs (those lack the current `remaining` the walk
-- needs). One row per live resting order; the snapshot loop replaces a pair's rows wholesale. The
-- 0-LCD solver reads this ordered by price + FIFO instead of querying the chain.
CREATE TABLE IF NOT EXISTS resting_limit_orders (
    pair_id INTEGER NOT NULL REFERENCES pairs(id) ON DELETE CASCADE,
    order_id BIGINT NOT NULL,
    side VARCHAR(3) NOT NULL CHECK (side IN ('bid', 'ask')),
    price NUMERIC(38, 18) NOT NULL,
    remaining NUMERIC(38, 0) NOT NULL,
    owner VARCHAR(64),
    expires_at BIGINT,
    block_height BIGINT,
    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (pair_id, order_id)
);
-- Walk order: best price first (bids DESC, asks ASC), then FIFO by order_id.
CREATE INDEX IF NOT EXISTS idx_resting_orders_book ON resting_limit_orders (pair_id, side, price, order_id);
