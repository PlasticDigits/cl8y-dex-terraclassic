-- GitLab #1269: persist every multihop AMM hop in protocol_fee_events.
--
-- UNIQUE (tx_hash, source, ordinal) collided when parse_swaps restarted swap_index per pair
-- (#287). Hop 2+ in the same router tx shared (tx, swap_amm, 0); ON CONFLICT DO NOTHING kept
-- the first hop. PostgreSQL UNIQUE treats NULL as distinct, so a nullable pair_id without
-- partial uniques would break wrap / window replay. Two partial indexes:
--   pair-scoped: (tx_hash, source, pair_id, ordinal) WHERE pair_id IS NOT NULL  (swap_amm)
--   non-pair:    (tx_hash, source, ordinal)          WHERE pair_id IS NULL
--     (wrap / unwrap / ust1_* / book_take / limit_place — ordinal uniqueness unchanged)
-- Replay must skip duplicates, never overwrite stored amounts.
-- Do not edit 20260821120000_protocol_fees.sql in place.

ALTER TABLE protocol_fee_events
    ADD COLUMN IF NOT EXISTS pair_id INT REFERENCES pairs(id);

COMMENT ON COLUMN protocol_fee_events.pair_id IS
    'Factory pair for swap_amm (GitLab #1269). NULL for wrap / unwrap / ust1_mint / ust1_redeem / book_take / limit_place. Uniqueness is two partial indexes — never a nullable UNIQUE (tx_hash, source, ordinal).';

COMMENT ON TABLE protocol_fee_events IS
    'Canonical treasury fee rows (GitLab #586 / #1269). swap_amm unique (tx_hash, source, pair_id, ordinal) WHERE pair_id IS NOT NULL so multihop hops with per-pair swap_index=0 coexist. Non-pair sources unique (tx_hash, source, ordinal) WHERE pair_id IS NULL. Poller replay skips duplicate (tx, source, pair, ordinal) rows and never overwrites amounts. fee_usd stamped at ingest — do not rewrite from live hub.';

ALTER TABLE protocol_fee_events
    DROP CONSTRAINT IF EXISTS protocol_fee_events_tx_hash_source_ordinal_key;

DROP INDEX IF EXISTS protocol_fee_events_tx_hash_source_ordinal_key;

CREATE UNIQUE INDEX IF NOT EXISTS protocol_fee_events_pair_tx_source_ordinal_uidx
    ON protocol_fee_events (tx_hash, source, pair_id, ordinal)
    WHERE pair_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS protocol_fee_events_nopair_tx_source_ordinal_uidx
    ON protocol_fee_events (tx_hash, source, ordinal)
    WHERE pair_id IS NULL;

-- Attach surviving colliding swap_amm rows to the first matching hop (amount match preferred,
-- then earliest swap_events.id = ingest order). Then insert hops still missing.
UPDATE protocol_fee_events e
SET pair_id = s.pair_id
FROM (
    SELECT DISTINCT ON (e2.id)
        e2.id AS fee_id,
        se.pair_id
    FROM protocol_fee_events e2
    JOIN swap_events se
      ON se.tx_hash = e2.tx_hash
     AND se.swap_index = e2.ordinal
     AND se.commission_amount IS NOT NULL
     AND se.commission_amount > 0
    WHERE e2.source = 'swap_amm'
      AND e2.pair_id IS NULL
    ORDER BY e2.id,
             (se.commission_amount = e2.amount_raw) DESC,
             se.id ASC
) s
WHERE e.id = s.fee_id
  AND e.pair_id IS NULL
  AND e.source = 'swap_amm';

INSERT INTO protocol_fee_events
    (block_height, block_timestamp, tx_hash, source, ordinal,
     pair_id, asset_id, amount_raw, decimals, fee_usd)
SELECT
    se.block_height,
    se.block_timestamp,
    se.tx_hash,
    'swap_amm',
    se.swap_index,
    se.pair_id,
    se.ask_asset_id,
    se.commission_amount,
    a.decimals,
    NULL
FROM swap_events se
JOIN assets a ON a.id = se.ask_asset_id
WHERE se.commission_amount IS NOT NULL
  AND se.commission_amount > 0
ON CONFLICT DO NOTHING;

-- If attach failed and the INSERT created pair-scoped copies, drop the old 3-column
-- colliding row so the first hop is not double-counted.
DELETE FROM protocol_fee_events e
WHERE e.source = 'swap_amm'
  AND e.pair_id IS NULL
  AND EXISTS (
      SELECT 1
      FROM protocol_fee_events p
      WHERE p.source = 'swap_amm'
        AND p.tx_hash = e.tx_hash
        AND p.ordinal = e.ordinal
        AND p.pair_id IS NOT NULL
  );
