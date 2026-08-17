-- GitLab #543: additive human OHLC beside factory USD candles.
--
-- open/high/low/close stay factory USD of 1 asset_0 (price_usd only — no human fallback).
-- open_human/high_human/low_human/close_human are human quote-per-base (se.price) for
-- per-bar invertUsd on the dApp. Bars with NULL price_usd are not stored (USD series gap).

ALTER TABLE candles
    ADD COLUMN IF NOT EXISTS open_human NUMERIC(78, 18),
    ADD COLUMN IF NOT EXISTS high_human NUMERIC(78, 18),
    ADD COLUMN IF NOT EXISTS low_human NUMERIC(78, 18),
    ADD COLUMN IF NOT EXISTS close_human NUMERIC(78, 18);

TRUNCATE candles;

INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                     open_human, high_human, low_human, close_human,
                     volume_base, volume_quote, trade_count)
SELECT
  se.pair_id,
  iv.interval,
  date_trunc('minute', se.block_timestamp)
    - (EXTRACT(MINUTE FROM se.block_timestamp)::int
       % (iv.epoch_sec / 60)) * interval '1 minute' AS open_time,
  (array_agg(se.price_usd ORDER BY se.block_timestamp ASC, se.id ASC))[1],
  MAX(se.price_usd),
  MIN(se.price_usd),
  (array_agg(se.price_usd ORDER BY se.block_timestamp DESC, se.id DESC))[1],
  (array_agg(se.price ORDER BY se.block_timestamp ASC, se.id ASC))[1],
  MAX(se.price),
  MIN(se.price),
  (array_agg(se.price ORDER BY se.block_timestamp DESC, se.id DESC))[1],
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
WHERE se.price_usd IS NOT NULL AND se.price_usd > 0 AND se.price > 0
GROUP BY se.pair_id, iv.interval, open_time;
