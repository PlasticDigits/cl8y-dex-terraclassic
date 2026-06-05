use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct LimitOrderFillRow {
    pub id: i64,
    pub pair_id: i32,
    pub swap_event_id: Option<i64>,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub order_id: i64,
    pub side: String,
    pub maker: String,
    pub price: BigDecimal,
    pub token0_amount: BigDecimal,
    pub token1_amount: BigDecimal,
    pub commission_amount: BigDecimal,
}

pub async fn fill_exists(
    pool: &PgPool,
    tx_hash: &str,
    pair_id: i32,
    order_id: i64,
) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM limit_order_fills WHERE tx_hash = $1 AND pair_id = $2 AND order_id = $3",
    )
    .bind(tx_hash)
    .bind(pair_id)
    .bind(order_id)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

/// Resolve the `swap_events.id` for a maker fill by its per-pair swap ordinal within the tx
/// (GitLab #316). Replaces the old `ORDER BY id ASC LIMIT 1` (always the FIRST swap), which
/// mis-linked fills when a tx had multiple swaps on the same pair. `swap_index` is the deterministic
/// parser walk ordinal carried on the fill; `(tx_hash, pair_id, swap_index)` is unique (GitLab
/// #287), so this returns the exact swap row that produced the fill, or `None` if absent.
pub async fn swap_id_for_tx_pair_index(
    pool: &PgPool,
    tx_hash: &str,
    pair_id: i32,
    swap_index: i32,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT id FROM swap_events WHERE tx_hash = $1 AND pair_id = $2 AND swap_index = $3",
    )
    .bind(tx_hash)
    .bind(pair_id)
    .bind(swap_index)
    .fetch_optional(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_fill(
    pool: &PgPool,
    pair_id: i32,
    swap_event_id: Option<i64>,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    order_id: i64,
    side: &str,
    maker: &str,
    price: &BigDecimal,
    token0_amount: &BigDecimal,
    token1_amount: &BigDecimal,
    commission_amount: &BigDecimal,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash,
          order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         RETURNING id",
    )
    .bind(pair_id)
    .bind(swap_event_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(order_id)
    .bind(side)
    .bind(maker)
    .bind(price)
    .bind(token0_amount)
    .bind(token1_amount)
    .bind(commission_amount)
    .fetch_one(pool)
    .await
}

pub async fn list_fills_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<LimitOrderFillRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn list_fills_for_order(
    pool: &PgPool,
    pair_id: i32,
    order_id: i64,
    limit: i64,
) -> Result<Vec<LimitOrderFillRow>, sqlx::Error> {
    sqlx::query_as::<_, LimitOrderFillRow>(
        "SELECT * FROM limit_order_fills WHERE pair_id = $1 AND order_id = $2
         ORDER BY id DESC LIMIT $3",
    )
    .bind(pair_id)
    .bind(order_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_fills_for_maker(
    pool: &PgPool,
    maker: &str,
    pair_id: Option<i32>,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<LimitOrderFillRow>, sqlx::Error> {
    match (before_id, pair_id) {
        (Some(bid), Some(pid)) => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills
                 WHERE maker = $1 AND pair_id = $4 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(maker)
            .bind(limit)
            .bind(bid)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (None, Some(pid)) => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills
                 WHERE maker = $1 AND pair_id = $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(maker)
            .bind(limit)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (Some(bid), None) => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills
                 WHERE maker = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(maker)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, LimitOrderFillRow>(
                "SELECT * FROM limit_order_fills
                 WHERE maker = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(maker)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
