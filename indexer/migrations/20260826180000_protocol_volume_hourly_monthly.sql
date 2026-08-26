-- GitLab #668: UTC hourly + monthly Protocol volume rollups.
-- GET /protocol/volume/daily?grain= reads these tables only — never SUM swap_events.
-- Daily retain extends to 95d so grain=daily&limit=90 is fillable after prune.

CREATE TABLE IF NOT EXISTS protocol_hourly_volume (
    utc_hour TIMESTAMPTZ PRIMARY KEY,
    volume_usd NUMERIC(38, 18),
    trade_count BIGINT NOT NULL DEFAULT 0,
    unpriced_trade_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_hourly_volume IS
    'UTC-hour Protocol volume (GitLab #668). Bucket [hour, hour+1) UTC. Same P522-Q catalog as protocol_daily_volume. GET reads this table only. Prune >= 10d (240h).';
COMMENT ON COLUMN protocol_hourly_volume.utc_hour IS
    'Hour start timestamptz (UTC). Format in JSON: YYYY-MM-DDTHH.';
COMMENT ON COLUMN protocol_hourly_volume.volume_usd IS
    'SUM priced swap_events.volume_usd in [utc_hour, utc_hour+1h). 0 = idle. NULL = activity but all unpriced. Never invent $0 from unpriced.';

CREATE INDEX IF NOT EXISTS protocol_hourly_volume_hour_idx
    ON protocol_hourly_volume (utc_hour DESC);

CREATE TABLE IF NOT EXISTS protocol_monthly_volume (
    utc_month DATE PRIMARY KEY,
    volume_usd NUMERIC(38, 18),
    trade_count BIGINT NOT NULL DEFAULT 0,
    unpriced_trade_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_monthly_volume IS
    'UTC calendar-month Protocol volume (GitLab #668). utc_month is the first day of the month. Same P522-Q catalog. GET reads this table only. Retain >= 24 months.';
COMMENT ON COLUMN protocol_monthly_volume.utc_month IS
    'First day of the UTC calendar month. JSON: YYYY-MM.';

CREATE INDEX IF NOT EXISTS protocol_monthly_volume_month_idx
    ON protocol_monthly_volume (utc_month DESC);

COMMENT ON TABLE protocol_daily_volume IS
    'UTC-day Protocol volume (GitLab #652 / #668). Same P522-Q catalog as overview total_volume_*_usd — includes gems / wrap / window swaps. Not defillama_daily_stats. GET reads this table only. Prune >= 95 days (daily grain max 90).';
