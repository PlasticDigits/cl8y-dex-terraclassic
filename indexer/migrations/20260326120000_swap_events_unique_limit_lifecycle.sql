-- Deduplicate swap_events before unique constraint; repoint limit_order_fills to kept row.
UPDATE limit_order_fills l
SET swap_event_id = sub.keep_id
FROM (
    SELECT s.id AS dup_id, m.keep_id
    FROM swap_events s
    INNER JOIN (
        SELECT tx_hash, pair_id, MIN(id) AS keep_id
        FROM swap_events
        GROUP BY tx_hash, pair_id
    ) m ON s.tx_hash = m.tx_hash AND s.pair_id = m.pair_id
    WHERE s.id <> m.keep_id
) sub
WHERE l.swap_event_id = sub.dup_id;

WITH dup_ids AS (
    SELECT s.id
    FROM swap_events s
    INNER JOIN (
        SELECT tx_hash, pair_id, MIN(id) AS keep_id
        FROM swap_events
        GROUP BY tx_hash, pair_id
    ) m ON s.tx_hash = m.tx_hash AND s.pair_id = m.pair_id AND s.id <> m.keep_id
)
DELETE FROM swap_events WHERE id IN (SELECT id FROM dup_ids);

CREATE UNIQUE INDEX IF NOT EXISTS idx_swap_events_tx_hash_pair_id ON swap_events (tx_hash, pair_id);

-- Limit order lifecycle (place / cancel wasm events). Optional columns may be NULL when not emitted on-chain.
CREATE TABLE IF NOT EXISTS limit_order_placements (
    id BIGSERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    order_id BIGINT NOT NULL,
    owner VARCHAR(64),
    side VARCHAR(3) CHECK (side IS NULL OR side IN ('bid', 'ask')),
    price NUMERIC(38, 18),
    expires_at BIGINT,
    UNIQUE (tx_hash, pair_id, order_id)
);
CREATE INDEX IF NOT EXISTS idx_lo_placements_pair_time ON limit_order_placements (pair_id, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS limit_order_cancellations (
    id BIGSERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    order_id BIGINT NOT NULL,
    owner VARCHAR(64),
    UNIQUE (tx_hash, pair_id, order_id)
);
CREATE INDEX IF NOT EXISTS idx_lo_cancellations_pair_time ON limit_order_cancellations (pair_id, block_timestamp DESC);
