-- psql -X -v ON_ERROR_STOP=1 -v start=2026-08-17 -v end=2026-10-07 -f this-file
-- Dedicated read-only DB connection; no actors, transactions, DSNs or repricing.
-- This report does not refresh rollups. Exact same-snapshot rows stay private.
BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;
SET LOCAL timezone = 'UTC';
SET LOCAL statement_timeout = '30s';
SET LOCAL lock_timeout = '500ms';
-- Reject an unbounded or in-progress range before scanning event tables.
SELECT (:'start'::date >= DATE '2026-08-17'
        AND :'end'::date >= :'start'::date
        AND :'end'::date < CURRENT_DATE
        AND :'end'::date - :'start'::date < 400) AS valid_range \gset
\if :valid_range
\else
  \echo 'ERROR: require 1-400 completed UTC days at/after adapter start'
  \quit 2
\endif

WITH gem_addresses(address) AS (VALUES
    ('terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94'),
    ('terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena'),
    ('terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr'),
    ('terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9'),
    ('terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc'),
    ('terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme'),
    ('terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z'),
    ('terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs')
), all_gem_pairs AS (
    SELECT p.id FROM pairs p
    JOIN assets a0 ON a0.id = p.asset_0_id
    JOIN assets a1 ON a1.id = p.asset_1_id
    WHERE LOWER(COALESCE(a0.contract_address, '')) IN (SELECT address FROM gem_addresses)
       OR LOWER(COALESCE(a1.contract_address, '')) IN (SELECT address FROM gem_addresses)
), classified AS (
    SELECT e.*, e.block_timestamp::date AS utc_day,
      EXISTS (SELECT 1 FROM swap_events s JOIN all_gem_pairs g ON g.id=s.pair_id
              WHERE s.tx_hash=e.tx_hash) AS gem_swap_tx,
      EXISTS (SELECT 1 FROM limit_order_fills f JOIN all_gem_pairs g ON g.id=f.pair_id
              WHERE f.tx_hash=e.tx_hash) AS gem_fill_tx,
      EXISTS (SELECT 1 FROM limit_order_placements p JOIN all_gem_pairs g ON g.id=p.pair_id
              WHERE p.tx_hash=e.tx_hash) AS gem_place_tx,
      COALESCE(e.pair_id, legacy.pair_id) AS resolved_pair_id,
      COALESCE(e.pair_id, legacy.pair_id) IN (SELECT id FROM all_gem_pairs) AS own_pair_is_gem
    FROM protocol_fee_events e
    LEFT JOIN LATERAL (
      SELECT CASE WHEN COUNT(DISTINCT c.pair_id)=1 THEN MIN(c.pair_id) END AS pair_id
      FROM (
        SELECT s.pair_id FROM swap_events s WHERE e.source='swap_amm'
          AND s.tx_hash=e.tx_hash AND s.swap_index=e.ordinal
        UNION ALL
        SELECT f.pair_id FROM limit_order_fills f WHERE e.source='book_take'
          AND f.tx_hash=e.tx_hash AND f.order_id=e.ordinal
        UNION ALL
        SELECT p.pair_id FROM limit_order_placements p WHERE e.source='limit_place'
          AND p.tx_hash=e.tx_hash AND p.order_id=e.ordinal
      ) c
    ) legacy ON e.pair_id IS NULL AND e.source IN ('swap_amm','book_take','limit_place')
    WHERE e.block_timestamp >= :'start'::date
      AND e.block_timestamp < :'end'::date + INTERVAL '1 day'
), scoped AS (
    SELECT *, CASE source
        WHEN 'swap_amm' THEN NOT gem_swap_tx
        WHEN 'book_take' THEN NOT gem_fill_tx
        WHEN 'limit_place' THEN NOT gem_place_tx
        ELSE TRUE END AS included_by_legacy_tx_rule,
      COALESCE(NOT own_pair_is_gem,TRUE) AS included_by_current_rollup,
      CASE WHEN resolved_pair_id IS NOT NULL THEN NOT own_pair_is_gem
        WHEN source NOT IN ('swap_amm','book_take','limit_place') THEN TRUE
        ELSE NULL END AS included_by_verified_pair_scope
    FROM classified
), per_pair AS (
    SELECT utc_day, source, pair_id, asset_id,
      COUNT(*) AS event_count,
      COUNT(*) FILTER (WHERE fee_usd IS NULL OR fee_usd <= 0) AS unpriced_count,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0),0) AS ledger_priced_usd,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0 AND included_by_current_rollup),0) AS current_rule_usd,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0 AND included_by_verified_pair_scope),0) AS verified_pair_included_usd,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0 AND NOT included_by_verified_pair_scope),0) AS verified_pair_excluded_usd,
      COUNT(*) FILTER (WHERE included_by_verified_pair_scope IS NULL) AS unresolved_scope_events,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0 AND included_by_verified_pair_scope
                                  AND NOT included_by_legacy_tx_rule),0) AS mixed_tx_lost_priced_usd
    FROM scoped GROUP BY utc_day, source, pair_id, asset_id
)
SELECT pp.*,
  SUM(pp.current_rule_usd) OVER (PARTITION BY pp.utc_day, pp.source) AS current_rule_day_source_usd,
  f.amount_usd AS stored_day_source_usd,
  SUM(pp.current_rule_usd) OVER (PARTITION BY pp.utc_day, pp.source) - f.amount_usd AS stale_rollup_delta
