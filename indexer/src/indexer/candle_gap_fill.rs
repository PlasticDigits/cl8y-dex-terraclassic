//! One boot pass for pairs that have positive-price swaps and zero candle rows (#1315).
//!
//! The done marker is written only when `failure_count` is 0. A failed pair rolls
//! back, stays at zero candle rows, and is retried on the next boot. Stamped buckets
//! that committed stay as written.

use std::fmt;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};

use crate::db::queries::assets;
use crate::db::queries::hub_prices;
use crate::indexer::candle_builder::{self, INTERVALS};
use crate::indexer::pair_price_usd::{candle_usd_subject, CandleUsdPrints, CandleUsdSubject};
use crate::indexer::swap_orientation;

#[derive(Debug)]
struct FillError(String);

impl fmt::Display for FillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FillError {}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapFillReport {
    pub pairs_considered: u64,
    pub rows_inserted: u64,
    pub swaps_skipped: u64,
    pub failure_count: u64,
    pub already_done: bool,
}

/// `se.price > 0` and zero `candles` rows. The live pass uses this predicate in SQL.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_zero_candle_gap_candidate(positive_price: bool, candle_rows: i64) -> bool {
    positive_price && candle_rows == 0
}

/// Boot must not stick a failed pass. The next start retries pairs that still have no candles.
pub fn should_set_gap_fill_done(failure_count: u64) -> bool {
    failure_count == 0
}

pub async fn run_once(
    pool: &PgPool,
    configured_ustc_denom: Option<&str>,
    configured_usdt_address: &str,
) -> Result<GapFillReport, sqlx::Error> {
    if gap_fill_done(pool).await? {
        tracing::debug!("candle gap fill already done");
        return Ok(GapFillReport {
            pairs_considered: 0,
            rows_inserted: 0,
            swaps_skipped: 0,
            failure_count: 0,
            already_done: true,
        });
    }

    let pair_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT p.id
           FROM pairs p
          WHERE EXISTS (
                  SELECT 1 FROM swap_events se
                   WHERE se.pair_id = p.id AND se.price > 0
                )
            AND NOT EXISTS (
                  SELECT 1 FROM candles c WHERE c.pair_id = p.id
                )
          ORDER BY p.id",
    )
    .fetch_all(pool)
    .await?;

    let hub = hub_prices::load_quote_usd(pool).await.ok();
    let mut rows_inserted = 0u64;
    let mut swaps_skipped = 0u64;
    let mut failure_count = 0u64;

    for pair_id in &pair_ids {
        match fill_pair(
            pool,
            *pair_id,
            hub.as_ref(),
            configured_ustc_denom,
            configured_usdt_address,
        )
        .await
        {
            Ok(stats) => {
                rows_inserted += stats.rows_inserted;
                swaps_skipped += stats.swaps_skipped;
            }
            Err(e) => {
                failure_count += 1;
                tracing::warn!(pair_id, error = %e, "candle gap fill pair failed");
            }
        }
    }

    if should_set_gap_fill_done(failure_count) {
        sqlx::query(
            "INSERT INTO candle_gap_fill_1315_done (id, finished_at)
             VALUES (1, NOW())
             ON CONFLICT (id) DO UPDATE SET finished_at = EXCLUDED.finished_at",
        )
        .execute(pool)
        .await?;
    }

    let report = GapFillReport {
        pairs_considered: pair_ids.len() as u64,
        rows_inserted,
        swaps_skipped,
        failure_count,
        already_done: false,
    };
    tracing::info!(
        pairs_considered = report.pairs_considered,
        rows_inserted = report.rows_inserted,
        swaps_skipped = report.swaps_skipped,
        failure_count = report.failure_count,
        "candle gap fill"
    );
    Ok(report)
}

struct PairFillStats {
    rows_inserted: u64,
    swaps_skipped: u64,
}

