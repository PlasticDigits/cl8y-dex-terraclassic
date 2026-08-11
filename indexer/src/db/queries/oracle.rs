use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::oracle::OracleTicker;

#[derive(Debug, Clone, FromRow)]
pub struct OraclePriceRow {
    pub id: i64,
    pub ticker: String,
    pub price_usd: BigDecimal,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
}

pub async fn insert_price(
    pool: &PgPool,
    ticker: OracleTicker,
    price_usd: &BigDecimal,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ($1, $2, $3, NOW())",
    )
    .bind(ticker.as_str())
    .bind(price_usd)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_latest_average_price(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<Option<BigDecimal>, sqlx::Error> {
    sqlx::query_scalar::<_, BigDecimal>(
        "SELECT price_usd FROM oracle_prices
         WHERE ticker = $1 AND source = 'average'
         ORDER BY fetched_at DESC LIMIT 1",
    )
    .bind(ticker.as_str())
    .fetch_optional(pool)
    .await
}

pub async fn get_latest_prices_by_source(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<Vec<OraclePriceRow>, sqlx::Error> {
    sqlx::query_as::<_, OraclePriceRow>(
        "SELECT DISTINCT ON (source) id, ticker, price_usd, source, fetched_at
         FROM oracle_prices
         WHERE ticker = $1
         ORDER BY source, fetched_at DESC",
    )
    .bind(ticker.as_str())
    .fetch_all(pool)
    .await
}

pub async fn get_price_history(
    pool: &PgPool,
    ticker: OracleTicker,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<OraclePriceRow>, sqlx::Error> {
    sqlx::query_as::<_, OraclePriceRow>(
        "SELECT id, ticker, price_usd, source, fetched_at
         FROM oracle_prices
         WHERE ticker = $1 AND source = 'average'
           AND fetched_at >= $2 AND fetched_at <= $3
         ORDER BY fetched_at DESC LIMIT $4",
    )
    .bind(ticker.as_str())
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
}
