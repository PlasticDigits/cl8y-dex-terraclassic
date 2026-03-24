//! OHLC candles from indexed swaps. Non-positive prices skip updates. See `merge_candle_ohlc` tests and `docs/indexer-invariants.md`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Timelike, Utc};
use sqlx::PgPool;

use crate::db::queries::candles::{self, CandleRow};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

/// Merge one trade price into an existing candle OHLC. **Invariant:** `high >= low`, `close == price`,
/// `open` unchanged, `high >= close`, `low <= close`.
pub(crate) fn merge_candle_ohlc(
    price: &BigDecimal,
    existing_open: &BigDecimal,
    existing_high: &BigDecimal,
    existing_low: &BigDecimal,
) -> (BigDecimal, BigDecimal, BigDecimal, BigDecimal) {
    let high = if price > existing_high {
        price.clone()
    } else {
        existing_high.clone()
    };
    let low = if price < existing_low {
        price.clone()
    } else {
        existing_low.clone()
    };
    let close = price.clone();
    (existing_open.clone(), high, low, close)
}

pub async fn update_candles_for_swap(
    pool: &PgPool,
    pair_id: i32,
    timestamp: DateTime<Utc>,
    price: &BigDecimal,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> Result<(), BoxError> {
    let zero = BigDecimal::from(0);
    if price <= &zero {
        tracing::debug!("Skipping candle update for non-positive price");
        return Ok(());
    }

    for &interval in INTERVALS {
        let open_time = truncate_to_interval(timestamp, interval);

        let existing = get_candle_at(pool, pair_id, interval, open_time).await?;

        let (open, high, low, close, vol_base, vol_quote, count) = match existing {
            Some(candle) => {
                let (open, high, low, close) =
                    merge_candle_ohlc(price, &candle.open, &candle.high, &candle.low);
                (
                    open,
                    high,
                    low,
                    close,
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
        "1w" => 604800,
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
        "1w" => {
            let days_since_monday = zeroed.weekday().num_days_from_monday();
            let monday = zeroed - chrono::Duration::days(days_since_monday as i64);
            monday
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;
    use std::str::FromStr;

    #[test]
    fn merge_ohlc_invariants_hold() {
        let p = BigDecimal::from_str("0.95").unwrap();
        let o = BigDecimal::from_str("0.90").unwrap();
        let h = BigDecimal::from_str("0.96").unwrap();
        let l = BigDecimal::from_str("0.88").unwrap();
        let (open, high, low, close) = merge_candle_ohlc(&p, &o, &h, &l);
        assert_eq!(open, o);
        assert_eq!(close, p);
        assert!(high >= low);
        assert!(high >= close);
        assert!(low <= close);
    }

    #[test]
    fn merge_ohlc_new_high_and_new_low() {
        let o = BigDecimal::from_str("1.0").unwrap();
        let h = BigDecimal::from_str("1.1").unwrap();
        let l = BigDecimal::from_str("0.9").unwrap();
        let spike = BigDecimal::from_str("1.5").unwrap();
        let (_, hi, lo, _) = merge_candle_ohlc(&spike, &o, &h, &l);
        assert_eq!(hi, spike);
        assert_eq!(lo, l);

        let dip = BigDecimal::from_str("0.5").unwrap();
        let (_, hi2, lo2, _) = merge_candle_ohlc(&dip, &o, &h, &l);
        assert_eq!(hi2, h);
        assert_eq!(lo2, dip);
    }

    #[test]
    fn truncate_1m_zeros_seconds() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 15, 14, 37, 45).unwrap();
        let t = truncate_to_interval(ts, "1m");
        assert_eq!(t.second(), 0);
        assert_eq!(t.minute(), 37);
    }

    #[test]
    fn truncate_5m_aligns() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 15, 14, 37, 0).unwrap();
        let t = truncate_to_interval(ts, "5m");
        assert_eq!(t.minute(), 35);
    }

    #[test]
    fn truncate_1h_zeros_minutes() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 0).unwrap();
        let t = truncate_to_interval(ts, "1h");
        assert_eq!(t.hour(), 14);
        assert_eq!(t.minute(), 0);
    }

    #[test]
    fn truncate_1w_monday_midnight() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 18, 12, 0, 0).unwrap(); // Wednesday
        let t = truncate_to_interval(ts, "1w");
        assert_eq!(t.weekday(), chrono::Weekday::Mon);
        assert_eq!(t.hour(), 0);
    }
}

#[cfg(test)]
mod merge_ohlc_proptest {
    use super::merge_candle_ohlc;
    use bigdecimal::BigDecimal;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn merge_ohlc_invariants_for_i64_prices(
            mo in any::<i64>(),
            mh in any::<i64>(),
            ml in any::<i64>(),
            mp in any::<i64>(),
        ) {
            let open = BigDecimal::from(mo);
            let high_b = BigDecimal::from(mh);
            let low_b = BigDecimal::from(ml);
            let price = BigDecimal::from(mp);
            let (o, h, l, c) = merge_candle_ohlc(&price, &open, &high_b, &low_b);
            prop_assert_eq!(o, open);
            prop_assert!(h >= l);
            prop_assert!(&h >= &price);
            prop_assert!(&l <= &price);
            prop_assert_eq!(c, price);
        }
    }
}
