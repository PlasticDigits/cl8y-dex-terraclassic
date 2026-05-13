-- Speed up trader-scoped cancel history (`WHERE owner = $addr ORDER BY block_timestamp DESC`).
CREATE INDEX IF NOT EXISTS idx_lo_cancellations_owner_time
ON limit_order_cancellations (owner, block_timestamp DESC)
WHERE owner IS NOT NULL;
