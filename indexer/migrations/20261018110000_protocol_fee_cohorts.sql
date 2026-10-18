-- #1211: private attribution is separate from the public materialized ledger.
CREATE TABLE protocol_fee_attributions (
    fee_event_id BIGINT PRIMARY KEY REFERENCES protocol_fee_events(id) ON DELETE CASCADE,
    actor TEXT CHECK (actor IS NULL OR (length(actor) BETWEEN 20 AND 90)),
    cohort TEXT NOT NULL CHECK (cohort IN ('retail', 'mm', 'unknown')),
    CHECK (actor IS NOT NULL OR cohort = 'unknown')
);
CREATE TABLE fee_registry_events (
    registry TEXT NOT NULL,
    block_height BIGINT NOT NULL,
    tx_index BIGINT NOT NULL,
    action_index BIGINT NOT NULL,
    actor TEXT,
    valid_state BOOLEAN NOT NULL DEFAULT true,
    registered BOOLEAN NOT NULL,
    tier_id SMALLINT NOT NULL CHECK (tier_id BETWEEN 0 AND 255),
    CHECK (NOT valid_state OR actor IS NOT NULL),
    PRIMARY KEY (registry, block_height, tx_index, action_index)
);
CREATE INDEX fee_registry_actor_position ON fee_registry_events
    (registry, actor, block_height DESC, tx_index DESC, action_index DESC);
CREATE TABLE fee_registry_snapshots (
    registry TEXT NOT NULL,
    actor TEXT NOT NULL,
    block_height BIGINT NOT NULL,
    registered BOOLEAN NOT NULL,
    tier_id SMALLINT NOT NULL CHECK (tier_id BETWEEN 0 AND 255),
    PRIMARY KEY (registry, actor, block_height)
);
CREATE TABLE protocol_fee_stats_by_cohort (
    "window" TEXT NOT NULL CHECK ("window" IN ('24h', '7d', '30d')),
    cohort TEXT NOT NULL CHECK (cohort IN ('retail', 'mm', 'unknown')),
    amount_usd NUMERIC(38,18),
    event_count BIGINT NOT NULL DEFAULT 0,
    share_pct NUMERIC(38,18),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("window", cohort)
);
