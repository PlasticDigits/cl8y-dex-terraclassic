//! Mirrored v2 pair reserves (GitLab #279 Phase 1a) — current pool state in Postgres so the
//! hybrid solver can price the pool leg without an LCD call. One row per pair, replaced per snapshot.

use bigdecimal::BigDecimal;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PairReservesRow {
    pub pair_id: i32,
    /// Reserve of `asset_0` (the pair's token0), raw integer units.
    pub reserve_0: BigDecimal,
    /// Reserve of `asset_1` (the pair's token1), raw integer units.
    pub reserve_1: BigDecimal,
    pub fee_bps: i16,
    /// Chain height the snapshot was taken at, when known.
    pub block_height: Option<i64>,
}

/// Upsert the current reserves snapshot for a pair (replaces any prior row).
pub async fn upsert_pair_reserves(
    pool: &PgPool,
    pair_id: i32,
    reserve_0: &BigDecimal,
    reserve_1: &BigDecimal,
    fee_bps: i16,
    block_height: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pair_reserves
            (pair_id, reserve_0, reserve_1, fee_bps, block_height, snapshot_at)
         VALUES ($1, $2, $3, $4, $5, NOW())
         ON CONFLICT (pair_id) DO UPDATE SET
            reserve_0 = EXCLUDED.reserve_0,
            reserve_1 = EXCLUDED.reserve_1,
            fee_bps = EXCLUDED.fee_bps,
            block_height = EXCLUDED.block_height,
            snapshot_at = NOW()",
    )
    .bind(pair_id)
    .bind(reserve_0)
    .bind(reserve_1)
    .bind(fee_bps)
    .bind(block_height)
    .execute(pool)
    .await?;
    Ok(())
}

/// The current reserves snapshot for a pair, or `None` if it has not been snapshotted yet
/// (callers treat a missing snapshot as degrade-not-error).
pub async fn get_pair_reserves(
    pool: &PgPool,
    pair_id: i32,
) -> Result<Option<PairReservesRow>, sqlx::Error> {
    sqlx::query_as::<_, PairReservesRow>(
        "SELECT pair_id, reserve_0, reserve_1, fee_bps, block_height
         FROM pair_reserves WHERE pair_id = $1",
    )
    .bind(pair_id)
    .fetch_optional(pool)
    .await
}
