//! Listing-only wrap principal and backing; deliberately independent of economic tables.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PrincipalEvent {
    pub id: i64,
    pub mapper_address: String,
    pub native_denom: String,
    pub direction: String,
    pub principal_raw: BigDecimal,
    pub custody_raw: BigDecimal,
    pub tx_hash: String,
    pub trader_address: String,
    pub source_ordinal: i64,
    pub txn_index: i64,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub block_hash: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct CustodySnapshot {
    pub mapper_address: String,
    pub native_denom: String,
    pub custody_raw: BigDecimal,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub block_hash: String,
}

pub async fn latest_custody(
    pool: &PgPool,
    mapper: &str,
    denom: &str,
) -> Result<Option<CustodySnapshot>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM listing_mapper_custody WHERE mapper_address = $1 AND native_denom = $2 ORDER BY block_height DESC LIMIT 1")
        .bind(mapper).bind(denom).fetch_optional(pool).await
}

pub async fn get_principal_events(
    pool: &PgPool,
    mapper: &str,
    from_height: Option<i64>,
    to_height: Option<i64>,
) -> Result<Vec<PrincipalEvent>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM listing_wrap_principal WHERE mapper_address = $1 AND ($2::BIGINT IS NULL OR block_height >= $2) AND ($3::BIGINT IS NULL OR block_height <= $3) ORDER BY block_height, txn_index, source_ordinal")
        .bind(mapper).bind(from_height).bind(to_height).fetch_all(pool).await
}
