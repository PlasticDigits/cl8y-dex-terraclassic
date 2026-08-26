-- Per-pair factory v2 AMM TVL stamp (GitLab #664 / shared with #655).
-- Written by protocol TVL refresh — not on GET /api/v1/pairs/{addr}.
-- Unpriced / stale / same-asset / overflow pairs have no row (LEFT JOIN → NULL, not $0).

CREATE TABLE IF NOT EXISTS pair_liquidity_usd (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    liquidity_usd NUMERIC(38, 18) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pair_liquidity_usd_usd
    ON pair_liquidity_usd (liquidity_usd DESC);

COMMENT ON TABLE pair_liquidity_usd IS
    'Human USD of priced factory pair_reserves (protocol_pair_tvl); refreshed with protocol TVL — not live GET.';
COMMENT ON COLUMN pair_liquidity_usd.liquidity_usd IS
    'Human USD decimal. Absent row = unpriced (omit JSON / em-dash). Never COALESCE to 0 for display.';
