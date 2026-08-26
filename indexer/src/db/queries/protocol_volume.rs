//! UTC-day Protocol volume rollup (GitLab #652).
//!
//! Same P522-Q catalog as overview `total_volume_*_usd` — includes gems, wrap, and
//! UST1-window swaps. Not `defillama_daily_stats`. GET reads this table only.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::defillama::utc_day_start;
use crate::indexer::pair_price_usd::fits_numeric_38_18;

/// Refresh today plus this many prior UTC days, then prune older than [`PRUNE_AFTER_DAYS`].
pub const DAILY_LOOKBACK_DAYS: i64 = 34;
pub const PRUNE_AFTER_DAYS: i64 = 35;

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolDailyVolumeRow {
    pub utc_day: NaiveDate,
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

/// Materialize today and the prior [`DAILY_LOOKBACK_DAYS`] UTC days; prune ≥ 35d.
pub async fn refresh_protocol_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = utc_day_start(Utc::now());
    for offset in 0..=DAILY_LOOKBACK_DAYS {
        let start = today - Duration::days(offset);
        refresh_protocol_day(pool, start).await?;
    }
    let prune_before = (today - Duration::days(PRUNE_AFTER_DAYS)).date_naive();
    sqlx::query("DELETE FROM protocol_daily_volume WHERE utc_day < $1")
        .bind(prune_before)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn refresh_protocol_day(
    pool: &PgPool,
    day_start: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let day_end = day_start + Duration::days(1);
    let utc_day = day_start.date_naive();

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
    .bind(day_start)
    .bind(day_end)
    .fetch_one(pool)
    .await?;

    let volume_usd = if vol.trade_count <= 0 {
        Some(BigDecimal::from(0))
    } else {
        match vol.volume_usd {
            Some(v) if v > BigDecimal::from(0) && fits_numeric_38_18(&v) => Some(v),
            _ => None,
        }
    };

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
    .bind(vol.trade_count)
    .bind(vol.unpriced_trade_count)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_protocol_volume_days;

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
}
