-- #1391: advisory hub marks need reserve-time freshness and fixed-reference receipts.
ALTER TABLE hub_prices ADD COLUMN as_of TIMESTAMPTZ;
ALTER TABLE hub_prices ADD COLUMN priced_leg_depth_usd NUMERIC(38,18);
ALTER TABLE hub_prices ADD COLUMN quality TEXT NOT NULL DEFAULT 'unverified';
CREATE TABLE hub_price_references (
    identity_key TEXT NOT NULL,
    last_observed_ratio NUMERIC(78,38) NOT NULL CHECK (last_observed_ratio > 0),
    ticker TEXT PRIMARY KEY CHECK (ticker IN ('ust1', 'ustr')),
    reference_ratio NUMERIC(78,38) NOT NULL CHECK (reference_ratio > 0),
    reference_at TIMESTAMPTZ NOT NULL,
    source_pair_address TEXT NOT NULL,
    pending_pair_address TEXT NOT NULL,
    pending_since TIMESTAMPTZ NOT NULL,
    last_observed_at TIMESTAMPTZ NOT NULL,
    confirmed BOOLEAN NOT NULL,
    quality TEXT NOT NULL,
    divergence_pct NUMERIC(78,18) NOT NULL
);
COMMENT ON TABLE hub_price_references IS 'Fixed USTC-unit reference, never auto-reset on outage; administrative reset requires independent review. Non-oracle-grade spots (#1391).';

ALTER TABLE economic_token_marks ADD COLUMN as_of TIMESTAMPTZ;

ALTER TABLE defillama_daily_assets ADD COLUMN price_as_of TIMESTAMPTZ;

ALTER TABLE economic_token_marks ADD COLUMN anchor_ticker TEXT CHECK (anchor_ticker IN ('custc','lunc','ust1','ustr'));
