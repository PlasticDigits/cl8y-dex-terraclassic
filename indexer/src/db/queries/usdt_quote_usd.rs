//! NULL-only USDT quote USD backfill (GitLab #1258).
//!
//! Advisory $1 on the pinned registry USDT CW20. Never rewrites non-NULL
//! `price_usd` / `volume_usd` (#568). Never touches UST1/cUSTC history.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::candles;

/// Same grain list as live candle writes. Kept local so `db::queries` does not
/// import `indexer::candle_builder` (that module already depends on `queries::candles`).
const CANDLE_INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

/// Fill NULL `swap_events.price_usd` / `volume_usd` when the pair quote (or a swap
/// leg) is the pinned USDT CW20, then rebuild candles for those pairs.
pub async fn backfill_null_usdt_quote_usd(
    pool: &PgPool,
    usdt_address: &str,
) -> Result<UsdtQuoteBackfillStats, sqlx::Error> {
    let pin = usdt_address.trim().to_ascii_lowercase();
    if pin.is_empty() {
        return Ok(UsdtQuoteBackfillStats::default());
    }

    let price_filled = sqlx::query(
        "UPDATE swap_events se
         SET price_usd = se.price
         FROM pairs p
         JOIN assets q ON q.id = p.asset_1_id
         WHERE se.pair_id = p.id
           AND se.price_usd IS NULL
           AND se.price > 0
           AND se.price < POWER(10::numeric, 20)
           AND q.is_cw20
           AND lower(q.contract_address) = $1",
    )
    .bind(&pin)
    .execute(pool)
    .await?
    .rows_affected();

    let volume_filled = sqlx::query(
        "UPDATE swap_events se
         SET volume_usd = x.usd
         FROM (
             SELECT se2.id,
                    CASE
                        WHEN se2.offer_asset_id = u.id
                             AND u.decimals BETWEEN 0 AND 38
                            THEN se2.offer_amount / POWER(10::numeric, u.decimals)
                        WHEN se2.ask_asset_id = u.id
                             AND u.decimals BETWEEN 0 AND 38
                            THEN se2.return_amount / POWER(10::numeric, u.decimals)
                        ELSE NULL
                    END AS usd
             FROM swap_events se2
             JOIN pairs p ON p.id = se2.pair_id
             JOIN assets u ON u.is_cw20 AND lower(u.contract_address) = $1
             WHERE se2.volume_usd IS NULL
         ) x
         WHERE se.id = x.id
           AND x.usd IS NOT NULL
           AND x.usd > 0
           AND x.usd < POWER(10::numeric, 20)
           AND se.volume_usd IS NULL",
    )
    .bind(&pin)
    .execute(pool)
    .await?
    .rows_affected();

    let pair_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT p.id
         FROM pairs p
         JOIN assets a0 ON a0.id = p.asset_0_id
         JOIN assets a1 ON a1.id = p.asset_1_id
         WHERE (a0.is_cw20 AND lower(a0.contract_address) = $1)
            OR (a1.is_cw20 AND lower(a1.contract_address) = $1)",
    )
    .bind(&pin)
    .fetch_all(pool)
    .await?;

    let from = DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now);
    let mut candles_rebuilt = 0u64;
    for pair_id in &pair_ids {
        for interval in CANDLE_INTERVALS {
            candles::rebuild_candles_from_swaps(pool, *pair_id, interval, from).await?;
            candles_rebuilt += 1;
        }
    }

    if price_filled > 0 || volume_filled > 0 {
        tracing::info!(
            price_filled,
            volume_filled,
            pairs = pair_ids.len(),
            "NULL-only USDT quote USD backfill (GitLab #1258)"
        );
    }

    Ok(UsdtQuoteBackfillStats {
        price_filled,
        volume_filled,
        pairs: pair_ids.len() as u64,
        candles_rebuilt,
    })
}

#[derive(Debug, Clone, Default)]
pub struct UsdtQuoteBackfillStats {
    pub price_filled: u64,
    pub volume_filled: u64,
    pub pairs: u64,
    pub candles_rebuilt: u64,
}
