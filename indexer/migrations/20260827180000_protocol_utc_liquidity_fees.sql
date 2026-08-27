-- GitLab #689: UTC hourly/daily/monthly Protocol liquidity (stock) + fees (flow).
-- GET /protocol/liquidity/daily and GET /protocol/fees/daily read these tables only.
-- Do not SUM protocol_fee_events, walk global_liquidity_snapshots, or read Llama on GET.

CREATE TABLE IF NOT EXISTS protocol_hourly_liquidity (
    utc_hour TIMESTAMPTZ PRIMARY KEY,
    liquidity_usd NUMERIC(38, 18),
    priced_pair_count INT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_hourly_liquidity IS
    'UTC-hour Protocol pool TVL stock (GitLab #689). Last global_liquidity_snapshots row in [hour, hour+1). GET reads this table only — never walk snapshots / pair_reserves. NULL liquidity_usd = no snapshot in bucket (unknown ≠ $0). Prune ~10d (240h).';
COMMENT ON COLUMN protocol_hourly_liquidity.liquidity_usd IS
    'Last snapshot total_liquidity_usd in the hour. NULL when the bucket has no sample. Never SUM snapshots (would inflate TVL).';

CREATE INDEX IF NOT EXISTS protocol_hourly_liquidity_hour_idx
    ON protocol_hourly_liquidity (utc_hour DESC);

CREATE TABLE IF NOT EXISTS protocol_daily_liquidity (
    utc_day DATE PRIMARY KEY,
    liquidity_usd NUMERIC(38, 18),
    priced_pair_count INT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_daily_liquidity IS
    'UTC-day Protocol pool TVL stock (GitLab #689). Last snapshot in the UTC calendar day. GET reads this table only. Downsample before 35d snapshot prune. Daily retain ≥ 95d.';
COMMENT ON COLUMN protocol_daily_liquidity.liquidity_usd IS
    'Last snapshot in the UTC day. NULL = no sample that day.';

CREATE INDEX IF NOT EXISTS protocol_daily_liquidity_day_idx
    ON protocol_daily_liquidity (utc_day DESC);

CREATE TABLE IF NOT EXISTS protocol_monthly_liquidity (
    utc_month DATE PRIMARY KEY,
    liquidity_usd NUMERIC(38, 18),
    priced_pair_count INT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_monthly_liquidity IS
    'UTC calendar-month Protocol pool TVL stock (GitLab #689). Last snapshot in the month. Persist on the aggregator BEFORE global_liquidity_snapshots 35d prune so Monthly can retain ≥ 24 months. GET reads this table only.';
COMMENT ON COLUMN protocol_monthly_liquidity.utc_month IS
    'First day of the UTC calendar month. JSON: YYYY-MM.';

CREATE INDEX IF NOT EXISTS protocol_monthly_liquidity_month_idx
    ON protocol_monthly_liquidity (utc_month DESC);

CREATE TABLE IF NOT EXISTS protocol_hourly_fees (
    utc_hour TIMESTAMPTZ PRIMARY KEY,
    fees_usd NUMERIC(38, 18),
    event_count BIGINT NOT NULL DEFAULT 0,
    unpriced_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_hourly_fees IS
    'UTC-hour Protocol treasury fee flow (GitLab #689). SUM priced protocol_fee_events.fee_usd in [hour, hour+1). GET reads this table only — never SUM events / swap_events / Llama. Prune ~10d.';
COMMENT ON COLUMN protocol_hourly_fees.fees_usd IS
    '0 = idle (no events). NULL = activity but all unpriced. Mixed priced+unpriced = SUM priced (do not null the bucket). Never invent $0 from unpriced.';

CREATE INDEX IF NOT EXISTS protocol_hourly_fees_hour_idx
    ON protocol_hourly_fees (utc_hour DESC);

CREATE TABLE IF NOT EXISTS protocol_daily_fees (
    utc_day DATE PRIMARY KEY,
    fees_usd NUMERIC(38, 18),
    event_count BIGINT NOT NULL DEFAULT 0,
    unpriced_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_daily_fees IS
    'UTC-day Protocol treasury fee flow (GitLab #689). Same PFee sources / Protocol catalog as GET /protocol/fees (not defillama_daily_fees). GET reads this table only. Prune ≥ 95d.';

CREATE INDEX IF NOT EXISTS protocol_daily_fees_day_idx
    ON protocol_daily_fees (utc_day DESC);

CREATE TABLE IF NOT EXISTS protocol_monthly_fees (
    utc_month DATE PRIMARY KEY,
    fees_usd NUMERIC(38, 18),
    event_count BIGINT NOT NULL DEFAULT 0,
    unpriced_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE protocol_monthly_fees IS
    'UTC calendar-month Protocol treasury fee flow (GitLab #689). GET reads this table only. Retain ≥ 24 months.';
COMMENT ON COLUMN protocol_monthly_fees.utc_month IS
    'First day of the UTC calendar month. JSON: YYYY-MM.';

CREATE INDEX IF NOT EXISTS protocol_monthly_fees_month_idx
    ON protocol_monthly_fees (utc_month DESC);
