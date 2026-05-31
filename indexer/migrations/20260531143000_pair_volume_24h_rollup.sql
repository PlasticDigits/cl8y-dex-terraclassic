-- Materialized 24h pair quote volume for paginated pair listing (GitLab #243 / M4).
-- Refreshed by indexer background task every ~5 min (see volume_aggregator.rs).

CREATE TABLE IF NOT EXISTS pair_volume_24h (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    volume_quote NUMERIC(38, 18) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pair_volume_24h_volume ON pair_volume_24h (volume_quote DESC);

COMMENT ON TABLE pair_volume_24h IS
    'Rolling 24h SUM(return_amount) per pair; refreshed periodically — not real-time.';
