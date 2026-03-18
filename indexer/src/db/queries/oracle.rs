use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct UstcPriceRow {
    pub id: i64,
    pub price_usd: BigDecimal,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
}

pub async fn insert_price(
    pool: &PgPool,
    price_usd: &BigDecimal,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ustc_prices (price_usd, source, fetched_at) VALUES ($1, $2, NOW())",
    )
    .bind(price_usd)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_latest_average_price(
    pool: &PgPool,
) -> Result<Option<BigDecimal>, sqlx::Error> {
    sqlx::query_scalar::<_, BigDecimal>(
        "SELECT price_usd FROM ustc_prices WHERE source = 'average' ORDER BY fetched_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_latest_prices_by_source(
    pool: &PgPool,
) -> Result<Vec<UstcPriceRow>, sqlx::Error> {
    sqlx::query_as::<_, UstcPriceRow>(
        "SELECT DISTINCT ON (source) * FROM ustc_prices ORDER BY source, fetched_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_price_history(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<UstcPriceRow>, sqlx::Error> {
    sqlx::query_as::<_, UstcPriceRow>(
        "SELECT * FROM ustc_prices
         WHERE source = 'average' AND fetched_at >= $1 AND fetched_at <= $2
         ORDER BY fetched_at DESC LIMIT $3",
    )
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
}
