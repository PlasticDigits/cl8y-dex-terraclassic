-- Rollback for 20260509160000_limit_order_placement_lifecycle.sql (manual: not run by sqlx::migrate!).

DROP INDEX IF EXISTS idx_lo_placements_pair_lifecycle;

ALTER TABLE limit_order_placements
    DROP CONSTRAINT IF EXISTS chk_limit_order_placements_lifecycle_status;

ALTER TABLE limit_order_placements
    DROP COLUMN IF EXISTS refunded_tx_hash,
    DROP COLUMN IF EXISTS refunded_block_timestamp,
    DROP COLUMN IF EXISTS refunded_block_height,
    DROP COLUMN IF EXISTS parked_tx_hash,
    DROP COLUMN IF EXISTS parked_block_timestamp,
    DROP COLUMN IF EXISTS parked_block_height,
    DROP COLUMN IF EXISTS remaining_escrow,
    DROP COLUMN IF EXISTS lifecycle_status;
