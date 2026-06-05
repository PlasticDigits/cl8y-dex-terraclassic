-- GitLab #287: a single tx can emit more than one swap on the same pair (e.g. a route that
-- revisits a pair, or batched swaps). The old UNIQUE (tx_hash, pair_id) collapsed them into one
-- row via ON CONFLICT DO NOTHING, silently dropping every swap after the first. Add a
-- per-(tx_hash, pair_id) ordinal so each swap is stored and uniquely addressable.
ALTER TABLE swap_events ADD COLUMN IF NOT EXISTS swap_index INTEGER NOT NULL DEFAULT 0;

-- Backfill deterministically by insertion order. Existing rows are already one-per-(tx,pair)
-- from the earlier dedup migration, so this assigns 0 everywhere; the window stays correct if
-- any duplicate ever slipped in.
WITH ordered AS (
    SELECT id,
           (row_number() OVER (PARTITION BY tx_hash, pair_id ORDER BY id) - 1)::int AS idx
    FROM swap_events
)
UPDATE swap_events s
SET swap_index = ordered.idx
FROM ordered
WHERE s.id = ordered.id AND s.swap_index <> ordered.idx;

-- Replace the 2-column uniqueness with the 3-column key so same-(tx,pair) swaps coexist.
DROP INDEX IF EXISTS idx_swap_events_tx_hash_pair_id;
CREATE UNIQUE INDEX IF NOT EXISTS idx_swap_events_tx_hash_pair_id_swap_index
    ON swap_events (tx_hash, pair_id, swap_index);
