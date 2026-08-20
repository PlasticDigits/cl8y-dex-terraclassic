//! One-shot as-of oracle repair for flattened USTC/LUNC USD (GitLab #568).
//!
//! Historical `hub_prices` were never stored, so UST1/USTR-quoted `price_usd` cannot be
//! replayed. USTC/cUSTC/`uusd` and LUNC/cLUNC/`uluna` quotes *can* be rebuilt from
//! `oracle_prices` (`source = 'average'`) as-of `block_timestamp` / candle `open_time`.
//! Not called from the 10s hub loop.

use sqlx::PgPool;

/// Re-stamp USTC/LUNC-quoted `swap_events.price_usd` from as-of CEX average.
/// Native gems spoofing catalog symbols (non-`uusd`/`uluna` denom) are skipped.
pub async fn repair_ustc_lunc_swap_price_usd(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        r#"
UPDATE swap_events se
SET price_usd = se.price * sub.price_usd
FROM (
    SELECT se2.id, op.price_usd
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
) sub
WHERE se.id = sub.id
"#,
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// Re-stamp USTC/LUNC-quoted candle USD from preserved human OHLC × as-of oracle at `open_time`.
/// Does not invent `trade_count` / volume. UST1/USTR-quoted bars are left untouched.
pub async fn repair_ustc_lunc_candle_usd(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        r#"
UPDATE candles c
SET open = c.open_human * sub.price_usd,
    high = c.high_human * sub.price_usd,
    low = c.low_human * sub.price_usd,
    close = c.close_human * sub.price_usd,
    updated_at = NOW()
FROM (
    SELECT c2.id, op.price_usd
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
) sub
WHERE c.id = sub.id
"#,
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// Idempotent one-shot repair used by the #568 migration and tests.
pub async fn repair_ustc_lunc_usd_as_of_oracle(pool: &PgPool) -> Result<(), sqlx::Error> {
    repair_ustc_lunc_swap_price_usd(pool).await?;
    repair_ustc_lunc_candle_usd(pool).await?;
    Ok(())
}
