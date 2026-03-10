use bigdecimal::BigDecimal;
use chrono::{DateTime, Timelike, Utc};
use sqlx::PgPool;

use crate::db::queries::candles::{self, CandleRow};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d"];

pub async fn update_candles_for_swap(
    pool: &PgPool,
    pair_id: i32,
    timestamp: DateTime<Utc>,
    price: &BigDecimal,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> Result<(), BoxError> {
    for &interval in INTERVALS {
        let open_time = truncate_to_interval(timestamp, interval);

        let existing = get_candle_at(pool, pair_id, interval, open_time).await?;

        let (open, high, low, close, vol_base, vol_quote, count) = match existing {
            Some(candle) => {
                let high = if price > &candle.high {
                    price.clone()
                } else {
                    candle.high
                };
                let low = if price < &candle.low {
                    price.clone()
                } else {
                    candle.low
                };
                (
                    candle.open,
                    high,
                    low,
                    price.clone(),
                    candle.volume_base + offer_amount,
                    candle.volume_quote + return_amount,
                    candle.trade_count + 1,
                )
            }
            None => (
                price.clone(),
                price.clone(),
                price.clone(),
                price.clone(),
                offer_amount.clone(),
                return_amount.clone(),
                1,
            ),
        };

        candles::upsert_candle(
            pool, pair_id, interval, open_time, &open, &high, &low, &close, &vol_base, &vol_quote,
            count,
        )
        .await?;
    }

    Ok(())
}

pub fn interval_seconds(interval: &str) -> i64 {
    match interval {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "4h" => 14400,
        "1d" => 86400,
        _ => 60,
    }
}

pub fn truncate_to_interval(ts: DateTime<Utc>, interval: &str) -> DateTime<Utc> {
    let zeroed = ts.with_nanosecond(0).unwrap();
    match interval {
        "1m" => zeroed.with_second(0).unwrap(),
        "5m" => {
            let m = zeroed.minute();
            zeroed
                .with_minute(m - (m % 5))
                .unwrap()
                .with_second(0)
                .unwrap()
        }
        "15m" => {
            let m = zeroed.minute();
            zeroed
                .with_minute(m - (m % 15))
                .unwrap()
                .with_second(0)
                .unwrap()
        }
        "1h" => zeroed.with_minute(0).unwrap().with_second(0).unwrap(),
        "4h" => {
            let h = zeroed.hour();
            zeroed
                .with_hour(h - (h % 4))
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
        }
        "1d" => zeroed
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap(),
        _ => zeroed,
    }
}

async fn get_candle_at(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    open_time: DateTime<Utc>,
) -> Result<Option<CandleRow>, sqlx::Error> {
    sqlx::query_as::<_, CandleRow>(
        "SELECT * FROM candles WHERE pair_id = $1 AND interval = $2 AND open_time = $3",
    )
    .bind(pair_id)
    .bind(interval)
    .bind(open_time)
    .fetch_optional(pool)
    .await
}
