use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct SwapEventRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub sender: String,
    pub receiver: Option<String>,
    pub offer_asset_id: i32,
    pub ask_asset_id: i32,
    pub offer_amount: BigDecimal,
    pub return_amount: BigDecimal,
    pub spread_amount: Option<BigDecimal>,
    pub commission_amount: Option<BigDecimal>,
    pub effective_fee_bps: Option<i16>,
    pub price: BigDecimal,
    pub volume_usd: Option<BigDecimal>,
    pub pool_return_amount: Option<BigDecimal>,
    pub book_return_amount: Option<BigDecimal>,
    pub limit_book_offer_consumed: Option<BigDecimal>,
}

#[derive(Debug, Clone, Default)]
pub struct PairStats {
    pub volume_base: BigDecimal,
    pub volume_quote: BigDecimal,
    pub volume_usd: Option<BigDecimal>,
    pub trade_count: i64,
    pub high: Option<BigDecimal>,
    pub low: Option<BigDecimal>,
    pub open_price: Option<BigDecimal>,
    pub close_price: Option<BigDecimal>,
    pub price_change_pct: Option<f64>,
}

/// 24h hybrid vs pool-only attribution for consolidated CG/CMC reporting (GitLab #189).
#[derive(Debug, Clone, Default)]
pub struct HybridVolumeBreakdown {
    pub hybrid_trade_count: i64,
    pub pool_only_trade_count: i64,
    pub book_leg_volume_quote: BigDecimal,
    pub pool_leg_volume_quote: BigDecimal,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_swap(
    pool: &PgPool,
    pair_id: i32,
    swap_index: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    sender: &str,
    receiver: Option<&str>,
    offer_asset_id: i32,
    ask_asset_id: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
    spread_amount: Option<&BigDecimal>,
    commission_amount: Option<&BigDecimal>,
    effective_fee_bps: Option<i16>,
    price: &BigDecimal,
    volume_usd: Option<&BigDecimal>,
    pool_return_amount: Option<&BigDecimal>,
    book_return_amount: Option<&BigDecimal>,
    limit_book_offer_consumed: Option<&BigDecimal>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender, receiver,
          offer_asset_id, ask_asset_id, offer_amount, return_amount,
          spread_amount, commission_amount, effective_fee_bps, price, volume_usd,
          pool_return_amount, book_return_amount, limit_book_offer_consumed, swap_index)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
         ON CONFLICT (tx_hash, pair_id, swap_index) DO NOTHING
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(sender)
    .bind(receiver)
    .bind(offer_asset_id)
    .bind(ask_asset_id)
    .bind(offer_amount)
    .bind(return_amount)
    .bind(spread_amount)
    .bind(commission_amount)
    .bind(effective_fee_bps)
    .bind(price)
    .bind(volume_usd)
    .bind(pool_return_amount)
    .bind(book_return_amount)
    .bind(limit_book_offer_consumed)
    .bind(swap_index)
    .fetch_optional(pool)
    .await
}

