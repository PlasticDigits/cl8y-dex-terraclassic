-- GitLab #631 — UTC calendar-day rollup for DeFiLlama volume/fees.
-- GET /api/v1/defillama/daily reads these tables only (no swap_events /
-- protocol_fee_events scan on the request path). Refresh is the ~5 min
-- volume loop. Trailing 24h overview windows (#576) stay separate.

CREATE TABLE defillama_daily_stats (
    utc_day DATE PRIMARY KEY,
    volume_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    trade_count BIGINT NOT NULL DEFAULT 0,
    unpriced_trade_count BIGINT NOT NULL DEFAULT 0,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE defillama_daily_fees (
    utc_day DATE NOT NULL REFERENCES defillama_daily_stats(utc_day) ON DELETE CASCADE,
    source TEXT NOT NULL,
    amount_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    event_count BIGINT NOT NULL DEFAULT 0,
    unpriced_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (utc_day, source),
    CHECK (source IN (
        'swap_amm', 'book_take', 'limit_place',
        'wrap', 'unwrap', 'ust1_mint', 'ust1_redeem'
    ))
);

CREATE INDEX idx_defillama_daily_fees_day ON defillama_daily_fees (utc_day);

COMMENT ON TABLE defillama_daily_stats IS
    'UTC-day hybrid volume (swap_events only, gem pairs excluded). Llama dailyVolume. #631';
COMMENT ON TABLE defillama_daily_fees IS
    'UTC-day PFee/L7 treasury fees by source (gem pair txs excluded for pair sources). #631';
