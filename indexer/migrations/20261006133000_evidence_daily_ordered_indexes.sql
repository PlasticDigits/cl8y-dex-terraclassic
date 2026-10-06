-- #1397: equality on UTC day followed by the exact source pagination order.
-- LIMIT can stop each ordered source early, and empty dates do not scan history.
-- AT TIME ZONE 'UTC' avoids session timezone-dependent date casts.
CREATE INDEX IF NOT EXISTS evidence_swaps_day_order_idx ON swap_events
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash,
     (COALESCE(swap_index, 0)::bigint), id);
CREATE INDEX IF NOT EXISTS evidence_lp_day_order_idx ON liquidity_events
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash, event_type, id);
CREATE INDEX IF NOT EXISTS evidence_placements_day_order_idx ON limit_order_placements
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash, id);
CREATE INDEX IF NOT EXISTS evidence_cancellations_day_order_idx ON limit_order_cancellations
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash, id);
CREATE INDEX IF NOT EXISTS evidence_fills_day_order_idx ON limit_order_fills
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash, id);
CREATE INDEX IF NOT EXISTS evidence_wrap_day_order_idx ON protocol_fee_events
    (((block_timestamp AT TIME ZONE 'UTC')::date), block_height, tx_hash, source, ordinal, id)
    WHERE source IN ('wrap', 'unwrap');
