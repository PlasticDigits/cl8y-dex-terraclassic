//! Independently observed raw pool reserves (#1453). TVL and pool-only quotes read
//! current pool state; hybrid quotes read a separately atomic reserve/book generation.

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PairReservesRow {
    pub pair_id: i32,
    /// Reserve of `asset_0` (the pair's token0), raw integer units.
    pub reserve_0: BigDecimal,
    /// Reserve of `asset_1` (the pair's token1), raw integer units.
    pub reserve_1: BigDecimal,
    /// -1 means no compatible fee observation; cached quote consumers exclude it.
    pub fee_bps: i16,
    /// Chain height the snapshot was taken at, when known.
    pub block_height: Option<i64>,
    pub snapshot_at: DateTime<Utc>,
}

/// Upsert the current reserves snapshot for a pair (replaces any prior row).
pub async fn upsert_pair_reserves<'e, E>(
    executor: E,
    pair_id: i32,
    reserve_0: &BigDecimal,
    reserve_1: &BigDecimal,
    fee_bps: i16,
    block_height: Option<i64>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // Compatibility entrypoint for fixtures and callers supplying a full generation.
    // The production pool-only writer uses `upsert_pool_observation` instead.
    sqlx::query(
        "WITH observation AS (
            INSERT INTO pair_reserves
                (pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at)
            VALUES ($1, $2, $3, $4, $5, clock_timestamp())
            ON CONFLICT (pair_id) DO UPDATE SET
                reserve_0 = EXCLUDED.reserve_0, reserve_1 = EXCLUDED.reserve_1,
                fee_bps = EXCLUDED.fee_bps, block_height = EXCLUDED.block_height,
                snapshot_at = EXCLUDED.snapshot_at
            WHERE EXCLUDED.snapshot_at > pair_reserves.snapshot_at
              AND (pair_reserves.block_height IS NULL OR EXCLUDED.block_height >= pair_reserves.block_height)
            RETURNING *
        ) INSERT INTO hybrid_pair_reserves SELECT * FROM observation
          ON CONFLICT (pair_id) DO UPDATE SET
              reserve_0 = EXCLUDED.reserve_0, reserve_1 = EXCLUDED.reserve_1,
              fee_bps = EXCLUDED.fee_bps, block_height = EXCLUDED.block_height,
              snapshot_at = EXCLUDED.snapshot_at
          WHERE EXCLUDED.snapshot_at > hybrid_pair_reserves.snapshot_at
            AND (hybrid_pair_reserves.block_height IS NULL OR EXCLUDED.block_height >= hybrid_pair_reserves.block_height)",
    )
    .bind(pair_id).bind(reserve_0).bind(reserve_1).bind(fee_bps).bind(block_height)
    .execute(executor).await?;
    Ok(())
}

/// The current reserves snapshot for a pair, or `None` if it has not been snapshotted yet
/// (callers treat a missing snapshot as degrade-not-error).
pub async fn get_pair_reserves<'e, E>(
    executor: E,
    pair_id: i32,
) -> Result<Option<PairReservesRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, PairReservesRow>(
        "SELECT pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at
         FROM pair_reserves WHERE pair_id = $1",
    )
    .bind(pair_id)
    .fetch_optional(executor)
    .await
}

/// Set-based reserve snapshots for CG/CMC bid/ask (GitLab #685). No LCD.
pub async fn get_all_pair_reserves(
    pool: &PgPool,
) -> Result<HashMap<i32, PairReservesRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, PairReservesRow>(
        "SELECT pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at FROM pair_reserves",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.pair_id, r)).collect())
}

/// Publish a validated pool read independently, using only a compatible known fee and preventing
/// delayed/height-regressing observations from renewing the timestamp.
pub async fn upsert_pool_observation(
    pool: &PgPool,
    pair_id: i32,
    reserve_0: &BigDecimal,
    reserve_1: &BigDecimal,
    block_height: Option<i64>,
    observed_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at)
         VALUES ($1, $2, $3,
             COALESCE((SELECT fee_bps FROM pair_fee_observations
                 WHERE pair_id = $1 AND block_height = $4
                   AND observed_at <= $5 AND observed_at >= $5 - INTERVAL '60 seconds'), -1), $4, $5)
         ON CONFLICT (pair_id) DO UPDATE SET
             reserve_0 = EXCLUDED.reserve_0, reserve_1 = EXCLUDED.reserve_1,
             fee_bps = EXCLUDED.fee_bps,
             block_height = EXCLUDED.block_height, snapshot_at = EXCLUDED.snapshot_at
         WHERE EXCLUDED.snapshot_at > pair_reserves.snapshot_at
           AND (pair_reserves.block_height IS NULL OR EXCLUDED.block_height >= pair_reserves.block_height)",
    ).bind(pair_id).bind(reserve_0).bind(reserve_1).bind(block_height).bind(observed_at)
     .execute(pool).await?;
    Ok(result.rows_affected() == 1)
}

/// The last successful, separately atomic reserve/book generation. An independently
/// refreshed pool observation never renews this timestamp, including for empty books.
pub async fn get_hybrid_pair_reserves<'e, E>(
    executor: E,
    pair_id: i32,
) -> Result<Option<PairReservesRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, PairReservesRow>(
        "SELECT pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at
         FROM hybrid_pair_reserves WHERE pair_id = $1",
    )
    .bind(pair_id)
    .fetch_optional(executor)
    .await
}

/// Caller replaces the book only when this monotonic generation was admitted.
pub async fn upsert_hybrid_observation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    observation: &PairReservesRow,
    fee_bps: i16,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO hybrid_pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (pair_id) DO UPDATE SET
             reserve_0 = EXCLUDED.reserve_0, reserve_1 = EXCLUDED.reserve_1,
             fee_bps = EXCLUDED.fee_bps, block_height = EXCLUDED.block_height,
             snapshot_at = EXCLUDED.snapshot_at
         WHERE EXCLUDED.snapshot_at > hybrid_pair_reserves.snapshot_at
           AND (hybrid_pair_reserves.block_height IS NULL OR EXCLUDED.block_height >= hybrid_pair_reserves.block_height)",
    ).bind(observation.pair_id).bind(&observation.reserve_0).bind(&observation.reserve_1)
     .bind(fee_bps).bind(observation.block_height).bind(observation.snapshot_at)
     .execute(&mut **tx).await?;
    Ok(result.rows_affected() == 1)
}
