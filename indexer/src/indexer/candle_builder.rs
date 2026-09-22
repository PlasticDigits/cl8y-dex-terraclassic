//! OHLC candles from indexed swaps. Non-positive prices skip updates. See `merge_candle_ohlc` tests and `docs/indexer-invariants.md`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Timelike, Utc};
use sqlx::PgPool;

use crate::db::queries::candles::{self, CandleRow};
use crate::indexer::pair_price_usd::{CandleUsdLeg, CandleUsdSubject};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) const INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

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

/// Live candle write (GitLab #543 / #1315).
///
/// `subject` is the catalog decision. `Skip` writes nothing. `HumanOnly` stores
/// `*_human` and NULL USD. USD subjects store that print in `open/high/low/close`
/// plus `usd_leg`. `swap_events.price_usd` is unchanged by this function.
pub async fn update_candles_for_swap(
    pool: &PgPool,
    pair_id: i32,
    timestamp: DateTime<Utc>,
    subject: &CandleUsdSubject,
    price_human: &BigDecimal,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> Result<(), BoxError> {
    let mut conn = pool.acquire().await?;
    update_candles_for_swap_conn(
        &mut conn,
        pair_id,
        timestamp,
        subject,
        price_human,
        offer_amount,
        return_amount,
    )
    .await
}

pub(crate) async fn update_candles_for_swap_conn(
    conn: &mut sqlx::PgConnection,
    pair_id: i32,
    timestamp: DateTime<Utc>,
    subject: &CandleUsdSubject,
    price_human: &BigDecimal,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> Result<(), BoxError> {
    let zero = BigDecimal::from(0);
    let (usd_price, usd_leg) = match subject {
        CandleUsdSubject::Skip => {
            tracing::debug!(pair_id, mode = "skip", "candle_write");
            return Ok(());
        }
        CandleUsdSubject::HumanOnly => {
            if price_human <= &zero {
                tracing::debug!(pair_id, mode = "skip", "candle_write");
                return Ok(());
            }
            tracing::debug!(pair_id, mode = "human_only", "candle_write");
            (None, None)
        }
        CandleUsdSubject::Usd { leg, usd } => {
            if usd <= &zero {
                tracing::debug!(pair_id, mode = "skip", "candle_write");
                return Ok(());
            }
            tracing::debug!(pair_id, mode = subject.mode_str(), "candle_write");
            (Some(usd), Some(leg.as_str()))
        }
    };
    let human = if price_human > &zero {
        Some(price_human)
    } else {
        None
    };

    for &interval in INTERVALS {
        let open_time = truncate_to_interval(timestamp, interval);
        let existing = get_candle_at_conn(conn, pair_id, interval, open_time).await?;
        let merged = merge_swap_into_candle(existing.as_ref(), usd_price, usd_leg, human, offer_amount, return_amount);
        candles::upsert_candle(
            conn,
            pair_id,
            interval,
            open_time,
            merged.open.as_ref(),
            merged.high.as_ref(),
            merged.low.as_ref(),
            merged.close.as_ref(),
            merged.usd_leg.as_deref(),
            merged.open_h.as_ref(),
            merged.high_h.as_ref(),
            merged.low_h.as_ref(),
            merged.close_h.as_ref(),
            &merged.vol_base,
            &merged.vol_quote,
            merged.count,
        )
        .await?;
    }

    Ok(())
}

struct MergedCandle {
    open: Option<BigDecimal>,
    high: Option<BigDecimal>,
    low: Option<BigDecimal>,
    close: Option<BigDecimal>,
    usd_leg: Option<String>,
    open_h: Option<BigDecimal>,
    high_h: Option<BigDecimal>,
    low_h: Option<BigDecimal>,
    close_h: Option<BigDecimal>,
    vol_base: BigDecimal,
    vol_quote: BigDecimal,
    count: i32,
}

fn stored_usd_leg(candle: &CandleRow) -> Option<&'static str> {
    match candle.usd_leg.as_deref() {
        Some("asset_1") => Some("asset_1"),
        Some("asset_0") => Some("asset_0"),
        _ if candle.open.is_some() => Some("asset_0"),
        _ => None,
    }
}

fn merge_swap_into_candle(
    existing: Option<&CandleRow>,
    usd_price: Option<&BigDecimal>,
    usd_leg: Option<&str>,
    human: Option<&BigDecimal>,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> MergedCandle {
    let Some(candle) = existing else {
        let (open, high, low, close) = match usd_price {
            Some(price) => (
                Some(price.clone()),
                Some(price.clone()),
                Some(price.clone()),
                Some(price.clone()),
            ),
            None => (None, None, None, None),
        };
        return MergedCandle {
            open,
            high,
            low,
            close,
            usd_leg: usd_leg.map(|s| s.to_string()),
            open_h: human.cloned(),
            high_h: human.cloned(),
            low_h: human.cloned(),
            close_h: human.cloned(),
            vol_base: offer_amount.clone(),
            vol_quote: return_amount.clone(),
            count: 1,
        };
    };

    let (open_h, high_h, low_h, close_h) = merge_human_ohlc(human, candle);
    let kept = MergedCandle {
        open: candle.open.clone(),
        high: candle.high.clone(),
        low: candle.low.clone(),
        close: candle.close.clone(),
        usd_leg: candle
            .usd_leg
            .clone()
            .or_else(|| stored_usd_leg(candle).map(|s| s.to_string())),
        open_h: open_h.clone(),
        high_h: high_h.clone(),
        low_h: low_h.clone(),
        close_h: close_h.clone(),
        vol_base: &candle.volume_base + offer_amount,
        vol_quote: &candle.volume_quote + return_amount,
        count: candle.trade_count + 1,
    };

    let Some(price) = usd_price else {
        return kept;
    };
    let new_leg = usd_leg.unwrap_or("asset_0");
    match stored_usd_leg(candle) {
        Some(stored) if stored == new_leg => {
            let (open, high, low, close) = match (&candle.open, &candle.high, &candle.low) {
                (Some(o), Some(h), Some(l)) => merge_candle_ohlc(price, o, h, l),
                _ => (
                    price.clone(),
                    price.clone(),
                    price.clone(),
                    price.clone(),
                ),
            };
            MergedCandle {
                open: Some(open),
                high: Some(high),
                low: Some(low),
                close: Some(close),
                usd_leg: Some(new_leg.to_string()),
                open_h,
                high_h,
                low_h,
                close_h,
                vol_base: kept.vol_base,
                vol_quote: kept.vol_quote,
                count: kept.count,
            }
        }
        // Do not reclassify a stored leg or a human-only bucket.
        _ => kept,
    }
}

/// Idle mark-to-market candle write (GitLab #568).
///
/// Merges factory USD (and, on `trade_count = 0` bars, human) into the **current**
/// bucket. Does **not** increment `trade_count` or `volume_*`. Swap bars keep DEX
/// human OHLC. `open` stays the first print/mark in the bucket.
pub async fn update_candles_for_mark(
    pool: &PgPool,
    pair_id: i32,
    timestamp: DateTime<Utc>,
    price_usd: &BigDecimal,
    price_human: &BigDecimal,
) -> Result<(), sqlx::Error> {
    let zero = BigDecimal::from(0);
    if price_usd <= &zero || price_human <= &zero {
        return Ok(());
    }

    let mut conn = pool.acquire().await?;
    for &interval in INTERVALS {
        let open_time = truncate_to_interval(timestamp, interval);
        let existing = get_candle_at_conn(&mut conn, pair_id, interval, open_time).await?;
        // Idle marks are catalog `asset_1` USD of `asset_0`. Do not merge that print
        // into an `asset_1` subject series or a human-only bucket.
        if let Some(candle) = existing.as_ref() {
            if stored_usd_leg(candle) == Some("asset_1") || candle.open.is_none() {
                continue;
            }
        }

        let (open, high, low, close, open_h, high_h, low_h, close_h, vol_base, vol_quote, count) =
            match existing {
                Some(candle) if candle.trade_count > 0 => {
                    let (o, h, l) = match (&candle.open, &candle.high, &candle.low) {
                        (Some(o), Some(h), Some(l)) => (o.clone(), h.clone(), l.clone()),
                        _ => continue,
                    };
                    let (open, high, low, close) = merge_candle_ohlc(price_usd, &o, &h, &l);
                    (
                        open,
                        high,
                        low,
                        close,
                        candle.open_human,
                        candle.high_human,
                        candle.low_human,
                        candle.close_human,
                        candle.volume_base,
                        candle.volume_quote,
                        candle.trade_count,
                    )
                }
                Some(candle) => {
                    let (o, h, l) = match (&candle.open, &candle.high, &candle.low) {
                        (Some(o), Some(h), Some(l)) => (o.clone(), h.clone(), l.clone()),
                        _ => continue,
                    };
                    let (open, high, low, close) = merge_candle_ohlc(price_usd, &o, &h, &l);
                    let (open_h, high_h, low_h, close_h) =
                        merge_human_ohlc(Some(price_human), &candle);
                    (
                        open,
                        high,
                        low,
                        close,
                        open_h,
                        high_h,
                        low_h,
                        close_h,
                        candle.volume_base,
                        candle.volume_quote,
                        candle.trade_count,
                    )
                }
                None => (
                    price_usd.clone(),
                    price_usd.clone(),
                    price_usd.clone(),
                    price_usd.clone(),
                    Some(price_human.clone()),
                    Some(price_human.clone()),
                    Some(price_human.clone()),
                    Some(price_human.clone()),
                    zero.clone(),
                    zero.clone(),
                    0,
                ),
            };

        candles::upsert_candle(
            &mut conn,
            pair_id,
            interval,
            open_time,
            Some(&open),
            Some(&high),
            Some(&low),
            Some(&close),
            Some(CandleUsdLeg::Asset0.as_str()),
            open_h.as_ref(),
            high_h.as_ref(),
            low_h.as_ref(),
            close_h.as_ref(),
            &vol_base,
            &vol_quote,
            count,
        )
        .await?;
    }

    Ok(())
}

pub(crate) fn merge_human_ohlc(
    price_human: Option<&BigDecimal>,
    candle: &CandleRow,
) -> (
    Option<BigDecimal>,
    Option<BigDecimal>,
    Option<BigDecimal>,
    Option<BigDecimal>,
) {
    let Some(price) = price_human else {
        return (
            candle.open_human.clone(),
            candle.high_human.clone(),
            candle.low_human.clone(),
            candle.close_human.clone(),
        );
    };
    match (&candle.open_human, &candle.high_human, &candle.low_human) {
        (Some(open), Some(high), Some(low)) => {
            let (o, h, l, c) = merge_candle_ohlc(price, open, high, low);
            (Some(o), Some(h), Some(l), Some(c))
        }
        _ => (
            Some(price.clone()),
            Some(price.clone()),
            Some(price.clone()),
            Some(price.clone()),
        ),
    }
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

pub(crate) async fn get_candle_at_conn(
    conn: &mut sqlx::PgConnection,
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
    .fetch_optional(&mut *conn)
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
