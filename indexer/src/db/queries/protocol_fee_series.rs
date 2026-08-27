//! UTC Protocol treasury-fee (flow) grain tables (GitLab #689).
//!
//! Same idle / unpriced rules as volume: no events → `0`; activity + all unpriced → NULL;
//! mixed → SUM priced. GET reads these tables only — never `protocol_fee_events` /
//! `swap_events` / Llama.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Months, NaiveDate, TimeZone, Utc};
use sqlx::{FromRow, PgPool};

use crate::db::queries::protocol_volume::{
    utc_hour_start, utc_month_start, DAILY_LOOKBACK_DAYS, HOURLY_LOOKBACK_HOURS,
    HOURLY_PRUNE_AFTER_HOURS, MONTHLY_LOOKBACK_MONTHS, MONTHLY_PRUNE_AFTER_MONTHS, PRUNE_AFTER_DAYS,
};
use crate::indexer::defillama::utc_day_start;
use crate::indexer::pair_price_usd::fits_numeric_38_18;

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolDailyFeeSeriesRow {
    pub utc_day: NaiveDate,
    pub fees_usd: Option<BigDecimal>,
    pub event_count: i64,
    pub unpriced_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolHourlyFeeSeriesRow {
    pub utc_hour: DateTime<Utc>,
    pub fees_usd: Option<BigDecimal>,
    pub event_count: i64,
    pub unpriced_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolMonthlyFeeSeriesRow {
    pub utc_month: NaiveDate,
    pub fees_usd: Option<BigDecimal>,
    pub event_count: i64,
    pub unpriced_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

fn fees_usd_from_agg(event_count: i64, fees_usd: Option<BigDecimal>) -> Option<BigDecimal> {
    if event_count <= 0 {
        Some(BigDecimal::from(0))
    } else {
        match fees_usd {
            Some(v) if v > BigDecimal::from(0) && fits_numeric_38_18(&v) => Some(v),
            _ => None,
        }
    }
}

pub async fn get_daily_rows(
    pool: &PgPool,
    from_day: NaiveDate,
    to_day: NaiveDate,
) -> Result<Vec<ProtocolDailyFeeSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolDailyFeeSeriesRow>(
        r#"SELECT utc_day, fees_usd, event_count, unpriced_count, refreshed_at
           FROM protocol_daily_fees
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
) -> Result<Vec<ProtocolHourlyFeeSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolHourlyFeeSeriesRow>(
        r#"SELECT utc_hour, fees_usd, event_count, unpriced_count, refreshed_at
           FROM protocol_hourly_fees
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
) -> Result<Vec<ProtocolMonthlyFeeSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolMonthlyFeeSeriesRow>(
        r#"SELECT utc_month, fees_usd, event_count, unpriced_count, refreshed_at
           FROM protocol_monthly_fees
           WHERE utc_month >= $1 AND utc_month <= $2
           ORDER BY utc_month ASC"#,
    )
    .bind(from_month)
    .bind(to_month)
    .fetch_all(pool)
    .await
}

pub async fn refresh_protocol_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = utc_day_start(Utc::now());
    let from = today - Duration::days(DAILY_LOOKBACK_DAYS);
    let to_exclusive = today + Duration::days(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_daily_fees
            (utc_day, fees_usd, event_count, unpriced_count, refreshed_at)
        SELECT
            d.utc_day,
            CASE
                WHEN COALESCE(a.event_count, 0) = 0 THEN 0::numeric
                WHEN a.fees_usd IS NOT NULL AND a.fees_usd > 0 AND a.fees_usd < 1e20
                    THEN a.fees_usd
                ELSE NULL
            END,
            COALESCE(a.event_count, 0),
            COALESCE(a.unpriced_count, 0),
            NOW()
        FROM generate_series(($1::timestamptz AT TIME ZONE 'UTC')::date,
                             ($2::timestamptz AT TIME ZONE 'UTC')::date - 1,
                             interval '1 day') AS d(utc_day)
        LEFT JOIN (
            SELECT
                (block_timestamp AT TIME ZONE 'UTC')::date AS utc_day,
                COUNT(*)::bigint AS event_count,
                COUNT(*) FILTER (
                    WHERE fee_usd IS NULL OR fee_usd <= 0
                )::bigint AS unpriced_count,
                SUM(fee_usd) FILTER (
                    WHERE fee_usd IS NOT NULL AND fee_usd > 0
                ) AS fees_usd
            FROM protocol_fee_events
            WHERE block_timestamp >= $1 AND block_timestamp < $2
            GROUP BY 1
        ) a ON a.utc_day = d.utc_day
        ON CONFLICT (utc_day) DO UPDATE SET
            fees_usd = EXCLUDED.fees_usd,
            event_count = EXCLUDED.event_count,
            unpriced_count = EXCLUDED.unpriced_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = (today - Duration::days(PRUNE_AFTER_DAYS)).date_naive();
    sqlx::query("DELETE FROM protocol_daily_fees WHERE utc_day < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn refresh_protocol_hourly(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now_hour = utc_hour_start(Utc::now());
    let from = now_hour - Duration::hours(HOURLY_LOOKBACK_HOURS);
    let to_exclusive = now_hour + Duration::hours(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_hourly_fees
            (utc_hour, fees_usd, event_count, unpriced_count, refreshed_at)
        SELECT
            h.utc_hour,
            CASE
                WHEN COALESCE(a.event_count, 0) = 0 THEN 0::numeric
                WHEN a.fees_usd IS NOT NULL AND a.fees_usd > 0 AND a.fees_usd < 1e20
                    THEN a.fees_usd
                ELSE NULL
            END,
            COALESCE(a.event_count, 0),
            COALESCE(a.unpriced_count, 0),
            NOW()
        FROM generate_series($1::timestamptz, $2::timestamptz - interval '1 hour', interval '1 hour')
            AS h(utc_hour)
        LEFT JOIN (
            SELECT
                (date_trunc('hour', block_timestamp AT TIME ZONE 'UTC') AT TIME ZONE 'UTC') AS utc_hour,
                COUNT(*)::bigint AS event_count,
                COUNT(*) FILTER (
                    WHERE fee_usd IS NULL OR fee_usd <= 0
                )::bigint AS unpriced_count,
                SUM(fee_usd) FILTER (
                    WHERE fee_usd IS NOT NULL AND fee_usd > 0
                ) AS fees_usd
            FROM protocol_fee_events
            WHERE block_timestamp >= $1 AND block_timestamp < $2
            GROUP BY 1
        ) a ON a.utc_hour = h.utc_hour
        ON CONFLICT (utc_hour) DO UPDATE SET
            fees_usd = EXCLUDED.fees_usd,
            event_count = EXCLUDED.event_count,
            unpriced_count = EXCLUDED.unpriced_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = now_hour - Duration::hours(HOURLY_PRUNE_AFTER_HOURS);
    sqlx::query("DELETE FROM protocol_hourly_fees WHERE utc_hour < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn refresh_protocol_monthly(pool: &PgPool) -> Result<(), sqlx::Error> {
    let this_month = utc_month_start(Utc::now());
    let from = this_month
        .checked_sub_months(Months::new(MONTHLY_LOOKBACK_MONTHS))
        .expect("month lookback in range");
    let to_exclusive = this_month
        .checked_add_months(Months::new(1))
        .expect("next month");
    sqlx::query(
        r#"
        INSERT INTO protocol_monthly_fees
            (utc_month, fees_usd, event_count, unpriced_count, refreshed_at)
        SELECT
            m.utc_month,
            CASE
                WHEN COALESCE(a.event_count, 0) = 0 THEN 0::numeric
                WHEN a.fees_usd IS NOT NULL AND a.fees_usd > 0 AND a.fees_usd < 1e20
                    THEN a.fees_usd
                ELSE NULL
            END,
            COALESCE(a.event_count, 0),
            COALESCE(a.unpriced_count, 0),
            NOW()
        FROM generate_series($1::date, $2::date - interval '1 month', interval '1 month') AS g(ts)
        CROSS JOIN LATERAL (SELECT g.ts::date AS utc_month) m
        LEFT JOIN (
            SELECT
                (date_trunc('month', block_timestamp AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')::date AS utc_month,
                COUNT(*)::bigint AS event_count,
                COUNT(*) FILTER (
                    WHERE fee_usd IS NULL OR fee_usd <= 0
                )::bigint AS unpriced_count,
                SUM(fee_usd) FILTER (
                    WHERE fee_usd IS NOT NULL AND fee_usd > 0
                ) AS fees_usd
            FROM protocol_fee_events
            WHERE block_timestamp >= $3 AND block_timestamp < $4
            GROUP BY 1
        ) a ON a.utc_month = m.utc_month
        ON CONFLICT (utc_month) DO UPDATE SET
            fees_usd = EXCLUDED.fees_usd,
            event_count = EXCLUDED.event_count,
            unpriced_count = EXCLUDED.unpriced_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .bind(Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0).expect("midnight")))
    .bind(Utc.from_utc_datetime(&to_exclusive.and_hms_opt(0, 0, 0).expect("midnight")))
    .execute(pool)
    .await?;
    let prune_before = this_month
        .checked_sub_months(Months::new(MONTHLY_PRUNE_AFTER_MONTHS))
        .expect("month prune in range");
    sqlx::query("DELETE FROM protocol_monthly_fees WHERE utc_month < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn refresh_protocol_fee_series_rollups(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_protocol_daily(pool).await?;
    refresh_protocol_hourly(pool).await?;
    refresh_protocol_monthly(pool).await?;
    Ok(())
}

/// Single-window helper for tests (same idle / unpriced contract as bulk refresh).
pub async fn refresh_protocol_hour_window(
    pool: &PgPool,
    hour_start: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let hour_end = hour_start + Duration::hours(1);
    let (event_count, unpriced, fees_usd) =
        aggregate_fee_window(pool, hour_start, hour_end).await?;
    sqlx::query(
        r#"
        INSERT INTO protocol_hourly_fees
            (utc_hour, fees_usd, event_count, unpriced_count, refreshed_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (utc_hour) DO UPDATE SET
            fees_usd = EXCLUDED.fees_usd,
            event_count = EXCLUDED.event_count,
            unpriced_count = EXCLUDED.unpriced_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(hour_start)
    .bind(fees_usd.as_ref())
    .bind(event_count)
    .bind(unpriced)
    .execute(pool)
    .await?;
    Ok(())
}

async fn aggregate_fee_window(
    pool: &PgPool,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<(i64, i64, Option<BigDecimal>), sqlx::Error> {
    #[derive(FromRow)]
    struct FeeAgg {
        event_count: i64,
        unpriced_count: i64,
        fees_usd: Option<BigDecimal>,
    }

    let agg: FeeAgg = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS event_count,
            COUNT(*) FILTER (
                WHERE fee_usd IS NULL OR fee_usd <= 0
            )::bigint AS unpriced_count,
            SUM(fee_usd) FILTER (
                WHERE fee_usd IS NOT NULL AND fee_usd > 0
            ) AS fees_usd
        FROM protocol_fee_events
        WHERE block_timestamp >= $1
          AND block_timestamp < $2
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;

    Ok((
        agg.event_count,
        agg.unpriced_count,
        fees_usd_from_agg(agg.event_count, agg.fees_usd),
    ))
}