FROM per_pair pp LEFT JOIN defillama_daily_fees f
  ON f.utc_day=pp.utc_day AND f.source=pp.source
ORDER BY pp.utc_day, pp.source, pp.pair_id NULLS LAST, pp.asset_id;

-- Stored rollups versus ledger/current rules by day+source, including empty days.
-- Per-pair report above is needed to explain intentional gem exclusions;
-- this ledger total deliberately retains the broader protocol catalog.
WITH days AS (
    SELECT generate_series(:'start'::date, :'end'::date, INTERVAL '1 day')::date AS utc_day
), ledger AS (
    SELECT block_timestamp::date AS utc_day, source, COUNT(*) AS event_count,
      COUNT(*) FILTER (WHERE fee_usd IS NULL OR fee_usd <= 0) AS unpriced_count,
      COALESCE(SUM(fee_usd) FILTER (WHERE fee_usd > 0),0) AS ledger_priced_usd
    FROM protocol_fee_events
    WHERE block_timestamp >= :'start'::date AND block_timestamp < :'end'::date + INTERVAL '1 day'
    GROUP BY block_timestamp::date, source
), keys AS (
    SELECT utc_day, source FROM ledger
    UNION SELECT utc_day, source FROM defillama_daily_fees
      WHERE utc_day BETWEEN :'start'::date AND :'end'::date
)
SELECT d.utc_day, k.source, l.event_count AS ledger_event_count, l.unpriced_count,
  l.ledger_priced_usd, f.amount_usd AS llama_rollup_source_usd,
  f.event_count AS llama_rollup_event_count, f.unpriced_count AS llama_rollup_unpriced_count,
  s.refreshed_at AS llama_refreshed_at,
  p.fees_usd AS protocol_daily_usd, p.refreshed_at AS protocol_refreshed_at
FROM days d LEFT JOIN keys k ON k.utc_day=d.utc_day
LEFT JOIN ledger l ON l.utc_day=k.utc_day AND l.source=k.source
LEFT JOIN defillama_daily_fees f ON f.utc_day=k.utc_day AND f.source=k.source
LEFT JOIN defillama_daily_stats s ON s.utc_day=d.utc_day
LEFT JOIN protocol_daily_fees p ON p.utc_day=d.utc_day
ORDER BY d.utc_day,k.source;
ROLLBACK;
