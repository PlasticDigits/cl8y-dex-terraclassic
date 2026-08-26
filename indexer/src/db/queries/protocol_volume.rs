//! UTC Protocol volume rollups (GitLab #652 / #668).
//!
//! Same P522-Q catalog as overview `total_volume_*_usd` — includes gems, wrap, and
//! UST1-window swaps. Not `defillama_daily_stats`. GET reads these tables only.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Timelike, TimeZone, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::defillama::utc_day_start;
use crate::indexer::pair_price_usd::fits_numeric_38_18;

/// Refresh today plus this many prior UTC days (90-point daily grain + slack).
pub const DAILY_LOOKBACK_DAYS: i64 = 90;
/// Prune daily rows older than this (must be > daily grain max 90).
pub const PRUNE_AFTER_DAYS: i64 = 95;

/// Refresh this many prior UTC hours (hourly grain max 168 + slack).
pub const HOURLY_LOOKBACK_HOURS: i64 = 240;
/// Prune hourly rows older than this (~10d).
pub const HOURLY_PRUNE_AFTER_HOURS: i64 = 240;

pub const MONTHLY_LOOKBACK_MONTHS: u32 = 24;
pub const MONTHLY_PRUNE_AFTER_MONTHS: u32 = 26;

pub const GRAIN_MAX_HOURLY: i32 = 168;
pub const GRAIN_MAX_DAILY: i32 = 90;
pub const GRAIN_MAX_MONTHLY: i32 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VolumeGrain {
    Hourly,
    Daily,
    Monthly,
}

