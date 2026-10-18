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

-- Before the new indexer performs its first scheduled refresh, preserve the
-- existing published fee snapshot. Historical payer membership is unobserved,
-- so all existing activity starts in unknown; no event scan or history rewrite.
-- Reading this one materialized row keeps migration initialization O(1).
INSERT INTO protocol_fee_stats_by_cohort
    ("window", cohort, amount_usd, event_count, share_pct, updated_at)
SELECT w."window", c.cohort,
       CASE WHEN c.cohort <> 'unknown' OR w.event_count <= 0 THEN 0
            WHEN w.amount_usd > 0 THEN w.amount_usd ELSE NULL END,
       CASE WHEN c.cohort = 'unknown' THEN w.event_count ELSE 0 END,
       CASE WHEN w.event_count > 0 AND w.amount_usd > 0
            THEN CASE WHEN c.cohort = 'unknown' THEN 100 ELSE 0 END
            ELSE NULL END,
       COALESCE(w.updated_at, NOW())
FROM (
    SELECT v."window", g.updated_at,
           CASE v."window"
             WHEN '24h' THEN g.total_fees_24h_usd
             WHEN '7d' THEN g.total_fees_7d_usd
             WHEN '30d' THEN g.total_fees_30d_usd
           END AS amount_usd,
           COALESCE(CASE v."window"
             WHEN '24h' THEN g.fee_event_count_24h
             WHEN '7d' THEN g.fee_event_count_7d
             WHEN '30d' THEN g.fee_event_count_30d
           END, 0) AS event_count
    FROM (VALUES ('24h'), ('7d'), ('30d')) AS v("window")
    LEFT JOIN global_stats_24h g ON g.id = 1
) AS w
CROSS JOIN (VALUES ('retail'), ('mm'), ('unknown')) AS c(cohort);
