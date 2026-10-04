-- #1380: supersede pre-quality metadata without rewriting old migration checksums.
COMMENT ON COLUMN global_stats_24h.total_liquidity_usd IS
 'Nullable humanized supported-catalog factory AMM TVL. fresh/verified_empty are qualified totals, partial is a labeled lower bound, unknown/stale/unavailable are not current known zero. Excludes book escrow.';
COMMENT ON COLUMN global_stats_24h.liquidity_as_of IS
 'Original oldest used reserve/hub observation, or original complete empty-factory enumeration. Unrelated volume updates never renew liquidity time.';
COMMENT ON COLUMN global_stats_24h.liquidity_generation IS
 'Monotonic atomic publication generation shared with pair stamps, source terminal observation and all three liquidity grains.';
COMMENT ON TABLE global_liquidity_snapshots IS
 'Last actual valuation attempt per four-minute UTC terminal_slot, including degraded attempts. Legacy unqualified values remain unknown. Preserve grains before pruning >=35-day source retention.';
COMMENT ON COLUMN global_liquidity_snapshots.terminal_slot IS
 'Four-minute UTC slot; sampled_at is the actual last attempt, not slot start. A terminal outage cannot be hidden by snapshot throttling.';
COMMENT ON COLUMN global_stats_24h.liquidity_change_24h_pct IS
 'NULL unless current and nearest ~24h terminal baseline are fresh/verified_empty with identical supported inventory identity and positive baseline. Never skip a nearer degraded observation.';
COMMENT ON COLUMN global_stats_24h.liquidity_change_30d_pct IS
 'NULL unless current and nearest ~30d terminal baseline are fresh/verified_empty with identical supported inventory identity and positive baseline.';
COMMENT ON COLUMN protocol_hourly_liquidity.liquidity_usd IS
 'Terminal hourly valuation; NULL includes no sample or degraded unknown, disambiguated by quality/reasons. Never an earlier healthy sample relabeled as close.';
COMMENT ON COLUMN protocol_daily_liquidity.liquidity_usd IS
 'Terminal daily valuation; NULL includes no sample or degraded unknown, disambiguated by quality/reasons. Not sum/average.';
COMMENT ON COLUMN protocol_monthly_liquidity.liquidity_usd IS
 'Terminal monthly valuation including unknown close. Retained independently of source pruning; missing new source does not erase retained history.';
