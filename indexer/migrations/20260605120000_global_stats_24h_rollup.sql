-- Materialized global 24h overview stats for GET /api/v1/overview (GitLab #333 / #281 follow-up).
-- Refreshed by indexer background task every ~5 min (see volume_aggregator.rs).
-- Single-row table: O(1) read on /overview cache miss instead of scanning swap_events.

CREATE TABLE IF NOT EXISTS global_stats_24h (
    id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    total_volume NUMERIC(38, 18) NOT NULL DEFAULT 0,
    total_volume_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    total_trades BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Backfill from existing swap_events on upgrade; fresh DBs get zeros until first refresh.
INSERT INTO global_stats_24h (id, total_volume, total_volume_usd, total_trades, updated_at)
SELECT 1,
       COALESCE(SUM(offer_amount), 0),
       COALESCE(SUM(volume_usd), 0),
       COUNT(*),
       NOW()
FROM swap_events
WHERE block_timestamp >= NOW() - INTERVAL '24 hours'
ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE global_stats_24h IS
    'Rolling 24h SUM(offer_amount), SUM(volume_usd), COUNT(*) across all pairs; refreshed periodically — not real-time.';
