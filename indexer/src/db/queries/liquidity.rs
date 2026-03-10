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
