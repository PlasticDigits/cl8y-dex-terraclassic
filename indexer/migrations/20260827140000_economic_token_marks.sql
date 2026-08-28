-- GitLab #683: internal DEX reserve marks for factory-listed economic CW20s (CL8Y first).
-- Sibling of hub_prices so GET /hub-prices stay custc|lunc|ust1|ustr (H11). Advisory, not settlement.
-- NULL-only backfill of protocol_fee_events.fee_usd uses these marks as-of refresh — never rewrite non-null stamps.

CREATE TABLE IF NOT EXISTS economic_token_marks (
    contract_address TEXT PRIMARY KEY,
    asset_id INT REFERENCES assets(id),
    price_usd NUMERIC(38, 18) NOT NULL,
    source_pair_id INT,
    source_pair_address TEXT,
    tvl_usd NUMERIC(38, 18),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT economic_token_marks_usd_positive CHECK (price_usd > 0),
    CONSTRAINT economic_token_marks_contract_chk CHECK (
        contract_address ~ '^terra1[0-9a-z]{20,86}$'
    )
);

CREATE INDEX IF NOT EXISTS economic_token_marks_asset_id_idx
    ON economic_token_marks (asset_id)
    WHERE asset_id IS NOT NULL;

COMMENT ON TABLE economic_token_marks IS
    'Internal factory economic CW20 USD marks (#683). Largest USD-TVL pair vs already-priced hub/catalog. Not GET /hub-prices. Never gems / vFDUSD / $1 CL8Y.';
COMMENT ON COLUMN economic_token_marks.contract_address IS
    'CW20 bech32 (A1). Never a display ticker (CL8Y-cb / TCL8Y).';
COMMENT ON COLUMN economic_token_marks.price_usd IS
    'USD of 1 human unit from CPAMM reserves vs a priced hub leg. As-of snapshot.';

COMMENT ON COLUMN protocol_fee_events.fee_usd IS
    'Humanized × P522-Q/hub/economic-factory mark as-of ingest (#586 / #683). NULL when unpriced / overflow. Never vFDUSD. Never $1 UST1/CL8Y. Non-null stamps are never rewritten; NULL-only backfill is as-of, not historical MTM.';
