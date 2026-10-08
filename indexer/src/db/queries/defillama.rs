//! UTC-day DeFiLlama rollup (GitLab #631).
//!
//! GET reads `defillama_daily_stats` / `defillama_daily_fees` only.
//! Refresh scans `swap_events` + `protocol_fee_events` off the request path.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::defillama::{gem_addresses_lowercased, utc_day_start, DAILY_LOOKBACK_DAYS};
use crate::indexer::protocol_fees::FeeSource;

#[derive(Debug, Clone, FromRow)]
pub struct DailyStatRow {
    pub utc_day: NaiveDate,
    pub volume_usd: BigDecimal,
    pub trade_count: i64,
    pub unpriced_trade_count: i64,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DailyFeeRow {
    pub utc_day: NaiveDate,
    pub source: String,
    pub amount_usd: BigDecimal,
    pub event_count: i64,
    pub unpriced_count: i64,
    pub unresolved_pair_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct DailyAssetRow {
    pub utc_day: NaiveDate,
    pub ticker: String,
    pub volume_usd: BigDecimal,
    pub trade_count: i64,
    pub unpriced_trade_count: i64,
    pub fees_usd: BigDecimal,
    pub fee_event_count: i64,
    pub fee_unpriced_count: i64,
    pub price_usd: Option<BigDecimal>,
    pub price_as_of: Option<DateTime<Utc>>,
    pub circulating_raw: Option<BigDecimal>,
}

pub async fn get_daily_stat<'e, E>(
    executor: E,
    utc_day: NaiveDate,
) -> Result<Option<DailyStatRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, DailyStatRow>(
        r#"SELECT utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at
           FROM defillama_daily_stats
           WHERE utc_day = $1"#,
    )
    .bind(utc_day)
    .fetch_optional(executor)
    .await
}

pub async fn get_daily_fees<'e, E>(
    executor: E,
    utc_day: NaiveDate,
) -> Result<Vec<DailyFeeRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, DailyFeeRow>(
        r#"SELECT utc_day, source, amount_usd, event_count, unpriced_count, unresolved_pair_count
           FROM defillama_daily_fees
           WHERE utc_day = $1"#,
    )
    .bind(utc_day)
    .fetch_all(executor)
    .await
}

pub async fn get_daily_assets<'e, E>(
    executor: E,
    utc_day: NaiveDate,
) -> Result<Vec<DailyAssetRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, DailyAssetRow>(
        r#"SELECT utc_day, ticker, volume_usd, trade_count, unpriced_trade_count,
                  fees_usd, fee_event_count, fee_unpriced_count, price_usd, circulating_raw, price_as_of
           FROM defillama_daily_assets
           WHERE utc_day = $1
           ORDER BY ticker"#,
    )
    .bind(utc_day)
    .fetch_all(executor)
    .await
}

/// Materialize today and the prior [`DAILY_LOOKBACK_DAYS`] UTC days.
pub async fn refresh_defillama_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let today = utc_day_start(Utc::now());
    for offset in 0..=DAILY_LOOKBACK_DAYS {
        let start = today - Duration::days(offset);
        refresh_defillama_day(pool, start).await?;
    }
    Ok(())
}

pub async fn refresh_defillama_day(
    pool: &PgPool,
    day_start: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    materialize_defillama_day(pool, day_start, false).await
}

/// Reaggregate historical ledger amounts without resampling advisory asset marks
/// or circulating supply. Missing historical snapshots remain unavailable.
pub async fn repair_defillama_day(
    pool: &PgPool,
    day_start: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    materialize_defillama_day(pool, day_start, true).await
}

