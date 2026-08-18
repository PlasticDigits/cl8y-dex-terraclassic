-- GitLab #556: materialized DEX hub USD (cUSTC / UST1 / USTR).
-- Refreshed on book-snapshot + oracle tick. Reads are O(1). Not CEX oracle_prices.

CREATE TABLE hub_prices (
    ticker TEXT PRIMARY KEY,
    asset_id INTEGER REFERENCES assets(id) ON DELETE SET NULL,
    price_usd NUMERIC(38, 18) NOT NULL,
    source_pair_id INTEGER REFERENCES pairs(id) ON DELETE SET NULL,
    source_pair_address TEXT,
    tvl_usd NUMERIC(38, 18),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT hub_prices_ticker_chk CHECK (ticker IN ('custc', 'ust1', 'ustr')),
    CONSTRAINT hub_prices_usd_positive CHECK (price_usd > 0)
);

CREATE INDEX idx_hub_prices_asset_id ON hub_prices (asset_id)
    WHERE asset_id IS NOT NULL;

COMMENT ON TABLE hub_prices IS
    'DEX hub USD marks (#556). usd(cUSTC)=USTC oracle; UST1/USTR from max USD-TVL factory reserves. Advisory, not settlement.';
