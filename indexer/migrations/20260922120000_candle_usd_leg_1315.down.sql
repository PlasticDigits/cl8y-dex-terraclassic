-- Restore the pre-#1315 GET /candles contract before an old indexer binary serves it.
-- Stamped gap-fill keys, every usd_leg = asset_1 row, and every NULL open row
-- are not readable as non-null USD of asset_0.

DELETE FROM candles c
USING candle_gap_fill_1315 s
WHERE c.pair_id = s.pair_id
  AND c.interval = s.interval
  AND c.open_time = s.open_time;

DELETE FROM candles WHERE usd_leg = 'asset_1';

DELETE FROM candles WHERE open IS NULL;

ALTER TABLE candles DROP CONSTRAINT IF EXISTS candles_usd_leg_check;
ALTER TABLE candles DROP COLUMN IF EXISTS usd_leg;

DROP TABLE IF EXISTS candle_gap_fill_1315;
DROP TABLE IF EXISTS candle_gap_fill_1315_done;
