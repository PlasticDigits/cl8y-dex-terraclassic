use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct LiquidityEventRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub provider: String,
    pub event_type: String,
    pub asset_0_amount: BigDecimal,
    pub asset_1_amount: BigDecimal,
    pub lp_amount: BigDecimal,
    pub created_at: DateTime<Utc>,
}

pub async fn liquidity_event_exists(
    pool: &PgPool,
    tx_hash: &str,
    pair_id: i32,
    event_type: &str,
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM liquidity_events WHERE tx_hash = $1 AND pair_id = $2 AND event_type = $3",
    )
    .bind(tx_hash)
    .bind(pair_id)
    .bind(event_type)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_liquidity_event(
    pool: &PgPool,
    pair_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    provider: &str,
    event_type: &str,
    asset_0_amount: &BigDecimal,
    asset_1_amount: &BigDecimal,
    lp_amount: &BigDecimal,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO liquidity_events
         (pair_id, block_height, block_timestamp, tx_hash, provider, event_type,
          asset_0_amount, asset_1_amount, lp_amount)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(provider)
    .bind(event_type)
    .bind(asset_0_amount)
    .bind(asset_1_amount)
    .bind(lp_amount)
    .fetch_one(pool)
    .await
}

pub async fn list_liquidity_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<LiquidityEventRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, LiquidityEventRow>(
                "SELECT * FROM liquidity_events WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, LiquidityEventRow>(
                "SELECT * FROM liquidity_events WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
