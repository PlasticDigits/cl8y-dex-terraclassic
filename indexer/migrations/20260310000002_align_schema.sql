-- Align schema with application queries

-- Rename ohlcv_candles -> candles and adjust columns
ALTER TABLE IF EXISTS ohlcv_candles RENAME TO candles;
ALTER TABLE candles ADD COLUMN IF NOT EXISTS id BIGSERIAL;
ALTER TABLE candles RENAME COLUMN open_price TO open;
ALTER TABLE candles RENAME COLUMN high_price TO high;
ALTER TABLE candles RENAME COLUMN low_price TO low;
ALTER TABLE candles RENAME COLUMN close_price TO close;
ALTER TABLE candles ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE candles ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Rename traders columns to match query code
ALTER TABLE traders RENAME COLUMN total_volume_usd TO total_volume;
ALTER TABLE traders RENAME COLUMN volume_24h_usd TO volume_24h;
ALTER TABLE traders RENAME COLUMN volume_7d_usd TO volume_7d;
ALTER TABLE traders RENAME COLUMN volume_30d_usd TO volume_30d;
ALTER TABLE traders RENAME COLUMN fee_discount_registered TO registered;
ALTER TABLE traders ALTER COLUMN total_trades TYPE BIGINT;

-- Rename token_volume_stats.period -> window and add id
ALTER TABLE token_volume_stats RENAME COLUMN period TO window;
ALTER TABLE token_volume_stats ADD COLUMN IF NOT EXISTS id BIGSERIAL;
ALTER TABLE token_volume_stats ALTER COLUMN trade_count TYPE BIGINT;

-- Add created_at to liquidity_events
ALTER TABLE liquidity_events ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add created_at to traders
ALTER TABLE traders ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
