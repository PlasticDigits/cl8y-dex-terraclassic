//! UTC Protocol liquidity (stock) grain tables (GitLab #689).
//!
//! Last `global_liquidity_snapshots` sample in each UTC hour / day / month — not SUM,
//! not average. GET reads these tables only. Downsample **before** the 35d snapshot prune
//! so Monthly can retain ≥ 24 months.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Months, NaiveDate, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

use crate::db::queries::liquidity_snapshots::SNAPSHOT_RETENTION;
use crate::db::queries::protocol_volume::{
    utc_hour_start, utc_month_start, HOURLY_LOOKBACK_HOURS, HOURLY_PRUNE_AFTER_HOURS,
    MONTHLY_PRUNE_AFTER_MONTHS, PRUNE_AFTER_DAYS,
};
use crate::indexer::defillama::utc_day_start;

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolDailyLiquidityRow {
    pub utc_day: NaiveDate,
    pub liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub refreshed_at: DateTime<Utc>,
    pub quality: String,
    pub as_of: Option<DateTime<Utc>>,
    pub coverage_identity: String,
    pub generation: i64,
    pub reasons: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolHourlyLiquidityRow {
    pub utc_hour: DateTime<Utc>,
    pub liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub refreshed_at: DateTime<Utc>,
    pub quality: String,
    pub as_of: Option<DateTime<Utc>>,
    pub coverage_identity: String,
    pub generation: i64,
    pub reasons: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolMonthlyLiquidityRow {
    pub utc_month: NaiveDate,
    pub liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub refreshed_at: DateTime<Utc>,
    pub quality: String,
    pub as_of: Option<DateTime<Utc>>,
    pub coverage_identity: String,
    pub generation: i64,
    pub reasons: serde_json::Value,
}

pub async fn get_daily_rows(
    pool: &PgPool,
    from_day: NaiveDate,
    to_day: NaiveDate,
) -> Result<Vec<ProtocolDailyLiquidityRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolDailyLiquidityRow>(
        r#"SELECT utc_day, liquidity_usd, priced_pair_count, refreshed_at, quality, as_of, coverage_identity, generation, reasons
           FROM protocol_daily_liquidity
           WHERE utc_day >= $1 AND utc_day <= $2
           ORDER BY utc_day ASC"#,
    )
    .bind(from_day)
    .bind(to_day)
    .fetch_all(pool)
    .await
}

pub async fn get_hourly_rows(
    pool: &PgPool,
    from_hour: DateTime<Utc>,
    to_hour: DateTime<Utc>,
) -> Result<Vec<ProtocolHourlyLiquidityRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolHourlyLiquidityRow>(
        r#"SELECT utc_hour, liquidity_usd, priced_pair_count, refreshed_at, quality, as_of, coverage_identity, generation, reasons
           FROM protocol_hourly_liquidity
           WHERE utc_hour >= $1 AND utc_hour <= $2
           ORDER BY utc_hour ASC"#,
    )
    .bind(from_hour)
    .bind(to_hour)
    .fetch_all(pool)
    .await
}

pub async fn get_monthly_rows(
    pool: &PgPool,
    from_month: NaiveDate,
    to_month: NaiveDate,
) -> Result<Vec<ProtocolMonthlyLiquidityRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolMonthlyLiquidityRow>(
        r#"SELECT utc_month, liquidity_usd, priced_pair_count, refreshed_at, quality, as_of, coverage_identity, generation, reasons
           FROM protocol_monthly_liquidity
           WHERE utc_month >= $1 AND utc_month <= $2
           ORDER BY utc_month ASC"#,
    )
    .bind(from_month)
    .bind(to_month)
    .fetch_all(pool)
    .await
}

