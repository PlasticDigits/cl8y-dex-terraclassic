-- GitLab #586: protocol treasury fee events + O(1) rollup for /overview and GET /protocol/fees.
-- GET paths must not SUM protocol_fee_events / swap_events / fills. Stamp fee_usd at ingest (#568).
-- Book taker fees come from limit_order_fills.commission_amount (not swap book_commission_amount) — L7.

CREATE TABLE IF NOT EXISTS protocol_fee_events (
    id BIGSERIAL PRIMARY KEY,
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash TEXT NOT NULL,
    source TEXT NOT NULL,
    ordinal BIGINT NOT NULL DEFAULT 0,
    asset_id INT NOT NULL REFERENCES assets(id),
    amount_raw NUMERIC(38, 0) NOT NULL,
    decimals SMALLINT NOT NULL,
    fee_usd NUMERIC(38, 18),
    UNIQUE (tx_hash, source, ordinal),
    CONSTRAINT protocol_fee_events_source_chk CHECK (
        source IN ('swap_amm', 'book_take', 'limit_place', 'wrap', 'unwrap')
    ),
    CONSTRAINT protocol_fee_events_amount_chk CHECK (amount_raw > 0)
);

CREATE INDEX IF NOT EXISTS protocol_fee_events_ts_idx
    ON protocol_fee_events (block_timestamp);
CREATE INDEX IF NOT EXISTS protocol_fee_events_source_ts_idx
    ON protocol_fee_events (source, block_timestamp);
CREATE INDEX IF NOT EXISTS protocol_fee_events_asset_ts_idx
    ON protocol_fee_events (asset_id, block_timestamp);

COMMENT ON TABLE protocol_fee_events IS
    'Canonical treasury fee rows (GitLab #586). Unique (tx_hash, source, ordinal) so poller replay does not double-count. fee_usd stamped at ingest — do not rewrite from live hub.';
COMMENT ON COLUMN protocol_fee_events.source IS
    'swap_amm = pool commission_amount; book_take = limit_order_fills.commission_amount; limit_place = maker_fee_amount; wrap/unwrap = pinned WRAP_MAPPER_ADDRESS only.';
COMMENT ON COLUMN protocol_fee_events.fee_usd IS
    'Humanized × P522-Q/hub as-of ingest. NULL when unpriced / overflow. Never vFDUSD. Never $1 UST1.';

ALTER TABLE limit_order_placements
    ADD COLUMN IF NOT EXISTS maker_fee_amount NUMERIC(38, 0);

COMMENT ON COLUMN limit_order_placements.maker_fee_amount IS
    'On-chain maker_fee_amount (offer/escrow token). Zero/NULL places are not protocol fee events.';

ALTER TABLE global_stats_24h
    ADD COLUMN IF NOT EXISTS total_fees_24h_usd NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS total_fees_7d_usd NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS total_fees_30d_usd NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS fees_change_24h_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS fees_change_7d_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS fees_change_30d_pct NUMERIC(38, 18),
    ADD COLUMN IF NOT EXISTS fee_event_count_24h BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS fee_event_count_7d BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS fee_event_count_30d BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS wrap_mapper_configured BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN global_stats_24h.total_fees_24h_usd IS
    'SUM priced protocol_fee_events.fee_usd in trailing 24h. 0 = idle (no events). NULL = activity but all unpriced.';
COMMENT ON COLUMN global_stats_24h.fees_change_24h_pct IS
    'Flow Δ% vs prior equal window [now-48h, now-24h). NULL if no prior events / then<=0 / overflow. Never Infinity.';
COMMENT ON COLUMN global_stats_24h.wrap_mapper_configured IS
    'True when WRAP_MAPPER_ADDRESS parsed as a valid terra1 bech32 at last fee rollup. Unconfigured wrap/unwrap omitted from breakdown (not fake $0).';

CREATE TABLE IF NOT EXISTS protocol_fee_stats_by_source (
    "window" TEXT NOT NULL,
    source TEXT NOT NULL,
    amount_usd NUMERIC(38, 18),
    event_count BIGINT NOT NULL DEFAULT 0,
    share_pct NUMERIC(38, 18),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("window", source),
    CONSTRAINT protocol_fee_stats_by_source_window_chk CHECK ("window" IN ('24h', '7d', '30d')),
    CONSTRAINT protocol_fee_stats_by_source_source_chk CHECK (
        source IN ('swap_amm', 'book_take', 'limit_place', 'wrap', 'unwrap')
    )
);

CREATE TABLE IF NOT EXISTS protocol_fee_stats_by_token (
    "window" TEXT NOT NULL,
    asset_id INT REFERENCES assets(id),
    amount_human NUMERIC(38, 18),
    amount_usd NUMERIC(38, 18),
    share_pct NUMERIC(38, 18),
    rank INT NOT NULL,
    is_other BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("window", rank),
    CONSTRAINT protocol_fee_stats_by_token_window_chk CHECK ("window" IN ('24h', '7d', '30d'))
);

COMMENT ON TABLE protocol_fee_stats_by_source IS
    'Materialized fee USD by source for GET /api/v1/protocol/fees. GET must not scan protocol_fee_events.';
COMMENT ON TABLE protocol_fee_stats_by_token IS
    'Materialized top-8 tokens by USD + other (rank 9). other.asset_id is NULL; amount_human omitted (mixed units).';
