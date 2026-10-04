//! Materialized per-pair factory v2 AMM TVL (GitLab #664 / #655).
//!
//! Written by protocol TVL refresh. `GET /api/v1/pairs/{addr}` reads this stamp only —
//! never `pair_reserves` × oracles × hub on the request path.

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use sqlx::{FromRow, PgPool, Postgres, Transaction};

#[derive(Debug, Clone)]
pub struct PairLiquidityStamp {
    pub pair_id: i32,
    pub liquidity_usd: BigDecimal,
}

/// Replace stamps inside the global publication transaction; omitted pairs disappear.
pub async fn replace_pair_liquidity_usd_tx(
    tx: &mut Transaction<'_, Postgres>,
    rows: &[PairLiquidityStamp],
    generation: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pair_liquidity_usd")
        .execute(&mut **tx)
        .await?;
    for row in rows {
        sqlx::query(
            r#"INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at, generation)
               VALUES ($1, $2, NOW(), $3)"#,
        )
        .bind(row.pair_id)
        .bind(&row.liquidity_usd)
        .bind(generation)
        .execute(&mut **tx)
        .await?;
    }
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

/// Set-based stamp map for CG/CMC list paths (GitLab #685). GET never recomputes TVL.
pub async fn get_all_pair_liquidity_usd(
    pool: &PgPool,
) -> Result<HashMap<i32, BigDecimal>, sqlx::Error> {
    #[derive(FromRow)]
    struct Row {
        pair_id: i32,
        liquidity_usd: BigDecimal,
    }
    let rows = sqlx::query_as::<_, Row>("SELECT pair_id, liquidity_usd FROM pair_liquidity_usd")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.pair_id, r.liquidity_usd))
        .collect())
}
