//! UTC Protocol liquidity (stock) grain tables (GitLab #689).
//!
//! Last `global_liquidity_snapshots` sample in each UTC hour / day / month — not SUM,
//! not average. GET reads these tables only. Downsample **before** the 35d snapshot prune
//! so Monthly can retain ≥ 24 months.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Months, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};

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
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolHourlyLiquidityRow {
    pub utc_hour: DateTime<Utc>,
    pub liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolMonthlyLiquidityRow {
    pub utc_month: NaiveDate,
    pub liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub refreshed_at: DateTime<Utc>,
}

pub async fn get_daily_rows(
    pool: &PgPool,
    from_day: NaiveDate,
    to_day: NaiveDate,
) -> Result<Vec<ProtocolDailyLiquidityRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolDailyLiquidityRow>(
        r#"SELECT utc_day, liquidity_usd, priced_pair_count, refreshed_at
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
        r#"SELECT utc_hour, liquidity_usd, priced_pair_count, refreshed_at
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
        r#"SELECT utc_month, liquidity_usd, priced_pair_count, refreshed_at
           FROM protocol_monthly_liquidity
           WHERE utc_month >= $1 AND utc_month <= $2
           ORDER BY utc_month ASC"#,
    )
    .bind(from_month)
    .bind(to_month)
    .fetch_all(pool)
    .await
}

/// Last snapshot per UTC hour over the hourly lookback; prune older than ~10d.
pub async fn refresh_protocol_hourly(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now_hour = utc_hour_start(Utc::now());
    let from = now_hour - Duration::hours(HOURLY_LOOKBACK_HOURS);
    let to_exclusive = now_hour + Duration::hours(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_hourly_liquidity
            (utc_hour, liquidity_usd, priced_pair_count, refreshed_at)
        SELECT
            h.utc_hour,
            a.liquidity_usd,
            COALESCE(a.priced_pair_count, 0),
            NOW()
        FROM generate_series($1::timestamptz, $2::timestamptz - interval '1 hour', interval '1 hour')
            AS h(utc_hour)
        LEFT JOIN (
            SELECT DISTINCT ON (
                date_trunc('hour', sampled_at AT TIME ZONE 'UTC')
            )
                (date_trunc('hour', sampled_at AT TIME ZONE 'UTC') AT TIME ZONE 'UTC') AS utc_hour,
                total_liquidity_usd AS liquidity_usd,
                priced_pair_count
            FROM global_liquidity_snapshots
            WHERE sampled_at >= $1 AND sampled_at < $2
            ORDER BY date_trunc('hour', sampled_at AT TIME ZONE 'UTC'), sampled_at DESC
        ) a ON a.utc_hour = h.utc_hour
        ON CONFLICT (utc_hour) DO UPDATE SET
            liquidity_usd = EXCLUDED.liquidity_usd,
            priced_pair_count = EXCLUDED.priced_pair_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = now_hour - Duration::hours(HOURLY_PRUNE_AFTER_HOURS);
    sqlx::query("DELETE FROM protocol_hourly_liquidity WHERE utc_hour < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

/// Last snapshot per UTC day over snapshot retention (~35d). Does **not** generate_series
/// 90 idle days (that would NULL-out older daily rows after snapshots prune).
pub async fn refresh_protocol_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = utc_day_start(Utc::now());
    let from = today - SNAPSHOT_RETENTION;
    let to_exclusive = today + Duration::days(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_daily_liquidity
            (utc_day, liquidity_usd, priced_pair_count, refreshed_at)
        SELECT
            d.utc_day,
            a.liquidity_usd,
            COALESCE(a.priced_pair_count, 0),
            NOW()
        FROM generate_series(($1::timestamptz AT TIME ZONE 'UTC')::date,
                             ($2::timestamptz AT TIME ZONE 'UTC')::date - 1,
                             interval '1 day') AS d(utc_day)
        LEFT JOIN (
            SELECT DISTINCT ON ((sampled_at AT TIME ZONE 'UTC')::date)
                (sampled_at AT TIME ZONE 'UTC')::date AS utc_day,
                total_liquidity_usd AS liquidity_usd,
                priced_pair_count
            FROM global_liquidity_snapshots
            WHERE sampled_at >= $1 AND sampled_at < $2
            ORDER BY (sampled_at AT TIME ZONE 'UTC')::date, sampled_at DESC
        ) a ON a.utc_day = d.utc_day
        ON CONFLICT (utc_day) DO UPDATE SET
            liquidity_usd = EXCLUDED.liquidity_usd,
            priced_pair_count = EXCLUDED.priced_pair_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = (today - Duration::days(PRUNE_AFTER_DAYS)).date_naive();
    sqlx::query("DELETE FROM protocol_daily_liquidity WHERE utc_day < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

/// Last snapshot per UTC calendar month among snapshots still in the table.
/// Does not generate empty months (would wipe history after 35d snapshot prune).
pub async fn refresh_protocol_monthly(pool: &PgPool) -> Result<(), sqlx::Error> {
    let this_month = utc_month_start(Utc::now());
    let from = Utc::now() - SNAPSHOT_RETENTION;
    let to_exclusive = Utc::now() + Duration::hours(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_monthly_liquidity
            (utc_month, liquidity_usd, priced_pair_count, refreshed_at)
        SELECT
            (date_trunc('month', sampled_at AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')::date AS utc_month,
            (array_agg(total_liquidity_usd ORDER BY sampled_at DESC))[1],
            (array_agg(priced_pair_count ORDER BY sampled_at DESC))[1],
            NOW()
        FROM global_liquidity_snapshots
        WHERE sampled_at >= $1 AND sampled_at < $2
        GROUP BY 1
        ON CONFLICT (utc_month) DO UPDATE SET
            liquidity_usd = EXCLUDED.liquidity_usd,
            priced_pair_count = EXCLUDED.priced_pair_count,
            refreshed_at = EXCLUDED.refreshed_at
        -- Do not NULL/zero an existing month when this refresh saw no snapshot
        -- (35d snapshot prune must not wipe Monthly stock).
        WHERE EXCLUDED.liquidity_usd IS NOT NULL
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = this_month
        .checked_sub_months(Months::new(MONTHLY_PRUNE_AFTER_MONTHS))
        .expect("month prune in range");
    sqlx::query("DELETE FROM protocol_monthly_liquidity WHERE utc_month < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

/// Hourly + daily + monthly stock rollups. Call **before** snapshot prune.
pub async fn refresh_protocol_liquidity_rollups(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_protocol_hourly(pool).await?;
    refresh_protocol_daily(pool).await?;
    refresh_protocol_monthly(pool).await?;
    Ok(())
}
