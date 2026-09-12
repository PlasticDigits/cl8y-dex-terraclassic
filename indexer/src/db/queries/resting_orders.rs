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
/// `orders` must already be in on-chain DLL walk order (head → tail, bids then asks as provided).
/// `walk_index` is the insert position so equal-price FIFO after reprice is not rebuilt from
/// `order_id` (#1227).
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
    for (walk_index, o) in orders.iter().enumerate() {
        sqlx::query(
            "INSERT INTO resting_limit_orders
                (pair_id, order_id, side, price, remaining, owner, expires_at, block_height, snapshot_at, walk_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), $9)",
        )
        .bind(pair_id)
        .bind(o.order_id)
        .bind(&o.side)
        .bind(&o.price)
        .bind(&o.remaining)
        .bind(&o.owner)
        .bind(o.expires_at)
        .bind(block_height)
        .bind(walk_index as i32)
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

/// A pair's resting book for one side, in on-chain DLL walk order (snapshot `walk_index`).
/// New placements still happen to be ascending `order_id` at equal price; after
/// `UpdateLimitOrderPrice` the preserved id may be older than neighbors (#1227).
/// `side` is `"bid"` or `"ask"` (a controlled value — no caller input reaches the SQL).
pub async fn get_pair_resting_book<'e, E>(
    executor: E,
    pair_id: i32,
    side: &str,
) -> Result<Vec<RestingOrderRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, RestingOrderRow>(
        "SELECT pair_id, order_id, side, price, remaining, owner, expires_at
         FROM resting_limit_orders
         WHERE pair_id = $1 AND side = $2
         ORDER BY walk_index ASC",
    )
    .bind(pair_id)
    .bind(side)
    .fetch_all(executor)
    .await
}