impl VolumeGrain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Monthly => "monthly",
        }
    }

    pub fn max_limit(self) -> i32 {
        match self {
            Self::Hourly => GRAIN_MAX_HOURLY,
            Self::Daily => GRAIN_MAX_DAILY,
            Self::Monthly => GRAIN_MAX_MONTHLY,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolDailyVolumeRow {
    pub utc_day: NaiveDate,
    pub volume_usd: Option<BigDecimal>,
    pub trade_count: i64,
    pub unpriced_trade_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolHourlyVolumeRow {
    pub utc_hour: DateTime<Utc>,
    pub volume_usd: Option<BigDecimal>,
    pub trade_count: i64,
    pub unpriced_trade_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolMonthlyVolumeRow {
    pub utc_month: NaiveDate,
    pub volume_usd: Option<BigDecimal>,
    pub trade_count: i64,
    pub unpriced_trade_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

/// Allowlist `days=7|30`. Missing / empty / injection → `Err`.
pub fn parse_protocol_volume_days(raw: Option<&str>) -> Result<i32, ()> {
    match raw.map(str::trim) {
        Some("7") => Ok(7),
        Some("30") => Ok(30),
        _ => Err(()),
    }
}

/// Allowlist `grain=hourly|daily|monthly`.
pub fn parse_volume_grain(raw: Option<&str>) -> Result<VolumeGrain, ()> {
    match raw.map(str::trim) {
        Some("hourly") => Ok(VolumeGrain::Hourly),
        Some("daily") => Ok(VolumeGrain::Daily),
        Some("monthly") => Ok(VolumeGrain::Monthly),
        _ => Err(()),
    }
}

/// Integer `limit` in `1..=grain.max`. Non-digits / overflow / empty → `Err`.
pub fn parse_volume_limit(raw: Option<&str>, grain: VolumeGrain) -> Result<i32, ()> {
    let s = raw.map(str::trim).filter(|t| !t.is_empty()).ok_or(())?;
    if s.len() > 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    let n: i32 = s.parse().map_err(|_| ())?;
    if n < 1 || n > grain.max_limit() {
        return Err(());
    }
    Ok(n)
}

pub fn utc_hour_start(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .and_hms_opt(now.hour(), 0, 0)
        .map(|naive| Utc.from_utc_datetime(&naive))
        .expect("hour:00:00 is a valid naive time")
}

pub fn utc_month_start(now: DateTime<Utc>) -> NaiveDate {
    NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("month start is valid")
}

pub fn format_utc_hour(hour: DateTime<Utc>) -> String {
    hour.format("%Y-%m-%dT%H").to_string()
}

pub fn format_utc_month(month: NaiveDate) -> String {
    month.format("%Y-%m").to_string()
}

pub async fn get_daily_rows(
    pool: &PgPool,
    from_day: NaiveDate,
    to_day: NaiveDate,
) -> Result<Vec<ProtocolDailyVolumeRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolDailyVolumeRow>(
        r#"SELECT utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at
           FROM protocol_daily_volume
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
) -> Result<Vec<ProtocolHourlyVolumeRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolHourlyVolumeRow>(
        r#"SELECT utc_hour, volume_usd, trade_count, unpriced_trade_count, refreshed_at
           FROM protocol_hourly_volume
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
) -> Result<Vec<ProtocolMonthlyVolumeRow>, sqlx::Error> {
    sqlx::query_as::<_, ProtocolMonthlyVolumeRow>(
        r#"SELECT utc_month, volume_usd, trade_count, unpriced_trade_count, refreshed_at
           FROM protocol_monthly_volume
           WHERE utc_month >= $1 AND utc_month <= $2
           ORDER BY utc_month ASC"#,
    )
    .bind(from_month)
    .bind(to_month)
    .fetch_all(pool)
    .await
}

fn volume_usd_from_agg(trade_count: i64, volume_usd: Option<BigDecimal>) -> Option<BigDecimal> {
    if trade_count <= 0 {
        Some(BigDecimal::from(0))
    } else {
        match volume_usd {
            Some(v) if v > BigDecimal::from(0) && fits_numeric_38_18(&v) => Some(v),
            _ => None,
        }
    }
}

/// Materialize today and the prior [`DAILY_LOOKBACK_DAYS`] UTC days; prune older than [`PRUNE_AFTER_DAYS`].
pub async fn refresh_protocol_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = utc_day_start(Utc::now());
    let from = today - Duration::days(DAILY_LOOKBACK_DAYS);
    let to_exclusive = today + Duration::days(1);
    sqlx::query(
        r#"
        INSERT INTO protocol_daily_volume
            (utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at)
        SELECT
            d.utc_day,
            CASE
                WHEN COALESCE(a.trade_count, 0) = 0 THEN 0::numeric
                WHEN a.volume_usd IS NOT NULL AND a.volume_usd > 0 AND a.volume_usd < 1e20
                    THEN a.volume_usd
                ELSE NULL
            END,
            COALESCE(a.trade_count, 0),
            COALESCE(a.unpriced_trade_count, 0),
            NOW()
        FROM generate_series(($1::timestamptz AT TIME ZONE 'UTC')::date,
                             ($2::timestamptz AT TIME ZONE 'UTC')::date - 1,
                             interval '1 day') AS d(utc_day)
        LEFT JOIN (
            SELECT
                (block_timestamp AT TIME ZONE 'UTC')::date AS utc_day,
                COUNT(*)::bigint AS trade_count,
                COUNT(*) FILTER (
                    WHERE volume_usd IS NULL OR volume_usd <= 0
                )::bigint AS unpriced_trade_count,
                SUM(volume_usd) FILTER (
                    WHERE volume_usd IS NOT NULL AND volume_usd > 0
                ) AS volume_usd
            FROM swap_events
            WHERE block_timestamp >= $1 AND block_timestamp < $2
            GROUP BY 1
        ) a ON a.utc_day = d.utc_day
        ON CONFLICT (utc_day) DO UPDATE SET
            volume_usd = EXCLUDED.volume_usd,
            trade_count = EXCLUDED.trade_count,
            unpriced_trade_count = EXCLUDED.unpriced_trade_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = (today - Duration::days(PRUNE_AFTER_DAYS)).date_naive();
    sqlx::query("DELETE FROM protocol_daily_volume WHERE utc_day < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

/// Single UTC day (tests / incremental). Same idle / unpriced rules as the bulk refresh.
pub async fn refresh_protocol_day(
    pool: &PgPool,
    day_start: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let day_end = day_start + Duration::days(1);
    let utc_day = day_start.date_naive();
    let (trade_count, unpriced, volume_usd) =
        aggregate_swap_window(pool, day_start, day_end).await?;
    sqlx::query(
        r#"
        INSERT INTO protocol_daily_volume
            (utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (utc_day) DO UPDATE SET
            volume_usd = EXCLUDED.volume_usd,
            trade_count = EXCLUDED.trade_count,
            unpriced_trade_count = EXCLUDED.unpriced_trade_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(utc_day)
    .bind(volume_usd.as_ref())
    .bind(trade_count)
    .bind(unpriced)
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
        INSERT INTO protocol_hourly_volume
            (utc_hour, volume_usd, trade_count, unpriced_trade_count, refreshed_at)
        SELECT
            h.utc_hour,
            CASE
                WHEN COALESCE(a.trade_count, 0) = 0 THEN 0::numeric
                WHEN a.volume_usd IS NOT NULL AND a.volume_usd > 0 AND a.volume_usd < 1e20
                    THEN a.volume_usd
                ELSE NULL
            END,
            COALESCE(a.trade_count, 0),
            COALESCE(a.unpriced_trade_count, 0),
            NOW()
        FROM generate_series($1::timestamptz, $2::timestamptz - interval '1 hour', interval '1 hour')
            AS h(utc_hour)
        LEFT JOIN (
            SELECT
                (date_trunc('hour', block_timestamp AT TIME ZONE 'UTC') AT TIME ZONE 'UTC') AS utc_hour,
                COUNT(*)::bigint AS trade_count,
                COUNT(*) FILTER (
                    WHERE volume_usd IS NULL OR volume_usd <= 0
                )::bigint AS unpriced_trade_count,
                SUM(volume_usd) FILTER (
                    WHERE volume_usd IS NOT NULL AND volume_usd > 0
                ) AS volume_usd
            FROM swap_events
            WHERE block_timestamp >= $1 AND block_timestamp < $2
            GROUP BY 1
        ) a ON a.utc_hour = h.utc_hour
        ON CONFLICT (utc_hour) DO UPDATE SET
            volume_usd = EXCLUDED.volume_usd,
            trade_count = EXCLUDED.trade_count,
            unpriced_trade_count = EXCLUDED.unpriced_trade_count,
            refreshed_at = EXCLUDED.refreshed_at
        "#,
    )
    .bind(from)
    .bind(to_exclusive)
    .execute(pool)
    .await?;
    let prune_before = now_hour - Duration::hours(HOURLY_PRUNE_AFTER_HOURS);
    sqlx::query("DELETE FROM protocol_hourly_volume WHERE utc_hour < $1")
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
        INSERT INTO protocol_monthly_volume
            (utc_month, volume_usd, trade_count, unpriced_trade_count, refreshed_at)
        SELECT
            m.utc_month,
            CASE
                WHEN COALESCE(a.trade_count, 0) = 0 THEN 0::numeric
                WHEN a.volume_usd IS NOT NULL AND a.volume_usd > 0 AND a.volume_usd < 1e20
                    THEN a.volume_usd
                ELSE NULL
            END,
            COALESCE(a.trade_count, 0),
            COALESCE(a.unpriced_trade_count, 0),
            NOW()
        FROM generate_series($1::date, $2::date - interval '1 month', interval '1 month') AS g(ts)
        CROSS JOIN LATERAL (SELECT g.ts::date AS utc_month) m
        LEFT JOIN (
            SELECT
                (date_trunc('month', block_timestamp AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')::date AS utc_month,
                COUNT(*)::bigint AS trade_count,
                COUNT(*) FILTER (
                    WHERE volume_usd IS NULL OR volume_usd <= 0
                )::bigint AS unpriced_trade_count,
                SUM(volume_usd) FILTER (
                    WHERE volume_usd IS NOT NULL AND volume_usd > 0
                ) AS volume_usd
            FROM swap_events
            WHERE block_timestamp >= $3 AND block_timestamp < $4
            GROUP BY 1
        ) a ON a.utc_month = m.utc_month
        ON CONFLICT (utc_month) DO UPDATE SET
            volume_usd = EXCLUDED.volume_usd,
            trade_count = EXCLUDED.trade_count,
            unpriced_trade_count = EXCLUDED.unpriced_trade_count,
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
    sqlx::query("DELETE FROM protocol_monthly_volume WHERE utc_month < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

/// Daily + hourly + monthly rollups (aggregator / GitLab #668).
pub async fn refresh_protocol_volume_rollups(pool: &PgPool) -> Result<(), sqlx::Error> {
    refresh_protocol_daily(pool).await?;
    refresh_protocol_hourly(pool).await?;
    refresh_protocol_monthly(pool).await?;
    Ok(())
}

async fn aggregate_swap_window(
    pool: &PgPool,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<(i64, i64, Option<BigDecimal>), sqlx::Error> {
    #[derive(FromRow)]
    struct VolAgg {
        trade_count: i64,
        unpriced_trade_count: i64,
        volume_usd: Option<BigDecimal>,
    }

    let vol: VolAgg = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS trade_count,
            COUNT(*) FILTER (
                WHERE volume_usd IS NULL OR volume_usd <= 0
            )::bigint AS unpriced_trade_count,
            SUM(volume_usd) FILTER (
                WHERE volume_usd IS NOT NULL AND volume_usd > 0
            ) AS volume_usd
        FROM swap_events
        WHERE block_timestamp >= $1
          AND block_timestamp < $2
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;

    Ok((
        vol.trade_count,
        vol.unpriced_trade_count,
        volume_usd_from_agg(vol.trade_count, vol.volume_usd),
    ))
}

#[cfg(test)]
mod tests {
    use super::{parse_protocol_volume_days, parse_volume_grain, parse_volume_limit, VolumeGrain};

    #[test]
    fn parse_days_allowlist() {
        assert_eq!(parse_protocol_volume_days(Some("7")), Ok(7));
        assert_eq!(parse_protocol_volume_days(Some("30")), Ok(30));
        assert_eq!(parse_protocol_volume_days(Some(" 7 ")), Ok(7));
        assert_eq!(parse_protocol_volume_days(Some("1")), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("90")), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("")), Err(()));
        assert_eq!(parse_protocol_volume_days(None), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("7;")), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("999999")), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("-7")), Err(()));
        assert_eq!(parse_protocol_volume_days(Some("7%3b")), Err(()));
    }

    #[test]
    fn parse_grain_allowlist() {
        assert_eq!(parse_volume_grain(Some("hourly")), Ok(VolumeGrain::Hourly));
        assert_eq!(parse_volume_grain(Some("daily")), Ok(VolumeGrain::Daily));
        assert_eq!(parse_volume_grain(Some("monthly")), Ok(VolumeGrain::Monthly));
        assert_eq!(parse_volume_grain(Some(" daily ")), Ok(VolumeGrain::Daily));
        assert_eq!(parse_volume_grain(Some("week")), Err(()));
        assert_eq!(parse_volume_grain(Some("daily;")), Err(()));
        assert_eq!(parse_volume_grain(Some("daily%3b")), Err(()));
        assert_eq!(parse_volume_grain(Some("")), Err(()));
        assert_eq!(parse_volume_grain(None), Err(()));
        assert_eq!(parse_volume_grain(Some("DAILY")), Err(()));
    }

    #[test]
    fn parse_limit_allowlist() {
        assert_eq!(parse_volume_limit(Some("14"), VolumeGrain::Daily), Ok(14));
        assert_eq!(parse_volume_limit(Some("90"), VolumeGrain::Daily), Ok(90));
        assert_eq!(parse_volume_limit(Some("168"), VolumeGrain::Hourly), Ok(168));
        assert_eq!(parse_volume_limit(Some("24"), VolumeGrain::Monthly), Ok(24));
        assert_eq!(parse_volume_limit(Some("91"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("169"), VolumeGrain::Hourly), Err(()));
        assert_eq!(parse_volume_limit(Some("25"), VolumeGrain::Monthly), Err(()));
        assert_eq!(parse_volume_limit(Some("0"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("-1"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("999999"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("1e308"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some(""), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(None, VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("14;"), VolumeGrain::Daily), Err(()));
        assert_eq!(parse_volume_limit(Some("14' OR 1=1"), VolumeGrain::Daily), Err(()));
    }
}
