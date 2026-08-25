-- GitLab #631 — UTC-day UST1 / USTR dimensions for DeFiLlama.
-- UST1 is the unstablecoin (Llama Stablecoins). USTR is reserve-token info.

CREATE TABLE defillama_daily_assets (
    utc_day DATE NOT NULL REFERENCES defillama_daily_stats(utc_day) ON DELETE CASCADE,
    ticker TEXT NOT NULL,
    volume_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    trade_count BIGINT NOT NULL DEFAULT 0,
    unpriced_trade_count BIGINT NOT NULL DEFAULT 0,
    fees_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    fee_event_count BIGINT NOT NULL DEFAULT 0,
    fee_unpriced_count BIGINT NOT NULL DEFAULT 0,
    price_usd NUMERIC(38, 18),
    circulating_raw NUMERIC(78, 0),
    PRIMARY KEY (utc_day, ticker),
    CHECK (ticker IN ('ust1', 'ustr'))
);

CREATE INDEX idx_defillama_daily_assets_day ON defillama_daily_assets (utc_day);

COMMENT ON TABLE defillama_daily_assets IS
    'UTC-day UST1/USTR DEX volume + treasury fees + hub mark. #631';
