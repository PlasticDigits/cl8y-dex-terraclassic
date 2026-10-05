use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::oracle::{validate_price, OracleTicker, ORACLE_MAX_AGE_SECS, ORACLE_QUORUM};

#[derive(Debug, Clone, FromRow)]
pub struct OracleStateRow {
    pub ticker: String,
    pub price_usd: Option<BigDecimal>,
    pub quality: String,
    pub accepted_sources: Vec<String>,
    pub as_of: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

pub fn valid_decimal_price(price: &BigDecimal) -> bool {
    let minimum = BigDecimal::from(1) / BigDecimal::from(1_000_000_000_000_i64);
    price >= &minimum
        && price <= &BigDecimal::from(1_000_000)
        && price
            .to_string()
            .parse::<f64>()
            .is_ok_and(|p| validate_price(p).is_ok())
}

impl OracleStateRow {
    pub fn is_fresh_at(&self, now: DateTime<Utc>) -> bool {
        self.quality == "fresh"
            && OracleTicker::parse(&self.ticker).is_some()
            && (self.ticker != "vfdusd" || !self.accepted_sources.iter().any(|s| s == "kucoin"))
            && self.accepted_sources.len() >= ORACLE_QUORUM
            && self
                .accepted_sources
                .iter()
                .all(|s| matches!(s.as_str(), "mexc" | "kucoin" | "coingecko"))
            && {
                let mut names = self.accepted_sources.clone();
                names.sort();
                names.dedup();
                names.len() == self.accepted_sources.len()
            }
            && self.price_usd.as_ref().is_some_and(valid_decimal_price)
            && self.as_of.is_some_and(|time| {
                time <= now && now - time <= chrono::Duration::seconds(ORACLE_MAX_AGE_SECS)
            })
    }
}

pub async fn get_oracle_state(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<Option<OracleStateRow>, sqlx::Error> {
    sqlx::query_as::<_, OracleStateRow>("SELECT ticker, price_usd, quality, accepted_sources, as_of, observed_at FROM oracle_state WHERE ticker=$1")
        .bind(ticker.as_str()).fetch_optional(pool).await
}

/// A failed/no-quorum observation replaces prior health immediately. Price and
/// certification are persisted atomically; legacy averages are never certified.
pub async fn record_observation(
    pool: &PgPool,
    ticker: OracleTicker,
    price: Option<&BigDecimal>,
    sources: &[String],
    quality: &str,
) -> Result<DateTime<Utc>, sqlx::Error> {
    record_observation_at(pool, ticker, price, sources, quality, Utc::now()).await
}

pub async fn record_observation_at(
    pool: &PgPool,
    ticker: OracleTicker,
    price: Option<&BigDecimal>,
    sources: &[String],
    quality: &str,
    as_of: DateTime<Utc>,
) -> Result<DateTime<Utc>, sqlx::Error> {
    let now = Utc::now();
    let candidate = OracleStateRow {
        ticker: ticker.as_str().into(),
        price_usd: price.cloned(),
        quality: quality.into(),
        accepted_sources: sources.to_vec(),
        as_of: Some(as_of),
        observed_at: now,
    };
    if quality == "fresh" && !candidate.is_fresh_at(now)
        || quality != "fresh"
            && (price.is_some()
                || !sources.is_empty()
                || !matches!(quality, "degraded" | "unavailable"))
    {
        return Err(sqlx::Error::Protocol("invalid oracle observation".into()));
    }
    let mut tx = pool.begin().await?;
    if let Some(price) = price {
        sqlx::query("INSERT INTO oracle_prices(ticker,price_usd,source,fetched_at,accepted_sources) VALUES($1,$2,'average',$3,$4)")
            .bind(ticker.as_str()).bind(price).bind(as_of).bind(sources).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(ticker) DO UPDATE SET price_usd=EXCLUDED.price_usd,quality=EXCLUDED.quality,accepted_sources=EXCLUDED.accepted_sources,as_of=EXCLUDED.as_of,observed_at=EXCLUDED.observed_at")
        .bind(ticker.as_str()).bind(price).bind(quality).bind(sources).bind(price.map(|_| as_of)).bind(now).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(as_of)
}

/// Fail closed when a compound publication fails but a direct state write works.
pub async fn invalidate_observation(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE oracle_state SET price_usd=NULL,quality='unavailable',accepted_sources='{}',as_of=NULL,observed_at=NOW() WHERE ticker=$1")
        .bind(ticker.as_str()).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow)]
pub struct OraclePriceRow {
    pub id: i64,
    pub ticker: String,
    pub price_usd: BigDecimal,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
    pub accepted_sources: Vec<String>,
}

pub async fn insert_price(
    pool: &PgPool,
    ticker: OracleTicker,
    price_usd: &BigDecimal,
    source: &str,
) -> Result<(), sqlx::Error> {
    insert_price_at(pool, ticker, price_usd, source, Utc::now()).await
}

pub async fn insert_price_at(
    pool: &PgPool,
    ticker: OracleTicker,
    price_usd: &BigDecimal,
    source: &str,
    as_of: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    if !valid_decimal_price(price_usd)
        || !matches!(source, "kucoin" | "mexc" | "coingecko")
        || (ticker == OracleTicker::Vfdusd && source == "kucoin")
        || as_of > now
        || now - as_of > chrono::Duration::seconds(ORACLE_MAX_AGE_SECS)
    {
        return Err(sqlx::Error::Protocol(
            "invalid oracle source observation".into(),
        ));
    }
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(ticker.as_str())
    .bind(price_usd)
    .bind(source)
    .bind(as_of)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_latest_average_price(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<Option<BigDecimal>, sqlx::Error> {
    Ok(get_oracle_state(pool, ticker)
        .await?
        .filter(|state| state.is_fresh_at(Utc::now()))
        .and_then(|state| state.price_usd))
}

pub async fn get_latest_prices_by_source(
    pool: &PgPool,
    ticker: OracleTicker,
) -> Result<Vec<OraclePriceRow>, sqlx::Error> {
    sqlx::query_as::<_, OraclePriceRow>(
        "SELECT DISTINCT ON (source) id, ticker, price_usd, source, fetched_at, accepted_sources
         FROM oracle_prices
         WHERE ticker = $1 AND price_usd >= 0.000000000001 AND price_usd <= 1000000
           AND fetched_at <= NOW() AND fetched_at >= NOW() - INTERVAL '300 seconds'
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
        "SELECT id, ticker, price_usd, source, fetched_at, accepted_sources
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
