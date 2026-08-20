//! Persist last-good Venus vFDUSD redeem samples (GitLab #571).
//! Never write these rows into `oracle_prices` — the unit is FDUSD, not USD.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct VenusVfdusdRateRow {
    pub id: i64,
    pub fdusd_per_vfdusd: BigDecimal,
    pub vtoken: String,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
}

pub async fn insert_rate(
    pool: &PgPool,
    fdusd_per_vfdusd: &BigDecimal,
    vtoken: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO venus_vfdusd_rates (fdusd_per_vfdusd, vtoken, source, fetched_at)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(fdusd_per_vfdusd)
    .bind(vtoken)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_latest_rate(pool: &PgPool) -> Result<Option<VenusVfdusdRateRow>, sqlx::Error> {
    sqlx::query_as::<_, VenusVfdusdRateRow>(
        "SELECT id, fdusd_per_vfdusd, vtoken, source, fetched_at
         FROM venus_vfdusd_rates
         ORDER BY fetched_at DESC
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}
