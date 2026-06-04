-- GitLab #281: /overview global 24h stats run SUM(offer_amount)/SUM(volume_usd)/COUNT(*)
-- over swap_events WHERE block_timestamp >= $1 with no block_timestamp-leading index
-- (only idx_swaps_pair_time (pair_id, block_timestamp)), so the global aggregate full-scans
-- swap_events. A BRIN index on the monotonic, append-only block_timestamp lets the recent
-- 24h window skip old block ranges cheaply, with a tiny index footprint that suits a
-- high-insert-rate swap table.
CREATE INDEX IF NOT EXISTS idx_swaps_block_timestamp_brin
    ON swap_events USING BRIN (block_timestamp);
