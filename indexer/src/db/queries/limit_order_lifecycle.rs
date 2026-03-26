use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PlacementRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub order_id: i64,
    pub owner: Option<String>,
    pub side: Option<String>,
    pub price: Option<BigDecimal>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CancellationRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub order_id: i64,
    pub owner: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_placement(
    pool: &PgPool,
    pair_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    order_id: i64,
    owner: Option<&str>,
    side: Option<&str>,
    price: Option<&BigDecimal>,
    expires_at: Option<i64>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner, side, price, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (tx_hash, pair_id, order_id) DO NOTHING
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(order_id)
    .bind(owner)
    .bind(side)
    .bind(price)
    .bind(expires_at)
    .fetch_optional(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_cancellation(
    pool: &PgPool,
    pair_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    order_id: i64,
    owner: Option<&str>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO limit_order_cancellations
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (tx_hash, pair_id, order_id) DO NOTHING
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(order_id)
    .bind(owner)
    .fetch_optional(pool)
    .await
}

pub async fn list_placements_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<PlacementRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, PlacementRow>(
                "SELECT * FROM limit_order_placements WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, PlacementRow>(
                "SELECT * FROM limit_order_placements WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn list_cancellations_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<CancellationRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
