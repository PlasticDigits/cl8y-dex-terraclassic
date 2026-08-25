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
    pub circulating_raw: Option<BigDecimal>,
}

pub async fn get_daily_stat(
    pool: &PgPool,
    utc_day: NaiveDate,
) -> Result<Option<DailyStatRow>, sqlx::Error> {
    sqlx::query_as::<_, DailyStatRow>(
        r#"SELECT utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at
           FROM defillama_daily_stats
           WHERE utc_day = $1"#,
    )
    .bind(utc_day)
    .fetch_optional(pool)
    .await
}

pub async fn get_daily_fees(
    pool: &PgPool,
    utc_day: NaiveDate,
) -> Result<Vec<DailyFeeRow>, sqlx::Error> {
    sqlx::query_as::<_, DailyFeeRow>(
        r#"SELECT utc_day, source, amount_usd, event_count, unpriced_count
           FROM defillama_daily_fees
           WHERE utc_day = $1"#,
    )
    .bind(utc_day)
    .fetch_all(pool)
    .await
}

pub async fn get_daily_assets(
    pool: &PgPool,
    utc_day: NaiveDate,
) -> Result<Vec<DailyAssetRow>, sqlx::Error> {
    sqlx::query_as::<_, DailyAssetRow>(
        r#"SELECT utc_day, ticker, volume_usd, trade_count, unpriced_trade_count,
                  fees_usd, fee_event_count, fee_unpriced_count, price_usd, circulating_raw
           FROM defillama_daily_assets
           WHERE utc_day = $1
           ORDER BY ticker"#,
    )
    .bind(utc_day)
    .fetch_all(pool)
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
    .fetch_one(pool)
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
    .execute(pool)
    .await?;

    #[derive(FromRow)]
    struct FeeAgg {
        source: String,
        event_count: i64,
        unpriced_count: i64,
        amount_usd: Option<BigDecimal>,
    }

    let fees: Vec<FeeAgg> = sqlx::query_as(
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
            e.source,
            COUNT(*)::bigint AS event_count,
            COUNT(*) FILTER (
                WHERE e.fee_usd IS NULL OR e.fee_usd <= 0
            )::bigint AS unpriced_count,
            COALESCE(
                SUM(e.fee_usd) FILTER (
                    WHERE e.fee_usd IS NOT NULL AND e.fee_usd > 0
                ),
                0
            ) AS amount_usd
        FROM protocol_fee_events e
        WHERE e.block_timestamp >= $1
          AND e.block_timestamp < $2
          AND (
              e.source IN ('wrap', 'unwrap', 'ust1_mint', 'ust1_redeem')
              OR NOT EXISTS (
                  SELECT 1 FROM gem_pairs gp
                  WHERE
                      (e.source = 'swap_amm' AND EXISTS (
                          SELECT 1 FROM swap_events se
                          WHERE se.tx_hash = e.tx_hash AND se.pair_id = gp.id
                      ))
                      OR (e.source = 'book_take' AND EXISTS (
                          SELECT 1 FROM limit_order_fills f
                          WHERE f.tx_hash = e.tx_hash AND f.pair_id = gp.id
                      ))
                      OR (e.source = 'limit_place' AND EXISTS (
                          SELECT 1 FROM limit_order_placements pl
                          WHERE pl.tx_hash = e.tx_hash AND pl.pair_id = gp.id
                      ))
              )
          )
        GROUP BY e.source
        "#,
    )
    .bind(day_start)
    .bind(day_end)
    .bind(&gems)
    .fetch_all(pool)
    .await?;

    sqlx::query("DELETE FROM defillama_daily_fees WHERE utc_day = $1")
        .bind(utc_day)
        .execute(pool)
        .await?;

    for source in FeeSource::ALL {
        let row = fees.iter().find(|r| r.source == source.as_str());
        let (amount, count, unpriced) = match row {
            Some(r) => (
                r.amount_usd.clone().unwrap_or_else(|| BigDecimal::from(0)),
                r.event_count,
                r.unpriced_count,
            ),
            None => (BigDecimal::from(0), 0, 0),
        };
        sqlx::query(
            r#"
            INSERT INTO defillama_daily_fees
                (utc_day, source, amount_usd, event_count, unpriced_count)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(utc_day)
        .bind(source.as_str())
        .bind(&amount)
        .bind(count)
        .bind(unpriced)
        .execute(pool)
        .await?;
    }

    super::defillama_assets::refresh_daily_assets(pool, day_start, day_end, utc_day, &gems).await?;

    Ok(())
}
