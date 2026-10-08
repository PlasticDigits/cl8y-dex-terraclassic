-- #1354: e.pair_id is authoritative. Legacy rows resolve only when all
-- source-specific candidate events identify one pair. Mixed/absent candidates
-- remain explicitly unresolved; no transaction-wide gem exclusion or allocation.
WITH gem_pairs AS (
    SELECT p.id
    FROM pairs p
    JOIN assets a0 ON a0.id = p.asset_0_id
    JOIN assets a1 ON a1.id = p.asset_1_id
    WHERE LOWER(COALESCE(a0.contract_address, '')) = ANY($3)
       OR LOWER(COALESCE(a1.contract_address, '')) = ANY($3)
), scoped_fee_events AS (
    SELECT e.*, COALESCE(e.pair_id, legacy.pair_id) AS resolved_pair_id,
           e.source IN ('swap_amm', 'book_take', 'limit_place')
           AND e.pair_id IS NULL AND legacy.pair_id IS NULL AS unresolved_pair_scope
    FROM protocol_fee_events e
    LEFT JOIN LATERAL (
        SELECT CASE WHEN COUNT(DISTINCT candidate.pair_id) = 1
                    THEN MIN(candidate.pair_id) END AS pair_id
        FROM (
            SELECT se.pair_id FROM swap_events se
            WHERE e.source = 'swap_amm' AND se.tx_hash = e.tx_hash
              AND se.swap_index = e.ordinal
            UNION ALL
            SELECT f.pair_id FROM limit_order_fills f
            WHERE e.source = 'book_take' AND f.tx_hash = e.tx_hash
              AND f.order_id = e.ordinal
            UNION ALL
            SELECT pl.pair_id FROM limit_order_placements pl
            WHERE e.source = 'limit_place' AND pl.tx_hash = e.tx_hash
              AND pl.order_id = e.ordinal
        ) candidate
    ) legacy ON e.pair_id IS NULL
                AND e.source IN ('swap_amm', 'book_take', 'limit_place')
    WHERE e.block_timestamp >= $1 AND e.block_timestamp < $2
)
