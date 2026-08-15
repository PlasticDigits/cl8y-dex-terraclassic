-- GitLab #522: human-scale swap_events.price + persist USD of 1 human base (asset_0).
--
-- P522-1: human_quote_per_base = raw_quote_per_base * 10^(decimals_base − decimals_quote).
-- #466 already stored raw quote-per-base; this multiplies by the decimal factor.
-- P522-2: price_usd = human_quote_per_base * quote_token_usd (catalog + latest #515 oracle).
-- Candles labeled Price (USD) are rebuilt from COALESCE(price_usd, price).

ALTER TABLE swap_events
    ADD COLUMN IF NOT EXISTS price_usd NUMERIC(78, 18);

-- Scale historical raw quote-per-base → human (same-decimal pairs are * 10^0).
UPDATE swap_events se
SET price = se.price * POWER(10::numeric, a0.decimals - a1.decimals)
FROM pairs p
JOIN assets a0 ON a0.id = p.asset_0_id
JOIN assets a1 ON a1.id = p.asset_1_id
WHERE se.pair_id = p.id;

-- Latest advisory USTC / LUNC averages (may be NULL on empty oracle tables).
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
)
UPDATE swap_events se
SET price_usd = se.price * CASE
    WHEN UPPER(a1.symbol) IN ('UST1') THEN 1::numeric
    WHEN UPPER(a1.symbol) IN ('USTC', 'CUSTC') OR a1.denom = 'uusd' THEN (SELECT price_usd FROM ustc)
    WHEN UPPER(a1.symbol) IN ('LUNC', 'CLUNC') OR a1.denom = 'uluna' THEN (SELECT price_usd FROM lunc)
    WHEN UPPER(a1.symbol) = 'USTR' THEN 2.5::numeric * (SELECT price_usd FROM ustc)
    ELSE NULL
END
FROM pairs p
JOIN assets a1 ON a1.id = p.asset_1_id
WHERE se.pair_id = p.id;

-- Rebuild candles so history is not permanently T-scaled / quote-ratio on a USD axis.
TRUNCATE candles;

INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                     volume_base, volume_quote, trade_count)
SELECT
  se.pair_id,
  iv.interval,
  date_trunc('minute', se.block_timestamp)
    - (EXTRACT(MINUTE FROM se.block_timestamp)::int
       % (iv.epoch_sec / 60)) * interval '1 minute' AS open_time,
  (array_agg(COALESCE(se.price_usd, se.price) ORDER BY se.block_timestamp ASC, se.id ASC))[1],
  MAX(COALESCE(se.price_usd, se.price)),
  MIN(COALESCE(se.price_usd, se.price)),
  (array_agg(COALESCE(se.price_usd, se.price) ORDER BY se.block_timestamp DESC, se.id DESC))[1],
  SUM(CASE WHEN se.offer_asset_id = p.asset_0_id THEN se.offer_amount ELSE se.return_amount END),
  SUM(CASE WHEN se.offer_asset_id = p.asset_0_id THEN se.return_amount ELSE se.offer_amount END),
  COUNT(*)::int
FROM swap_events se
INNER JOIN pairs p ON p.id = se.pair_id
CROSS JOIN (
    VALUES
      ('1m', 60),
      ('5m', 300),
      ('15m', 900),
      ('1h', 3600),
      ('4h', 14400),
      ('1d', 86400),
      ('1w', 604800)
) AS iv(interval, epoch_sec)
WHERE COALESCE(se.price_usd, se.price) > 0
GROUP BY se.pair_id, iv.interval, open_time;
