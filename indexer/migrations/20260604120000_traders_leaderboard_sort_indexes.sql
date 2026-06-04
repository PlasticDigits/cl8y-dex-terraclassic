-- GitLab #280: /traders/leaderboard sorts on volume_24h / volume_7d / volume_30d /
-- total_trades, none of which were indexed -> a seq scan + top-N sort per request.
-- Add btree indexes mirroring idx_traders_volume(total_volume_usd DESC) so the
-- ORDER BY ... DESC LIMIT $1 uses an index scan instead of a full sort.
CREATE INDEX IF NOT EXISTS idx_traders_volume_24h ON traders(volume_24h DESC);
CREATE INDEX IF NOT EXISTS idx_traders_volume_7d ON traders(volume_7d DESC);
CREATE INDEX IF NOT EXISTS idx_traders_volume_30d ON traders(volume_30d DESC);
CREATE INDEX IF NOT EXISTS idx_traders_total_trades ON traders(total_trades DESC);
