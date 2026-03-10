use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct CandleRow {
    pub id: i64,
    pub pair_id: i32,
    pub interval: String,
    pub open_time: DateTime<Utc>,
    pub open: BigDecimal,
    pub high: BigDecimal,
    pub low: BigDecimal,
    pub close: BigDecimal,
    pub volume_base: BigDecimal,
    pub volume_quote: BigDecimal,
    pub trade_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_candle(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    open_time: DateTime<Utc>,
    open: &BigDecimal,
    high: &BigDecimal,
    low: &BigDecimal,
    close: &BigDecimal,
    vol_base: &BigDecimal,
    vol_quote: &BigDecimal,
    count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                             volume_base, volume_quote, trade_count)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (pair_id, interval, open_time)
           DO UPDATE SET open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
                        close = EXCLUDED.close, volume_base = EXCLUDED.volume_base,
                        volume_quote = EXCLUDED.volume_quote, trade_count = EXCLUDED.trade_count,
                        updated_at = NOW()",
    )
    .bind(pair_id)
    .bind(interval)
    .bind(open_time)
    .bind(open)
    .bind(high)
    .bind(low)
    .bind(close)
    .bind(vol_base)
    .bind(vol_quote)
    .bind(count)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_candles(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<CandleRow>, sqlx::Error> {
    sqlx::query_as::<_, CandleRow>(
        "SELECT * FROM candles
         WHERE pair_id = $1 AND interval = $2 AND open_time >= $3 AND open_time <= $4
         ORDER BY open_time ASC
         LIMIT $5",
    )
    .bind(pair_id)
    .bind(interval)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn rebuild_candles_from_swaps(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    from: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let interval_expr = match interval {
        "1m" => "1 minute",
        "5m" => "5 minutes",
        "15m" => "15 minutes",
        "1h" => "1 hour",
        "4h" => "4 hours",
        "1d" => "1 day",
        "1w" => "1 week",
        other => other,
    };

    let sql = format!(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                             volume_base, volume_quote, trade_count)
         SELECT
           $1 AS pair_id,
           $2 AS interval,
           date_trunc('minute', block_timestamp) -
             (EXTRACT(MINUTE FROM block_timestamp)::int %
              EXTRACT(EPOCH FROM interval '{}')::int / 60) * interval '1 minute' AS open_time,
           (array_agg(price ORDER BY block_timestamp ASC, id ASC))[1] AS open,
           MAX(price) AS high,
           MIN(price) AS low,
           (array_agg(price ORDER BY block_timestamp DESC, id DESC))[1] AS close,
           SUM(offer_amount) AS volume_base,
           SUM(return_amount) AS volume_quote,
           COUNT(*)::int AS trade_count
         FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= $3
         GROUP BY open_time
         ON CONFLICT (pair_id, interval, open_time)
           DO UPDATE SET open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
                        close = EXCLUDED.close, volume_base = EXCLUDED.volume_base,
                        volume_quote = EXCLUDED.volume_quote, trade_count = EXCLUDED.trade_count,
                        updated_at = NOW()",
        interval_expr
    );

    sqlx::query(&sql)
        .bind(pair_id)
        .bind(interval)
        .bind(from)
        .execute(pool)
        .await?;
    Ok(())
}
