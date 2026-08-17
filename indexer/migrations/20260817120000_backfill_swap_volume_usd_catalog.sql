-- GitLab #548: backfill swap_events.volume_usd via P522-Q catalog (shared with #544).
-- Prefer pair quote (asset_1); else offer; else ask. Never sum both legs.
-- Idempotent. Keep in sync with volume::backfill_swap_volume_usd.
-- USD is as-of latest oracle_prices averages at migration time (advisory, not mark-to-market).

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
)
UPDATE swap_events se
SET volume_usd = CASE
    WHEN q.usd_per_human IS NOT NULL AND q.usd_per_human > 0 THEN
        CASE
            WHEN se.offer_asset_id = p.asset_1_id THEN
                se.offer_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
            ELSE
                se.return_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
        END
    WHEN o.usd_per_human IS NOT NULL AND o.usd_per_human > 0 THEN
        se.offer_amount / POWER(10::numeric, o.decimals) * o.usd_per_human
    WHEN k.usd_per_human IS NOT NULL AND k.usd_per_human > 0 THEN
        se.return_amount / POWER(10::numeric, k.decimals) * k.usd_per_human
    ELSE NULL
END
FROM pairs p, catalog q, catalog o, catalog k
WHERE se.pair_id = p.id
  AND q.id = p.asset_1_id
  AND o.id = se.offer_asset_id
  AND k.id = se.ask_asset_id;

-- Refresh 24h overview rollup so Charts USD includes catalog-priced history.
INSERT INTO global_stats_24h (id, total_volume, total_volume_usd, total_trades, updated_at)
SELECT 1,
       COALESCE(SUM(offer_amount), 0),
       COALESCE(SUM(volume_usd), 0),
       COUNT(*),
       NOW()
FROM swap_events
WHERE block_timestamp >= NOW() - INTERVAL '24 hours'
ON CONFLICT (id) DO UPDATE SET
  total_volume = EXCLUDED.total_volume,
  total_volume_usd = EXCLUDED.total_volume_usd,
  total_trades = EXCLUDED.total_trades,
  updated_at = EXCLUDED.updated_at;