async fn materialize_defillama_day(
    pool: &PgPool,
    day_start: DateTime<Utc>,
    preserve_auxiliary_snapshots: bool,
) -> Result<(), sqlx::Error> {
    if day_start != utc_day_start(day_start) {
        return Err(sqlx::Error::Protocol(
            "DeFiLlama refresh requires a UTC day boundary".into(),
        ));
    }
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '120s'")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *transaction)
        .await?;
    // Never wait on a competing refresh after taking the repeatable-read snapshot.
    // A waiting advisory lock could otherwise publish data from before that writer.
    let locked: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(1354, ($1::date - DATE '1970-01-01')::integer)",
    )
    .bind(day_start.date_naive())
    .fetch_one(&mut *transaction)
    .await?;
    if !locked {
        return Err(sqlx::Error::Protocol(
            "UTC-day DeFiLlama refresh already running".into(),
        ));
    }
    let day_end = day_start + Duration::days(1);
    let utc_day = day_start.date_naive();
    let gems = gem_addresses_lowercased();

    #[derive(FromRow)]
    struct VolAgg {
        trade_count: i64,
        unpriced_trade_count: i64,
        volume_usd: Option<BigDecimal>,
    }

    let vol: VolAgg = sqlx::query_as(
        r#"
        WITH gem_pairs AS (
            SELECT p.id
            FROM pairs p
            JOIN assets a0 ON a0.id = p.asset_0_id
            JOIN assets a1 ON a1.id = p.asset_1_id
            WHERE LOWER(COALESCE(a0.contract_address, '')) = ANY($3)
               OR LOWER(COALESCE(a1.contract_address, '')) = ANY($3)
        )
        SELECT
            COUNT(*)::bigint AS trade_count,
            COUNT(*) FILTER (
                WHERE volume_usd IS NULL OR volume_usd <= 0
            )::bigint AS unpriced_trade_count,
            COALESCE(
                SUM(volume_usd) FILTER (
                    WHERE volume_usd IS NOT NULL AND volume_usd > 0
                ),
                0
            ) AS volume_usd
        FROM swap_events se
        WHERE se.block_timestamp >= $1
          AND se.block_timestamp < $2
          AND se.pair_id NOT IN (SELECT id FROM gem_pairs)
        "#,
    )
    .bind(day_start)
    .bind(day_end)
    .bind(&gems)
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO defillama_daily_stats
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
    .bind(vol.volume_usd.unwrap_or_else(|| BigDecimal::from(0)))
    .bind(vol.trade_count)
    .bind(vol.unpriced_trade_count)
    .execute(&mut *transaction)
    .await?;

    #[derive(FromRow)]
    struct FeeAgg {
        source: String,
        event_count: i64,
        unpriced_count: i64,
        unresolved_pair_count: i64,
        amount_usd: Option<BigDecimal>,
    }

    let fee_sql = format!(
        "{}{}",
        include_str!("defillama_fee_scope.sql"),
        r#"
        SELECT e.source,
               COUNT(*)::bigint AS event_count,
               COUNT(*) FILTER (WHERE e.fee_usd IS NULL OR e.fee_usd <= 0)::bigint AS unpriced_count,
               COUNT(*) FILTER (WHERE e.unresolved_pair_scope)::bigint AS unresolved_pair_count,
               COALESCE(SUM(e.fee_usd) FILTER (WHERE e.fee_usd > 0), 0) AS amount_usd
        FROM scoped_fee_events e
        WHERE e.resolved_pair_id IS NULL
           OR e.resolved_pair_id NOT IN (SELECT id FROM gem_pairs)
        GROUP BY e.source
        "#
    );
    let fees: Vec<FeeAgg> = sqlx::query_as(&fee_sql)
        .bind(day_start)
        .bind(day_end)
        .bind(&gems)
        .fetch_all(&mut *transaction)
        .await?;

    sqlx::query("DELETE FROM defillama_daily_fees WHERE utc_day = $1")
        .bind(utc_day)
        .execute(&mut *transaction)
        .await?;

    let pins: Option<(bool, bool)> = sqlx::query_as(
        "SELECT factory_configured, community_invoices_configured FROM global_stats_24h WHERE id = 1"
    ).fetch_optional(&mut *transaction).await?;
    let (factory_configured, community_invoices_configured) = pins.unwrap_or_default();
    for source in FeeSource::ALL {
        let row = fees.iter().find(|r| r.source == source.as_str());
        let has_attested_history = row.is_some_and(|r| r.event_count > 0);
        // Removing a current pin does not erase immutable already-ingested fees.
        if source == FeeSource::PairCreation && !factory_configured && !has_attested_history {
            continue;
        }
        if matches!(source, FeeSource::SkuUnlock | FeeSource::SettingsFee)
            && !community_invoices_configured
            && !has_attested_history
        {
            continue;
        }
        let (amount, count, unpriced, unresolved) = match row {
            Some(r) => (
                r.amount_usd.clone().unwrap_or_else(|| BigDecimal::from(0)),
                r.event_count,
                r.unpriced_count,
                r.unresolved_pair_count,
            ),
            None => (BigDecimal::from(0), 0, 0, 0),
        };
        sqlx::query(
            r#"
            INSERT INTO defillama_daily_fees
                (utc_day, source, amount_usd, event_count, unpriced_count, unresolved_pair_count)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(utc_day)
        .bind(source.as_str())
        .bind(&amount)
        .bind(count)
        .bind(unpriced)
        .bind(unresolved)
        .execute(&mut *transaction)
        .await?;
    }

    super::defillama_assets::refresh_daily_assets(
        &mut transaction,
        day_start,
        day_end,
        utc_day,
        &gems,
        preserve_auxiliary_snapshots,
    )
    .await?;
    transaction.commit().await?;

    Ok(())
}
