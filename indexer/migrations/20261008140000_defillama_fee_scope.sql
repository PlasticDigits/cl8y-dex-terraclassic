-- #1354: preserve explicit unresolved legacy pair scope in rollup-only responses.
ALTER TABLE defillama_daily_fees
    ADD COLUMN unresolved_pair_count BIGINT NOT NULL DEFAULT 0
    CHECK (unresolved_pair_count >= 0);
COMMENT ON COLUMN defillama_daily_fees.unresolved_pair_count IS
    'Legacy pair-linked fee events without a uniquely resolved pair; included in headline but never allocated to an asset. Historical rows require explicit reaggregation (#1354).';
