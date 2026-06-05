//! Materialized current-state limit book (GitLab #279 Phase 1a). Distinct from the append-only
//! lifecycle logs: this holds the live `remaining` per resting order so the 0-LCD solver can walk
//! the book straight from Postgres. The snapshot loop replaces a pair's rows wholesale.

use bigdecimal::BigDecimal;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct RestingOrderRow {
    pub pair_id: i32,
    pub order_id: i64,
    pub side: String,
    pub price: BigDecimal,
    pub remaining: BigDecimal,
    pub owner: Option<String>,
    pub expires_at: Option<i64>,
}

/// One resting order in a snapshot. `side` must be `"bid"` or `"ask"`.
#[derive(Debug, Clone)]
pub struct RestingOrderInput {
    pub order_id: i64,
    pub side: String,
    pub price: BigDecimal,
    pub remaining: BigDecimal,
    pub owner: Option<String>,
    pub expires_at: Option<i64>,
}

/// Replace a pair's entire materialized resting book with `orders` (caller owns the transaction).
pub async fn replace_pair_resting_orders_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pair_id: i32,
    block_height: Option<i64>,
    orders: &[RestingOrderInput],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM resting_limit_orders WHERE pair_id = $1")
        .bind(pair_id)
        .execute(&mut **tx)
        .await?;
    for o in orders {
        sqlx::query(
            "INSERT INTO resting_limit_orders
                (pair_id, order_id, side, price, remaining, owner, expires_at, block_height, snapshot_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())",
        )
        .bind(pair_id)
        .bind(o.order_id)
        .bind(&o.side)
        .bind(&o.price)
        .bind(&o.remaining)
        .bind(&o.owner)
        .bind(o.expires_at)
        .bind(block_height)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Replace a pair's entire materialized resting book with `orders`, atomically (the snapshot loop
/// produces the full current book per pair, so the table always reflects one consistent snapshot).
pub async fn replace_pair_resting_orders(
    pool: &PgPool,
    pair_id: i32,
    block_height: Option<i64>,
    orders: &[RestingOrderInput],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    replace_pair_resting_orders_in_tx(&mut tx, pair_id, block_height, orders).await?;
    tx.commit().await?;
    Ok(())
}

/// A pair's resting book for one side, in walk order: best price first (bids DESC, asks ASC), then
/// FIFO by `order_id`. `side` is `"bid"` or `"ask"` (a controlled value — only the sort direction
/// is interpolated, no caller input reaches the SQL).
pub async fn get_pair_resting_book(
    pool: &PgPool,
    pair_id: i32,
    side: &str,
) -> Result<Vec<RestingOrderRow>, sqlx::Error> {
    let price_dir = if side == "bid" { "DESC" } else { "ASC" };
    let sql = format!(
        "SELECT pair_id, order_id, side, price, remaining, owner, expires_at
         FROM resting_limit_orders
         WHERE pair_id = $1 AND side = $2
         ORDER BY price {price_dir}, order_id ASC"
    );
    sqlx::query_as::<_, RestingOrderRow>(&sql)
        .bind(pair_id)
        .bind(side)
        .fetch_all(pool)
        .await
}
