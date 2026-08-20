-- GitLab #569: protocol-wide factory pool TVL + snapshot history for 24h/30d Δ%.
-- GET /overview must stay O(1) rollup read — never SUM live pair TVL or walk snapshots.
-- Current TVL is humanized AMM reserves (pair_reserves), not book escrow / CG liquidity_in_usd.

ALTER TABLE global_stats_24h
    ADD COLUMN IF NOT EXISTS total_liquidity_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS liquidity_change_24h_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS liquidity_change_30d_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS priced_pair_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS unpriced_pair_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_liquidity_usd_24h_ago NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS total_liquidity_usd_30d_ago NUMERIC(38, 18);

CREATE TABLE IF NOT EXISTS global_liquidity_snapshots (
    sampled_at TIMESTAMPTZ PRIMARY KEY,
    total_liquidity_usd NUMERIC(38, 18) NOT NULL,
    priced_pair_count INT NOT NULL
);

COMMENT ON TABLE global_liquidity_snapshots IS
    'Periodic global AMM pool TVL (USD). Retain >= 35 days. Δ% looks up nearest sampled_at to now-24h / now-30d within ±30 min. Not reconstructed from liquidity_events.';
COMMENT ON COLUMN global_stats_24h.total_liquidity_usd IS
    'Humanized USD TVL of priced factory pair_reserves (P522-Q + hub). Idle / nothing priced = 0, not null. Not volume, not CG liquidity_in_usd, not book escrow.';
COMMENT ON COLUMN global_stats_24h.liquidity_change_24h_pct IS
    'Signed percent vs nearest snapshot to now()-24h within ±30 min. NULL if no baseline / then=0 / overflow. Not Infinity.';
COMMENT ON COLUMN global_stats_24h.liquidity_change_30d_pct IS
    'Signed percent vs nearest snapshot to now()-30d within ±30 min. NULL if no baseline / then=0 / overflow.';
COMMENT ON COLUMN global_stats_24h.priced_pair_count IS
    'Factory pairs included in total_liquidity_usd this refresh. Not a Protocol headline.';
