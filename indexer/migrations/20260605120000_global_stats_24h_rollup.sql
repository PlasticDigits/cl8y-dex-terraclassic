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

INSERT INTO global_stats_24h (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE global_stats_24h IS
    'Rolling 24h SUM(offer_amount), SUM(volume_usd), COUNT(*) across all pairs; refreshed periodically — not real-time.';