/// Trusted constants only; never accepts request-controlled SQL identifiers.
async fn refresh_grain_tx(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
    grain: &str,
) -> Result<(), sqlx::Error> {
    let (table, key, from, to, prune) = match grain {
        "hour" => {
            let start = utc_hour_start(now);
            (
                "protocol_hourly_liquidity",
                "utc_hour",
                start - Duration::hours(HOURLY_LOOKBACK_HOURS),
                start + Duration::hours(1),
                start - Duration::hours(HOURLY_PRUNE_AFTER_HOURS),
            )
        }
        "day" => {
            let start = utc_day_start(now);
            (
                "protocol_daily_liquidity",
                "utc_day",
                start - SNAPSHOT_RETENTION,
                start + Duration::days(1),
                start - Duration::days(PRUNE_AFTER_DAYS),
            )
        }
        "month" => {
            let prune = utc_month_start(now)
                .checked_sub_months(Months::new(MONTHLY_PRUNE_AFTER_MONTHS))
                .expect("month prune in range")
                .and_hms_opt(0, 0, 0)
                .expect("midnight")
                .and_utc();
            (
                "protocol_monthly_liquidity",
                "utc_month",
                now - SNAPSHOT_RETENTION,
                now + Duration::hours(1),
                prune,
            )
        }
        _ => {
            return Err(sqlx::Error::Protocol(
                "invalid internal liquidity grain".into(),
            ));
        }
    };
    let bucket = if grain == "hour" {
        format!("date_trunc('{grain}',sampled_at AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'")
    } else {
        format!("date_trunc('{grain}',sampled_at AT TIME ZONE 'UTC')::date")
    };
    // DISTINCT ON selects the terminal observation even when its total is NULL.
    // Monthly must update degraded terminal rows; a NULL sample is not no sample.
    let query = format!(
        "INSERT INTO {table}({key},liquidity_usd,priced_pair_count,refreshed_at,
        quality,as_of,coverage_identity,generation,reasons)
        SELECT DISTINCT ON ({bucket}) {bucket},total_liquidity_usd,priced_pair_count,NOW(),
        quality,as_of,coverage_identity,generation,reasons FROM global_liquidity_snapshots
        WHERE sampled_at >= $1 AND sampled_at < $2 ORDER BY {bucket},sampled_at DESC
        ON CONFLICT({key}) DO UPDATE SET liquidity_usd=EXCLUDED.liquidity_usd,
        priced_pair_count=EXCLUDED.priced_pair_count,refreshed_at=EXCLUDED.refreshed_at,
        quality=EXCLUDED.quality,as_of=EXCLUDED.as_of,coverage_identity=EXCLUDED.coverage_identity,
        generation=EXCLUDED.generation,reasons=EXCLUDED.reasons"
    );
    sqlx::query(&query)
        .bind(from)
        .bind(to)
        .execute(&mut **tx)
        .await?;
    let query = format!("DELETE FROM {table} WHERE {key} < $1");
    if grain == "hour" {
        sqlx::query(&query).bind(prune).execute(&mut **tx).await?;
    } else {
        sqlx::query(&query)
            .bind(prune.date_naive())
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
async fn refresh_grain(pool: &PgPool, grain: &str) -> Result<(), sqlx::Error> {
    let mut tx = super::liquidity_snapshots::begin_liquidity_refresh(pool).await?;
    refresh_grain_tx(&mut tx, Utc::now(), grain).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn refresh_protocol_hourly(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_grain(pool, "hour").await
}
pub async fn refresh_protocol_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_grain(pool, "day").await
}
pub async fn refresh_protocol_monthly(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_grain(pool, "month").await
}
/// Refresh all grains before source pruning, including unknown terminal values.
pub async fn refresh_protocol_liquidity_rollups_tx(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for grain in ["hour", "day", "month"] {
        refresh_grain_tx(tx, now, grain).await?;
    }
    Ok(())
}
pub async fn refresh_protocol_liquidity_rollups(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = super::liquidity_snapshots::begin_liquidity_refresh(pool).await?;
    refresh_protocol_liquidity_rollups_tx(&mut tx, Utc::now()).await?;
    tx.commit().await?;
    Ok(())
}
