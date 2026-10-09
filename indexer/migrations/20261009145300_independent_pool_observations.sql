-- #1453: a successful pool observation must survive unrelated fee/book failures.
-- Preserve the last compatible reserve/book generation separately. Backfill only
-- existing atomic snapshots, preserving their original time and height.
CREATE TABLE hybrid_pair_reserves (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    reserve_0 NUMERIC(39, 0) NOT NULL,
    reserve_1 NUMERIC(39, 0) NOT NULL,
    fee_bps SMALLINT NOT NULL,
    block_height BIGINT,
    snapshot_at TIMESTAMPTZ NOT NULL
);
INSERT INTO hybrid_pair_reserves SELECT * FROM pair_reserves;
-- Full Uint128 reserves require 39 decimal digits.
ALTER TABLE pair_reserves ALTER COLUMN reserve_0 TYPE NUMERIC(39, 0);
ALTER TABLE pair_reserves ALTER COLUMN reserve_1 TYPE NUMERIC(39, 0);
-- -1 is an explicitly unknown fee on a cold pool-only observation. TVL consumes
-- raw reserves; pool quote consumers must reject this sentinel.
COMMENT ON COLUMN pair_reserves.fee_bps IS '#1453: -1 means fee not observed; excluded by cached quote consumers';
CREATE TABLE pair_fee_observations (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    fee_bps SMALLINT NOT NULL CHECK (fee_bps BETWEEN 0 AND 10000),
    observed_at TIMESTAMPTZ NOT NULL,
    block_height BIGINT NOT NULL CHECK (block_height > 0)
);
-- The bounded book parser also accepts full Uint128 remaining amounts.
ALTER TABLE resting_limit_orders ALTER COLUMN remaining TYPE NUMERIC(39, 0);
-- Decimal has Uint128 atomics and 18 fractional digits. Its maximum needs 21
-- integer digits and is a valid human-scale price on a 6/18-decimal pair.
ALTER TABLE resting_limit_orders ALTER COLUMN price TYPE NUMERIC(39, 18);
