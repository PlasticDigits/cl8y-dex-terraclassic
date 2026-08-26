-- GitLab #652: volume flow Δ% on overview + UTC-day Protocol volume series.
-- GET /overview and GET /protocol/volume/daily read rollups only — never SUM 60d swap_events.

ALTER TABLE global_stats_24h
    ADD COLUMN IF NOT EXISTS volume_change_24h_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS volume_change_7d_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS volume_change_30d_pct NUMERIC(38, 18);

COMMENT ON COLUMN global_stats_24h.volume_change_24h_pct IS
    'Flow Δ% vs prior equal window [now-48h, now-24h). Same flow_change_pct as fees. NULL if no prior trades / then<=0 / unpriced current / overflow. Never Infinity. Not TVL snapshot Δ%.';
COMMENT ON COLUMN global_stats_24h.volume_change_7d_pct IS
    'Flow Δ% vs prior equal window [now-14d, now-7d). NULL if no prior trades / then<=0 / unpriced current / overflow.';
COMMENT ON COLUMN global_stats_24h.volume_change_30d_pct IS
    'Flow Δ% vs prior equal window [now-60d, now-30d). NULL if no prior trades / then<=0 / unpriced current / overflow.';

CREATE TABLE IF NOT EXISTS protocol_daily_volume (
    utc_day DATE PRIMARY KEY,
    volume_usd NUMERIC(38, 18),
    trade_count BIGINT NOT NULL DEFAULT 0,
    unpriced_trade_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_daily_volume IS
    'UTC-day Protocol volume (GitLab #652). Same P522-Q catalog as overview total_volume_*_usd — includes gems / wrap / window swaps. Not defillama_daily_stats. GET reads this table only. Prune >= 35 days.';
COMMENT ON COLUMN protocol_daily_volume.volume_usd IS
    'SUM priced swap_events.volume_usd in [utc_day, utc_day+1) UTC. 0 = idle (trade_count=0). NULL = activity but all unpriced. Never invent $0 from unpriced.';
COMMENT ON COLUMN protocol_daily_volume.trade_count IS
    'COUNT(*) of swap_events that UTC day (dust counts; same set as overview).';

CREATE INDEX IF NOT EXISTS protocol_daily_volume_day_idx
    ON protocol_daily_volume (utc_day DESC);