async fn gap_fill_done(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let done: Option<i32> =
        sqlx::query_scalar("SELECT id FROM candle_gap_fill_1315_done WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    Ok(done.is_some())
}

async fn fill_pair(
    pool: &PgPool,
    pair_id: i32,
    hub: Option<&crate::indexer::pair_price_usd::HubQuoteUsd>,
    configured_ustc_denom: Option<&str>,
    configured_usdt_address: &str,
) -> Result<PairFillStats, BoxError> {
    let pair = sqlx::query_as::<_, (i32, i32)>(
        "SELECT asset_0_id, asset_1_id FROM pairs WHERE id = $1",
    )
    .bind(pair_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| FillError(format!("pair {pair_id} missing")))?;
    let base = assets::get_asset_by_id(pool, pair.0)
        .await?
        .ok_or_else(|| FillError(format!("asset {} missing", pair.0)))?;
    let quote = assets::get_asset_by_id(pool, pair.1)
        .await?
        .ok_or_else(|| FillError(format!("asset {} missing", pair.1)))?;

    let swaps = sqlx::query_as::<_, GapSwap>(
        "SELECT block_timestamp, price, offer_amount, return_amount, offer_asset_id
           FROM swap_events
          WHERE pair_id = $1 AND price > 0
          ORDER BY block_timestamp ASC, id ASC",
    )
    .bind(pair_id)
    .fetch_all(pool)
    .await?;

    let mut tx = pool.begin().await?;
    let mut rows_inserted = 0u64;
    let mut swaps_skipped = 0u64;
    for swap in &swaps {
        let ustc = oracle_as_of(&mut tx, "ustc", swap.block_timestamp).await?;
        let lunc = oracle_as_of(&mut tx, "lunc", swap.block_timestamp).await?;
        let prints = CandleUsdPrints {
            ustc_usd: ustc.as_ref(),
            lunc_usd: lunc.as_ref(),
            hub,
            configured_ustc_denom,
            configured_usdt_address: Some(configured_usdt_address),
        };
        let subject = candle_usd_subject(&base, &quote, &swap.price, prints);
        if matches!(subject, CandleUsdSubject::Skip) {
            swaps_skipped += 1;
            continue;
        }
        let oriented = swap_orientation::orient_swap_leg(
            pair.0,
            swap.offer_asset_id,
            &swap.offer_amount,
            &swap.return_amount,
            base.decimals,
            quote.decimals,
        );
        candle_builder::update_candles_for_swap_conn(
            &mut *tx,
            pair_id,
            swap.block_timestamp,
            &subject,
            &swap.price,
            &oriented.volume_base,
            &oriented.volume_quote,
        )
        .await?;
        rows_inserted += stamp_swap_buckets(&mut tx, pair_id, swap.block_timestamp).await?;
    }
    tx.commit().await?;
    Ok(PairFillStats {
        rows_inserted,
        swaps_skipped,
    })
}

#[derive(sqlx::FromRow)]
struct GapSwap {
    block_timestamp: DateTime<Utc>,
    price: BigDecimal,
    offer_amount: BigDecimal,
    return_amount: BigDecimal,
    offer_asset_id: i32,
}

async fn oracle_as_of(
    tx: &mut Transaction<'_, Postgres>,
    ticker: &str,
    at: DateTime<Utc>,
) -> Result<Option<BigDecimal>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT price_usd FROM oracle_prices
          WHERE ticker = $1 AND source = 'average' AND fetched_at <= $2 AND price_usd > 0
          ORDER BY fetched_at DESC
          LIMIT 1",
    )
    .bind(ticker)
    .bind(at)
    .fetch_optional(&mut **tx)
    .await
}

async fn stamp_swap_buckets(
    tx: &mut Transaction<'_, Postgres>,
    pair_id: i32,
    timestamp: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0u64;
    for &interval in INTERVALS {
        let open_time = candle_builder::truncate_to_interval(timestamp, interval);
        let res = sqlx::query(
            "INSERT INTO candle_gap_fill_1315 (pair_id, interval, open_time)
             VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(pair_id)
        .bind(interval)
        .bind(open_time)
        .execute(&mut **tx)
        .await?;
        inserted += res.rows_affected();
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::{is_zero_candle_gap_candidate, should_set_gap_fill_done};

    #[test]
    fn selector_is_positive_price_and_zero_rows() {
        assert!(is_zero_candle_gap_candidate(true, 0));
        assert!(!is_zero_candle_gap_candidate(true, 1));
        assert!(!is_zero_candle_gap_candidate(false, 0));
        assert!(!is_zero_candle_gap_candidate(false, 4));
    }

    #[test]
    fn done_marker_only_when_no_failures() {
        assert!(should_set_gap_fill_done(0));
        assert!(!should_set_gap_fill_done(1));
    }
}