pub async fn get_trades_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<SwapEventRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn get_trades_for_trader(
    pool: &PgPool,
    sender: &str,
    pair_id: Option<i32>,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<SwapEventRow>, sqlx::Error> {
    match (before_id, pair_id) {
        (Some(bid), Some(pid)) => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events
                 WHERE sender = $1 AND pair_id = $4 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(sender)
            .bind(limit)
            .bind(bid)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (None, Some(pid)) => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events
                 WHERE sender = $1 AND pair_id = $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(sender)
            .bind(limit)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (Some(bid), None) => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events WHERE sender = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(sender)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, SwapEventRow>(
                "SELECT * FROM swap_events WHERE sender = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(sender)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn get_last_trade_for_pair(
    pool: &PgPool,
    pair_id: i32,
) -> Result<Option<SwapEventRow>, sqlx::Error> {
    sqlx::query_as::<_, SwapEventRow>(
        "SELECT * FROM swap_events WHERE pair_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(pair_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_24h_stats_for_pair(pool: &PgPool, pair_id: i32) -> Result<PairStats, sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct StatsRow {
        volume_base: Option<BigDecimal>,
        volume_quote: Option<BigDecimal>,
        volume_usd: Option<BigDecimal>,
        trade_count: Option<i64>,
        high: Option<BigDecimal>,
        low: Option<BigDecimal>,
    }

    let stats = sqlx::query_as::<_, StatsRow>(
        "SELECT
           COALESCE(SUM(offer_amount), 0) AS volume_base,
           COALESCE(SUM(return_amount), 0) AS volume_quote,
           SUM(volume_usd) AS volume_usd,
           COUNT(*) AS trade_count,
           MAX(price) AS high,
           MIN(price) AS low
         FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= $2",
    )
    .bind(pair_id)
    .bind(cutoff)
    .fetch_one(pool)
    .await?;

    #[derive(FromRow)]
    struct PriceRow {
        price: BigDecimal,
    }

    let open = sqlx::query_as::<_, PriceRow>(
        "SELECT price FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= $2
         ORDER BY block_timestamp ASC, id ASC LIMIT 1",
    )
    .bind(pair_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await?;

    let close = sqlx::query_as::<_, PriceRow>(
        "SELECT price FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= $2
         ORDER BY block_timestamp DESC, id DESC LIMIT 1",
    )
    .bind(pair_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await?;

    let open_price = open.map(|r| r.price);
    let close_price = close.map(|r| r.price);

    Ok(PairStats {
        volume_base: stats.volume_base.unwrap_or_default(),
        volume_quote: stats.volume_quote.unwrap_or_default(),
        volume_usd: stats.volume_usd,
        trade_count: stats.trade_count.unwrap_or(0),
        high: stats.high,
        low: stats.low,
        open_price: open_price.clone(),
        close_price: close_price.clone(),
        price_change_pct: price_change_pct(open_price.as_ref(), close_price.as_ref()),
    })
}

fn price_change_pct(open: Option<&BigDecimal>, close: Option<&BigDecimal>) -> Option<f64> {
    use bigdecimal::ToPrimitive;
    match (open, close) {
        (Some(o), Some(c)) => {
            let o_f = o.to_f64().unwrap_or(0.0);
            let c_f = c.to_f64().unwrap_or(0.0);
            if o_f != 0.0 {
                Some(((c_f - o_f) / o_f) * 100.0)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 24h stats for every pair that traded in the window — O(1) DB round-trips (GitLab #288).
pub async fn get_24h_stats_all_pairs(
    pool: &PgPool,
) -> Result<HashMap<i32, PairStats>, sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct AggRow {
        pair_id: i32,
        volume_base: Option<BigDecimal>,
        volume_quote: Option<BigDecimal>,
        volume_usd: Option<BigDecimal>,
        trade_count: Option<i64>,
        high: Option<BigDecimal>,
        low: Option<BigDecimal>,
    }

    let agg_rows = sqlx::query_as::<_, AggRow>(
        "SELECT
           pair_id,
           COALESCE(SUM(offer_amount), 0) AS volume_base,
           COALESCE(SUM(return_amount), 0) AS volume_quote,
           SUM(volume_usd) AS volume_usd,
           COUNT(*) AS trade_count,
           MAX(price) AS high,
           MIN(price) AS low
         FROM swap_events
         WHERE block_timestamp >= $1
         GROUP BY pair_id",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    #[derive(FromRow)]
    struct PriceRow {
        pair_id: i32,
        price: BigDecimal,
    }

    let open_rows = sqlx::query_as::<_, PriceRow>(
        "SELECT DISTINCT ON (pair_id) pair_id, price
         FROM swap_events
         WHERE block_timestamp >= $1
         ORDER BY pair_id, block_timestamp ASC, id ASC",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let close_rows = sqlx::query_as::<_, PriceRow>(
        "SELECT DISTINCT ON (pair_id) pair_id, price
         FROM swap_events
         WHERE block_timestamp >= $1
         ORDER BY pair_id, block_timestamp DESC, id DESC",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let open_map: HashMap<i32, BigDecimal> = open_rows.into_iter().map(|r| (r.pair_id, r.price)).collect();
    let close_map: HashMap<i32, BigDecimal> =
        close_rows.into_iter().map(|r| (r.pair_id, r.price)).collect();

    let mut result = HashMap::with_capacity(agg_rows.len());
    for row in agg_rows {
        let open_price = open_map.get(&row.pair_id).cloned();
        let close_price = close_map.get(&row.pair_id).cloned();
        let pct = price_change_pct(open_price.as_ref(), close_price.as_ref());
        result.insert(
            row.pair_id,
            PairStats {
                volume_base: row.volume_base.unwrap_or_default(),
                volume_quote: row.volume_quote.unwrap_or_default(),
                volume_usd: row.volume_usd,
                trade_count: row.trade_count.unwrap_or(0),
                high: row.high,
                low: row.low,
                open_price,
                close_price,
                price_change_pct: pct,
            },
        );
    }

    Ok(result)
}

/// Hybrid leg attribution for consolidated listing stats (ask-side: `pool_return_amount` /
/// `book_return_amount`). Totals in [`get_24h_stats_for_pair`] use `offer_amount` /
/// `return_amount` (Terraport-compatible totals); this query splits book vs pool legs without
/// double-counting `limit_order_fills` rows. See [integrators-hybrid-volume.md](../../../docs/integrators-hybrid-volume.md).
pub async fn get_24h_hybrid_breakdown(
    pool: &PgPool,
    pair_id: i32,
) -> Result<HybridVolumeBreakdown, sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct Row {
        hybrid_trade_count: Option<i64>,
        pool_only_trade_count: Option<i64>,
        book_leg_volume_quote: Option<BigDecimal>,
        pool_leg_volume_quote: Option<BigDecimal>,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT
           COUNT(*) FILTER (
             WHERE COALESCE(book_return_amount, 0) > 0
           ) AS hybrid_trade_count,
           COUNT(*) FILTER (
             WHERE book_return_amount IS NULL OR book_return_amount = 0
           ) AS pool_only_trade_count,
           COALESCE(SUM(book_return_amount), 0) AS book_leg_volume_quote,
           COALESCE(SUM(pool_return_amount), 0) AS pool_leg_volume_quote
         FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= $2",
    )
    .bind(pair_id)
    .bind(cutoff)
    .fetch_one(pool)
    .await?;

    Ok(HybridVolumeBreakdown {
        hybrid_trade_count: row.hybrid_trade_count.unwrap_or(0),
        pool_only_trade_count: row.pool_only_trade_count.unwrap_or(0),
        book_leg_volume_quote: row.book_leg_volume_quote.unwrap_or_default(),
        pool_leg_volume_quote: row.pool_leg_volume_quote.unwrap_or_default(),
    })
}

/// Hybrid breakdown for every pair that traded in the window — single grouped query (GitLab #288).
pub async fn get_24h_hybrid_breakdown_all_pairs(
    pool: &PgPool,
) -> Result<HashMap<i32, HybridVolumeBreakdown>, sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct Row {
        pair_id: i32,
        hybrid_trade_count: Option<i64>,
        pool_only_trade_count: Option<i64>,
        book_leg_volume_quote: Option<BigDecimal>,
        pool_leg_volume_quote: Option<BigDecimal>,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT
           pair_id,
           COUNT(*) FILTER (
             WHERE COALESCE(book_return_amount, 0) > 0
           ) AS hybrid_trade_count,
           COUNT(*) FILTER (
             WHERE book_return_amount IS NULL OR book_return_amount = 0
           ) AS pool_only_trade_count,
           COALESCE(SUM(book_return_amount), 0) AS book_leg_volume_quote,
           COALESCE(SUM(pool_return_amount), 0) AS pool_leg_volume_quote
         FROM swap_events
         WHERE block_timestamp >= $1
         GROUP BY pair_id",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.pair_id,
                HybridVolumeBreakdown {
                    hybrid_trade_count: row.hybrid_trade_count.unwrap_or(0),
                    pool_only_trade_count: row.pool_only_trade_count.unwrap_or(0),
                    book_leg_volume_quote: row.book_leg_volume_quote.unwrap_or_default(),
                    pool_leg_volume_quote: row.pool_leg_volume_quote.unwrap_or_default(),
                },
            )
        })
        .collect())
}

pub async fn trade_exists(
    pool: &PgPool,
    tx_hash: &str,
    pair_id: i32,
    swap_index: i32,
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM swap_events WHERE tx_hash = $1 AND pair_id = $2 AND swap_index = $3",
    )
    .bind(tx_hash)
    .bind(pair_id)
    .bind(swap_index)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
