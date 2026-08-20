-- GitLab #568: one-shot repair of as-of-now rewritten USTC/LUNC USD.
--
-- Hub refresh used to stamp every historical swap_events.price_usd and candle USD
-- with the *current* hub mark. USTC/LUNC quotes can be rebuilt from oracle_prices
-- (source = 'average') as-of block_timestamp / candle open_time.
--
-- UST1/USTR-quoted history cannot be reconstructed (hub_prices is a live snapshot
-- only). Going forward, ingest stamps as-of hub and idle marks write current buckets
-- only — never a full-table rewrite.
--
-- Idempotent. Native gems spoofing catalog symbols are skipped (denom must be
-- uusd/uluna, or CW20 with a contract address).
-- LATERAL is inside a subquery so it can correlate to swap_events / candles
-- (PostgreSQL UPDATE ... FROM cannot LATERAL-reference the target table).

UPDATE swap_events se
SET price_usd = se.price * x.quote_usd
FROM (
    SELECT se2.id, op.price_usd AS quote_usd
    FROM swap_events se2
    JOIN pairs p ON p.id = se2.pair_id
    JOIN assets a1 ON a1.id = p.asset_1_id
    JOIN LATERAL (
        SELECT o.price_usd
        FROM oracle_prices o
        WHERE o.ticker = CASE
                WHEN a1.denom = 'uusd' THEN 'ustc'
                WHEN a1.denom = 'uluna' THEN 'lunc'
                WHEN a1.is_cw20 AND UPPER(a1.symbol) IN ('USTC', 'CUSTC') THEN 'ustc'
                WHEN a1.is_cw20 AND UPPER(a1.symbol) IN ('LUNC', 'CLUNC') THEN 'lunc'
                ELSE NULL
            END
          AND o.source = 'average'
          AND o.fetched_at <= se2.block_timestamp
          AND o.price_usd > 0
        ORDER BY o.fetched_at DESC
        LIMIT 1
    ) op ON TRUE
    WHERE se2.price > 0
      AND op.price_usd > 0
      AND se2.price * op.price_usd > 0
      AND se2.price * op.price_usd < POWER(10::numeric, 20)
      AND (
            a1.denom IN ('uusd', 'uluna')
            OR (
                a1.is_cw20
                AND COALESCE(a1.contract_address, '') <> ''
                AND UPPER(a1.symbol) IN ('USTC', 'CUSTC', 'LUNC', 'CLUNC')
            )
          )
) x
WHERE se.id = x.id;

UPDATE candles c
SET open = c.open_human * x.quote_usd,
    high = c.high_human * x.quote_usd,
    low = c.low_human * x.quote_usd,
    close = c.close_human * x.quote_usd,
    updated_at = NOW()
FROM (
    SELECT c2.id, op.price_usd AS quote_usd
    FROM candles c2
    JOIN pairs p ON p.id = c2.pair_id
    JOIN assets a1 ON a1.id = p.asset_1_id
    JOIN LATERAL (
        SELECT o.price_usd
        FROM oracle_prices o
        WHERE o.ticker = CASE
                WHEN a1.denom = 'uusd' THEN 'ustc'
                WHEN a1.denom = 'uluna' THEN 'lunc'
                WHEN a1.is_cw20 AND UPPER(a1.symbol) IN ('USTC', 'CUSTC') THEN 'ustc'
                WHEN a1.is_cw20 AND UPPER(a1.symbol) IN ('LUNC', 'CLUNC') THEN 'lunc'
                ELSE NULL
            END
          AND o.source = 'average'
          AND o.fetched_at <= c2.open_time
          AND o.price_usd > 0
        ORDER BY o.fetched_at DESC
        LIMIT 1
    ) op ON TRUE
    WHERE c2.open_human IS NOT NULL
      AND c2.high_human IS NOT NULL
      AND c2.low_human IS NOT NULL
      AND c2.close_human IS NOT NULL
      AND c2.open_human > 0
      AND op.price_usd > 0
      AND c2.high_human * op.price_usd > 0
      AND c2.high_human * op.price_usd < POWER(10::numeric, 20)
      AND (
            a1.denom IN ('uusd', 'uluna')
            OR (
                a1.is_cw20
                AND COALESCE(a1.contract_address, '') <> ''
                AND UPPER(a1.symbol) IN ('USTC', 'CUSTC', 'LUNC', 'CLUNC')
            )
          )
) x
WHERE c.id = x.id;
