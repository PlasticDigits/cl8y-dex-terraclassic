-- Venus Core Pool vFDUSD redeem rate (GitLab #571).
-- Distinct from CEX FDUSD/USD in oracle_prices — this is FDUSD-out per 1 human vFDUSD,
-- never a USD quote and never mixed into volume_usd (X4).
-- Version 20260820200000 sits after same-day worktree timestamps (568/569 used 120000).

CREATE TABLE IF NOT EXISTS venus_vfdusd_rates (
    id BIGSERIAL PRIMARY KEY,
    fdusd_per_vfdusd NUMERIC(38, 18) NOT NULL,
    vtoken VARCHAR(42) NOT NULL,
    source VARCHAR(32) NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT venus_vfdusd_source_chk CHECK (source = 'venus_bsc'),
    CONSTRAINT venus_vfdusd_positive CHECK (fdusd_per_vfdusd > 0)
);

CREATE INDEX IF NOT EXISTS idx_venus_vfdusd_rates_fetched
    ON venus_vfdusd_rates (fetched_at DESC);
