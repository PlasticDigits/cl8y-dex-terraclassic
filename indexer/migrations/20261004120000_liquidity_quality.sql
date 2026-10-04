-- #1380: legacy observations have no reserve/price provenance. Keep their numbers
-- as evidence, explicitly tag unknown, and never use them as comparable baselines.
ALTER TABLE global_stats_24h ALTER COLUMN total_liquidity_usd DROP NOT NULL;
ALTER TABLE global_stats_24h ALTER COLUMN total_liquidity_usd DROP DEFAULT;
ALTER TABLE global_stats_24h
    ADD COLUMN liquidity_quality TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN liquidity_as_of TIMESTAMPTZ,
    ADD COLUMN liquidity_coverage_identity TEXT NOT NULL DEFAULT '',
    ADD COLUMN liquidity_generation BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN liquidity_reasons JSONB NOT NULL DEFAULT '{"legacy_unqualified":1}';
ALTER TABLE global_liquidity_snapshots ALTER COLUMN total_liquidity_usd DROP NOT NULL;
ALTER TABLE global_liquidity_snapshots
    ADD COLUMN quality TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN as_of TIMESTAMPTZ,
    ADD COLUMN coverage_identity TEXT NOT NULL DEFAULT '',
    ADD COLUMN generation BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN reasons JSONB NOT NULL DEFAULT '{"legacy_unqualified":1}',
    ADD COLUMN terminal_slot TIMESTAMPTZ;
CREATE UNIQUE INDEX global_liquidity_terminal_slot ON global_liquidity_snapshots(terminal_slot);
ALTER TABLE pair_liquidity_usd ADD COLUMN generation BIGINT NOT NULL DEFAULT 0;
DO $$
DECLARE table_name TEXT;
BEGIN
    FOREACH table_name IN ARRAY ARRAY['protocol_hourly_liquidity','protocol_daily_liquidity','protocol_monthly_liquidity'] LOOP
        EXECUTE format('ALTER TABLE %I ADD COLUMN quality TEXT NOT NULL DEFAULT ''unknown'', ADD COLUMN as_of TIMESTAMPTZ, ADD COLUMN coverage_identity TEXT NOT NULL DEFAULT '''', ADD COLUMN generation BIGINT NOT NULL DEFAULT 0, ADD COLUMN reasons JSONB NOT NULL DEFAULT ''{"legacy_unqualified":1}''',table_name);
    END LOOP;
END $$;

-- Scoped evidence-preserving repair of the two public anomalies cited in #1380.
-- No reconstructed value is claimed without historical reserves/quotes. Evidence
-- includes original source and derived rows; replay is idempotent.
CREATE TABLE IF NOT EXISTS liquidity_quality_repair_evidence (
    repair_key TEXT NOT NULL,
    source_table TEXT NOT NULL,
    source_key TEXT NOT NULL,
    original_row JSONB NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (repair_key,source_table,source_key)
);
INSERT INTO liquidity_quality_repair_evidence(repair_key,source_table,source_key,original_row)
SELECT 'issue-1380-public-hours','global_liquidity_snapshots',sampled_at::text,to_jsonb(s)
FROM global_liquidity_snapshots s
WHERE (sampled_at >= '2026-10-02 05:00Z' AND sampled_at < '2026-10-02 06:00Z' AND priced_pair_count <= 2)
   OR (sampled_at >= '2026-10-04 02:00Z' AND sampled_at < '2026-10-04 03:00Z' AND priced_pair_count = 0)
ON CONFLICT DO NOTHING;
INSERT INTO liquidity_quality_repair_evidence(repair_key,source_table,source_key,original_row)
SELECT 'issue-1380-public-hours','protocol_hourly_liquidity',utc_hour::text,to_jsonb(h)
FROM protocol_hourly_liquidity h
WHERE ((utc_hour='2026-10-02 05:00Z' AND priced_pair_count <= 2) OR (utc_hour='2026-10-04 02:00Z' AND priced_pair_count=0)) ON CONFLICT DO NOTHING;
UPDATE global_liquidity_snapshots SET total_liquidity_usd=NULL,quality='unknown',
    reasons='{"historical_coverage_unreconciled":1}',as_of=sampled_at
WHERE (sampled_at >= '2026-10-02 05:00Z' AND sampled_at < '2026-10-02 06:00Z' AND priced_pair_count <= 2)
   OR (sampled_at >= '2026-10-04 02:00Z' AND sampled_at < '2026-10-04 03:00Z' AND priced_pair_count = 0);
UPDATE protocol_hourly_liquidity SET liquidity_usd=NULL,quality='unknown',
    reasons='{"historical_coverage_unreconciled":1}'
WHERE ((utc_hour='2026-10-02 05:00Z' AND priced_pair_count <= 2) OR (utc_hour='2026-10-04 02:00Z' AND priced_pair_count=0));
-- A derived daily/monthly row is affected only if its terminal source sample is
-- in a repaired hour. An earlier intraday anomaly does not taint a later close.
DO $$
DECLARE table_name TEXT; bucket_key TEXT; grain TEXT;
BEGIN
    FOR table_name,bucket_key,grain IN VALUES
        ('protocol_daily_liquidity','utc_day','day'),
        ('protocol_monthly_liquidity','utc_month','month')
    LOOP
        EXECUTE format('INSERT INTO liquidity_quality_repair_evidence(repair_key,source_table,source_key,original_row)
            SELECT ''issue-1380-public-hours'',%L,d.%I::text,to_jsonb(d) FROM %I d
            WHERE EXISTS(SELECT 1 FROM global_liquidity_snapshots s
                WHERE date_trunc(%L,s.sampled_at AT TIME ZONE ''UTC'')::date=d.%I
                AND s.reasons ? ''historical_coverage_unreconciled''
                AND NOT EXISTS(SELECT 1 FROM global_liquidity_snapshots later
                    WHERE date_trunc(%L,later.sampled_at AT TIME ZONE ''UTC'')::date=d.%I
                    AND later.sampled_at>s.sampled_at)) ON CONFLICT DO NOTHING',
            table_name,bucket_key,table_name,grain,bucket_key,grain,bucket_key);
        -- Evidence is immutable audit data, not an instruction to keep a bucket
        -- unknown forever. Re-check its current terminal source on every replay.
        EXECUTE format('UPDATE %I d SET liquidity_usd=NULL,quality=''unknown'',reasons='' {"historical_coverage_unreconciled":1}''::jsonb
            WHERE EXISTS(SELECT 1 FROM global_liquidity_snapshots s
                WHERE date_trunc(%L,s.sampled_at AT TIME ZONE ''UTC'')::date=d.%I
                AND s.reasons ? ''historical_coverage_unreconciled''
                AND NOT EXISTS(SELECT 1 FROM global_liquidity_snapshots later
                    WHERE date_trunc(%L,later.sampled_at AT TIME ZONE ''UTC'')::date=d.%I
                    AND later.sampled_at>s.sampled_at))',
            table_name,grain,bucket_key,grain,bucket_key);
    END LOOP;
END $$;
UPDATE global_stats_24h SET liquidity_change_24h_pct=NULL,liquidity_change_30d_pct=NULL,
    total_liquidity_usd_24h_ago=NULL,total_liquidity_usd_30d_ago=NULL
WHERE liquidity_quality='unknown';
