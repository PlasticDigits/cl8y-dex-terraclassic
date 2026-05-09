-- GitLab #142 — distinguish parked-expired limit orders from active; terminal refunded after claim.

ALTER TABLE limit_order_placements
    ADD COLUMN lifecycle_status VARCHAR(32) NOT NULL DEFAULT 'active';

ALTER TABLE limit_order_placements
    ADD CONSTRAINT chk_limit_order_placements_lifecycle_status
        CHECK (lifecycle_status IN ('active', 'parked_expired', 'refunded'));

ALTER TABLE limit_order_placements
    ADD COLUMN remaining_escrow NUMERIC(38, 18);

ALTER TABLE limit_order_placements
    ADD COLUMN parked_block_height BIGINT,
    ADD COLUMN parked_block_timestamp TIMESTAMPTZ,
    ADD COLUMN parked_tx_hash VARCHAR(64);

ALTER TABLE limit_order_placements
    ADD COLUMN refunded_block_height BIGINT,
    ADD COLUMN refunded_block_timestamp TIMESTAMPTZ,
    ADD COLUMN refunded_tx_hash VARCHAR(64);

CREATE INDEX IF NOT EXISTS idx_lo_placements_pair_lifecycle
    ON limit_order_placements (pair_id, lifecycle_status);
