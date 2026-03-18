-- USTC oracle price tracking and USD volume denomination

CREATE TABLE IF NOT EXISTS ustc_prices (
    id BIGSERIAL PRIMARY KEY,
    price_usd NUMERIC(38, 18) NOT NULL,
    source VARCHAR(32) NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_ustc_prices_fetched ON ustc_prices(fetched_at DESC);
CREATE INDEX idx_ustc_prices_source ON ustc_prices(source, fetched_at DESC);

ALTER TABLE swap_events ADD COLUMN IF NOT EXISTS volume_usd NUMERIC(38, 18);

ALTER TABLE token_volume_stats ADD COLUMN IF NOT EXISTS volume_usd NUMERIC(38, 18) NOT NULL DEFAULT 0;
