//! Materialized per-pair AMM TVL for `GET /api/v1/pairs` (GitLab #655)
//! and single-pair GET (GitLab #664).
//!
//! Stamped on `refresh_protocol_liquidity`. List GET only JOINs this table.
//! Unpriced pairs have **no row** (LEFT JOIN → NULL / omit JSON) — never `$0`.

use bigdecimal::BigDecimal;
use sqlx::PgPool;

/// Replace the rollup: delete the previous set, then insert currently priced pairs.
///
/// Absent row = unpriced (stale / uncatalogued / overflow) — not `$0`, not a NULL column.
pub async fn replace_stamps(
    pool: &PgPool,
    stamps: &[(i32, BigDecimal)],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pair_liquidity_usd")
        .execute(&mut *tx)
        .await?;

    if !stamps.is_empty() {
        let ids: Vec<i32> = stamps.iter().map(|(id, _)| *id).collect();
        let usd: Vec<BigDecimal> = stamps.iter().map(|(_, v)| v.clone()).collect();
        sqlx::query(
            r#"INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at)
               SELECT u.pair_id, u.liquidity_usd, NOW()
               FROM UNNEST($1::int[], $2::numeric[]) AS u(pair_id, liquidity_usd)"#,
        )
        .bind(&ids)
        .bind(&usd)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn get_pair_liquidity_usd(
    pool: &PgPool,
    pair_id: i32,
) -> Result<Option<BigDecimal>, sqlx::Error> {
    sqlx::query_scalar("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
        .bind(pair_id)
        .fetch_optional(pool)
        .await
}
