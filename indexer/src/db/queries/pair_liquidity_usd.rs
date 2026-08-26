//! Materialized per-pair factory v2 AMM TVL (GitLab #664 / #655).
//!
//! Written by protocol TVL refresh. `GET /api/v1/pairs/{addr}` reads this stamp only —
//! never `pair_reserves` × oracles × hub on the request path.

use bigdecimal::BigDecimal;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone)]
pub struct PairLiquidityStamp {
    pub pair_id: i32,
    pub liquidity_usd: BigDecimal,
}

/// Replace the full stamp set so unpriced pairs drop out (no `$0` leftover).
pub async fn replace_pair_liquidity_usd(
    pool: &PgPool,
    rows: &[PairLiquidityStamp],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pair_liquidity_usd")
        .execute(&mut *tx)
        .await?;
    for row in rows {
        sqlx::query(
            r#"INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at)
               VALUES ($1, $2, NOW())"#,
        )
        .bind(row.pair_id)
        .bind(&row.liquidity_usd)
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
    #[derive(FromRow)]
    struct Row {
        liquidity_usd: BigDecimal,
    }
    let row =
        sqlx::query_as::<_, Row>("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
            .bind(pair_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.liquidity_usd))
}
