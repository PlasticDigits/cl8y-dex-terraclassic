-- #1315: store which factory leg candle USD describes.
-- NULL usd_leg + non-null open remains USD of asset_0 (rows written before this column).
-- NULL open is a neither-catalog human OHLC row.

ALTER TABLE candles
    ADD COLUMN IF NOT EXISTS usd_leg TEXT;

ALTER TABLE candles DROP CONSTRAINT IF EXISTS candles_usd_leg_check;
ALTER TABLE candles
    ADD CONSTRAINT candles_usd_leg_check
    CHECK (usd_leg IS NULL OR usd_leg IN ('asset_0', 'asset_1'));

CREATE TABLE IF NOT EXISTS candle_gap_fill_1315 (
    pair_id INTEGER NOT NULL,
    interval TEXT NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (pair_id, interval, open_time)
);

CREATE TABLE IF NOT EXISTS candle_gap_fill_1315_done (
    id INTEGER PRIMARY KEY,
    finished_at TIMESTAMPTZ NOT NULL
);
