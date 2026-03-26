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

#[allow(clippy::too_many_arguments)]
pub async fn insert_swap(
    pool: &PgPool,
    pair_id: i32,
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
          pool_return_amount, book_return_amount, limit_book_offer_consumed)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
         ON CONFLICT (tx_hash, pair_id) DO NOTHING
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
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<SwapEventRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
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
        None => {
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

    let price_change_pct = match (&open, &close) {
        (Some(o), Some(c)) => {
            use bigdecimal::ToPrimitive;
            let o_f = o.price.to_f64().unwrap_or(0.0);
            let c_f = c.price.to_f64().unwrap_or(0.0);
            if o_f != 0.0 {
                Some(((c_f - o_f) / o_f) * 100.0)
            } else {
                None
            }
        }
        _ => None,
    };

    Ok(PairStats {
        volume_base: stats.volume_base.unwrap_or_default(),
        volume_quote: stats.volume_quote.unwrap_or_default(),
        volume_usd: stats.volume_usd,
        trade_count: stats.trade_count.unwrap_or(0),
        high: stats.high,
        low: stats.low,
        open_price: open.map(|r| r.price),
        close_price: close.map(|r| r.price),
        price_change_pct,
    })
}

pub async fn trade_exists(pool: &PgPool, tx_hash: &str, pair_id: i32) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM swap_events WHERE tx_hash = $1 AND pair_id = $2",
    )
    .bind(tx_hash)
    .bind(pair_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
