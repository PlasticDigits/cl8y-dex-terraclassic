-- GitLab #548: backfill swap_events.volume_usd via P522-Q catalog (shared with #544).
-- Prefer pair quote (asset_1); else offer; else ask. Never sum both legs.
-- Idempotent. Keep in sync with volume::backfill_swap_volume_usd.
-- USD is as-of latest oracle_prices averages at migration time (advisory, not mark-to-market).
--
-- Coolify prod never applied the first cut of this file (numeric overflow). Edited in place.
--
-- NUMERIC(38, 18) only stores |x| < 10^20. Raw SUM(offer_amount) overflows on 18-decimal
-- CW20 volume (~100 human USTR). total_volume / pair quote volume are raw integers.

ALTER TABLE global_stats_24h
    ALTER COLUMN total_volume TYPE NUMERIC(38, 0);

ALTER TABLE pair_volume_24h
    ALTER COLUMN volume_quote TYPE NUMERIC(38, 0);

WITH latest_oracle AS (
    SELECT DISTINCT ON (ticker) ticker, price_usd
    FROM oracle_prices
    WHERE source = 'average'
    ORDER BY ticker, fetched_at DESC
),
ustc AS (
    SELECT price_usd FROM latest_oracle WHERE ticker = 'ustc'
),
lunc AS (
    SELECT price_usd FROM latest_oracle WHERE ticker = 'lunc'
),
catalog AS (
    SELECT
        a.id,
        a.decimals,
        CASE
            WHEN a.denom = 'uusd' THEN (SELECT price_usd FROM ustc)
            WHEN a.denom = 'uluna' THEN (SELECT price_usd FROM lunc)
            WHEN NOT a.is_cw20 AND a.denom IS NOT NULL AND a.denom NOT IN ('uusd', 'uluna') THEN NULL
            WHEN UPPER(a.symbol) = 'UST1' AND a.contract_address IS NOT NULL THEN 1::numeric
            WHEN UPPER(a.symbol) IN ('USTC', 'CUSTC')
                 AND (a.denom = 'uusd' OR a.contract_address IS NOT NULL)
                THEN (SELECT price_usd FROM ustc)
            WHEN UPPER(a.symbol) IN ('LUNC', 'CLUNC')
                 AND (a.denom = 'uluna' OR a.contract_address IS NOT NULL)
                THEN (SELECT price_usd FROM lunc)
            WHEN UPPER(a.symbol) = 'USTR' AND a.contract_address IS NOT NULL
                THEN 2.5::numeric * (SELECT price_usd FROM ustc)
            ELSE NULL
        END AS usd_per_human
    FROM assets a
),
priced AS (
    SELECT
        se.id,
        CASE
            WHEN q.usd_per_human IS NOT NULL AND q.usd_per_human > 0
                 AND q.decimals BETWEEN 0 AND 38 THEN
                CASE
                    WHEN se.offer_asset_id = p.asset_1_id THEN
                        se.offer_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
                    ELSE
                        se.return_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
                END
            WHEN o.usd_per_human IS NOT NULL AND o.usd_per_human > 0
                 AND o.decimals BETWEEN 0 AND 38 THEN
                se.offer_amount / POWER(10::numeric, o.decimals) * o.usd_per_human
            WHEN k.usd_per_human IS NOT NULL AND k.usd_per_human > 0
                 AND k.decimals BETWEEN 0 AND 38 THEN
                se.return_amount / POWER(10::numeric, k.decimals) * k.usd_per_human
            ELSE NULL
        END AS raw_usd
    FROM swap_events se
    JOIN pairs p ON p.id = se.pair_id
    JOIN catalog q ON q.id = p.asset_1_id
    JOIN catalog o ON o.id = se.offer_asset_id
    JOIN catalog k ON k.id = se.ask_asset_id
)
UPDATE swap_events se
SET volume_usd = CASE
    WHEN pr.raw_usd IS NULL OR pr.raw_usd <= 0 OR pr.raw_usd >= POWER(10::numeric, 20) THEN NULL
    ELSE pr.raw_usd
END
FROM priced pr
WHERE se.id = pr.id;

-- Refresh 24h overview rollup so Charts USD includes catalog-priced history.
INSERT INTO global_stats_24h (id, total_volume, total_volume_usd, total_trades, updated_at)
SELECT 1,
       LEAST(COALESCE(SUM(offer_amount), 0), POWER(10::numeric, 38) - 1),
       LEAST(COALESCE(SUM(volume_usd), 0), POWER(10::numeric, 20) - POWER(10::numeric, -18)),
       COUNT(*),
       NOW()
FROM swap_events
WHERE block_timestamp >= NOW() - INTERVAL '24 hours'
ON CONFLICT (id) DO UPDATE SET
  total_volume = EXCLUDED.total_volume,
  total_volume_usd = EXCLUDED.total_volume_usd,
  total_trades = EXCLUDED.total_trades,
  updated_at = EXCLUDED.updated_at;
