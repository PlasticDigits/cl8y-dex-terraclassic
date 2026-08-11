-- Multi-ticker external oracle storage (GitLab #515).
-- Replaces ustc-only `ustc_prices` with ticker-scoped `oracle_prices` (ustc, lunc).

CREATE TABLE IF NOT EXISTS oracle_prices (
    id BIGSERIAL PRIMARY KEY,
    ticker VARCHAR(16) NOT NULL,
    price_usd NUMERIC(38, 18) NOT NULL,
    source VARCHAR(32) NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oracle_prices_ticker_fetched
    ON oracle_prices (ticker, fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_oracle_prices_ticker_source
    ON oracle_prices (ticker, source, fetched_at DESC);

INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
SELECT 'ustc', price_usd, source, fetched_at
FROM ustc_prices;

DROP TABLE IF EXISTS ustc_prices;
