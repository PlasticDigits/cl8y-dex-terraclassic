-- GitLab #550: 7d/30d USD volume, active pairs, unique traders on the 24h rollup row.
-- Cache-miss /overview must stay O(1) + cheap COUNT(*) — no live 30d swap_events SUM.
-- Refresh remains in volume_aggregator (~5 min). Listing census uses created_at indexes.

ALTER TABLE global_stats_24h
    ADD COLUMN IF NOT EXISTS total_volume_7d_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_volume_30d_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_trades_7d BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_trades_30d BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS active_pairs_24h BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS unique_traders_24h BIGINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets (created_at);
CREATE INDEX IF NOT EXISTS idx_pairs_created_at ON pairs (created_at);

COMMENT ON COLUMN global_stats_24h.total_volume_7d_usd IS
    'SUM(volume_usd) over last 7d; USTC-oracle conversion only (X4). Refreshed ~5 min.';
COMMENT ON COLUMN global_stats_24h.total_volume_30d_usd IS
    'SUM(volume_usd) over last 30d; USTC-oracle conversion only (X4). Refreshed ~5 min.';
COMMENT ON COLUMN global_stats_24h.active_pairs_24h IS
    'Distinct pair_id with >=1 swap_events row in last 24h. Dust swaps count; not unique traders.';
COMMENT ON COLUMN global_stats_24h.unique_traders_24h IS
    'Distinct swap_events.sender in last 24h (materialized). Not a live COUNT DISTINCT.';
COMMENT ON INDEX idx_assets_created_at IS
    'Supports tokens_added_30d COUNT(*) on indexer first-seen created_at (GitLab #550). Reindex/rebuild makes all rows look new.';
COMMENT ON INDEX idx_pairs_created_at IS
    'Supports pairs_added_30d COUNT(*) on indexer first-seen created_at (GitLab #550).';
