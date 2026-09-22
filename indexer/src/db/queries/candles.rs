use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct CandleRow {
    pub id: i64,
    pub pair_id: i32,
    pub interval: String,
    pub open_time: DateTime<Utc>,
    /// Subject USD when present. NULL on a neither-catalog row.
    /// GET `/candles` does not publish this as `open` when `usd_leg = asset_1`.
    pub open: Option<BigDecimal>,
    pub high: Option<BigDecimal>,
    pub low: Option<BigDecimal>,
    pub close: Option<BigDecimal>,
    /// `asset_0` / `asset_1` at write. NULL means the USD columns are USD of `asset_0`
    /// when they are non-null, or a neither-catalog row when they are null.
    pub usd_leg: Option<String>,
    pub open_human: Option<BigDecimal>,
    pub high_human: Option<BigDecimal>,
    pub low_human: Option<BigDecimal>,
    pub close_human: Option<BigDecimal>,
    pub volume_base: BigDecimal,
    pub volume_quote: BigDecimal,
    pub trade_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_candle(
    conn: &mut sqlx::PgConnection,
    pair_id: i32,
    interval: &str,
    open_time: DateTime<Utc>,
    open: Option<&BigDecimal>,
    high: Option<&BigDecimal>,
    low: Option<&BigDecimal>,
    close: Option<&BigDecimal>,
    usd_leg: Option<&str>,
    open_human: Option<&BigDecimal>,
    high_human: Option<&BigDecimal>,
    low_human: Option<&BigDecimal>,
    close_human: Option<&BigDecimal>,
    vol_base: &BigDecimal,
    vol_quote: &BigDecimal,
    count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close, usd_leg,
                             open_human, high_human, low_human, close_human,
                             volume_base, volume_quote, trade_count)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         ON CONFLICT (pair_id, interval, open_time)
           DO UPDATE SET open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
                        close = EXCLUDED.close, usd_leg = EXCLUDED.usd_leg,
                        open_human = EXCLUDED.open_human, high_human = EXCLUDED.high_human,
                        low_human = EXCLUDED.low_human, close_human = EXCLUDED.close_human,
                        volume_base = EXCLUDED.volume_base,
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
    .bind(usd_leg)
    .bind(open_human)
    .bind(high_human)
    .bind(low_human)
    .bind(close_human)
    .bind(vol_base)
    .bind(vol_quote)
    .bind(count)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Latest `limit` bars in `[from, to]`, returned oldest→newest (GitLab #705).
///
/// Retail charts need **now**, not the start of the 90-day window. Fine intervals
/// (`1m`/`5m`/`15m`) are dense after idle marks (#568), so `ORDER BY open_time ASC
/// LIMIT n` previously returned the oldest N and dropped the current bucket.
pub async fn get_candles(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<CandleRow>, sqlx::Error> {
    sqlx::query_as::<_, CandleRow>(
        "SELECT * FROM (
             SELECT * FROM candles
              WHERE pair_id = $1 AND interval = $2 AND open_time >= $3 AND open_time <= $4
              ORDER BY open_time DESC
              LIMIT $5
         ) AS newest
         ORDER BY open_time ASC",
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
        _ => {
            return Err(sqlx::Error::Protocol(format!(
                "Invalid candle interval: {}",
                interval
            )))
        }
    };

    // USD OHLC from price_usd only (no human fallback). Human OHLC from se.price.
    // Buckets with no positive price_usd are omitted so GET /candles never returns
    // quote-per-base on columns labeled as factory USD (GitLab #543 / P522-5).
    let sql = format!(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                             open_human, high_human, low_human, close_human,
                             volume_base, volume_quote, trade_count)
         SELECT
           $1 AS pair_id,
           $2 AS interval,
           date_trunc('minute', se.block_timestamp) -
             (EXTRACT(MINUTE FROM se.block_timestamp)::int %
              EXTRACT(EPOCH FROM interval '{}')::int / 60) * interval '1 minute' AS open_time,
           (array_agg(se.price_usd ORDER BY se.block_timestamp ASC, se.id ASC))[1] AS open,
           MAX(se.price_usd) AS high,
           MIN(se.price_usd) AS low,
           (array_agg(se.price_usd ORDER BY se.block_timestamp DESC, se.id DESC))[1] AS close,
           (array_agg(se.price ORDER BY se.block_timestamp ASC, se.id ASC))[1] AS open_human,
           MAX(se.price) AS high_human,
           MIN(se.price) AS low_human,
           (array_agg(se.price ORDER BY se.block_timestamp DESC, se.id DESC))[1] AS close_human,
           SUM(CASE WHEN se.offer_asset_id = p.asset_0_id THEN se.offer_amount ELSE se.return_amount END) AS volume_base,
           SUM(CASE WHEN se.offer_asset_id = p.asset_0_id THEN se.return_amount ELSE se.offer_amount END) AS volume_quote,
           COUNT(*)::int AS trade_count
         FROM swap_events se
         INNER JOIN pairs p ON p.id = se.pair_id
         WHERE se.pair_id = $1 AND se.block_timestamp >= $3
           AND se.price_usd IS NOT NULL AND se.price_usd > 0 AND se.price > 0
         GROUP BY open_time
         ON CONFLICT (pair_id, interval, open_time)
           DO UPDATE SET open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
                        close = EXCLUDED.close, usd_leg = NULL,
                        open_human = EXCLUDED.open_human, high_human = EXCLUDED.high_human,
                        low_human = EXCLUDED.low_human, close_human = EXCLUDED.close_human,
                        volume_base = EXCLUDED.volume_base,
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

/// JSON fields for GET `/api/v1/pairs/{addr}/candles`.
///
/// `open/high/low/close` stay USD of `asset_0`. When `usd_leg = asset_1`, those four
/// fields are omitted and the subject OHLC is `subject_*` so a cached client drops the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandleApiFields {
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: Option<String>,
    pub usd_leg: Option<String>,
    pub subject_open: Option<String>,
    pub subject_high: Option<String>,
    pub subject_low: Option<String>,
    pub subject_close: Option<String>,
}

fn bd_string(v: &BigDecimal) -> String {
    v.to_string()
}

fn ohlc_strings(
    open: &BigDecimal,
    high: &BigDecimal,
    low: &BigDecimal,
    close: &BigDecimal,
) -> (String, String, String, String) {
    (bd_string(open), bd_string(high), bd_string(low), bd_string(close))
}

pub fn project_candle_api(row: &CandleRow) -> CandleApiFields {
    let complete = match (&row.open, &row.high, &row.low, &row.close) {
        (Some(o), Some(h), Some(l), Some(c)) => Some(ohlc_strings(o, h, l, c)),
        _ => None,
    };
    if row.usd_leg.as_deref() == Some("asset_1") {
        let (subject_open, subject_high, subject_low, subject_close) = match complete {
            Some((o, h, l, c)) => (Some(o), Some(h), Some(l), Some(c)),
            None => (None, None, None, None),
        };
        return CandleApiFields {
            open: None,
            high: None,
            low: None,
            close: None,
            usd_leg: Some("asset_1".to_string()),
            subject_open,
            subject_high,
            subject_low,
            subject_close,
        };
    }
    if let Some((open, high, low, close)) = complete {
        return CandleApiFields {
            open: Some(open),
            high: Some(high),
            low: Some(low),
            close: Some(close),
            usd_leg: row
                .usd_leg
                .as_deref()
                .filter(|leg| *leg == "asset_0")
                .map(|leg| leg.to_string()),
            subject_open: None,
            subject_high: None,
            subject_low: None,
            subject_close: None,
        };
    }
    CandleApiFields {
        open: None,
        high: None,
        low: None,
        close: None,
        usd_leg: None,
        subject_open: None,
        subject_high: None,
        subject_low: None,
        subject_close: None,
    }
}

#[cfg(test)]
mod project_tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::Utc;
    use std::str::FromStr;

    fn row(open: Option<&str>, usd_leg: Option<&str>) -> CandleRow {
        let bd = |s: &str| BigDecimal::from_str(s).unwrap();
        let usd = open.map(bd);
        CandleRow {
            id: 1,
            pair_id: 1,
            interval: "1h".to_string(),
            open_time: Utc::now(),
            open: usd.clone(),
            high: usd.clone(),
            low: usd.clone(),
            close: usd,
            usd_leg: usd_leg.map(|s| s.to_string()),
            open_human: Some(bd("2")),
            high_human: Some(bd("2")),
            low_human: Some(bd("2")),
            close_human: Some(bd("2")),
            volume_base: bd("1"),
            volume_quote: bd("1"),
            trade_count: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn asset_1_omits_factory_ohlc_and_sends_subject() {
        let fields = project_candle_api(&row(Some("1.5"), Some("asset_1")));
        assert!(fields.open.is_none());
        assert!(fields.high.is_none());
        assert!(fields.low.is_none());
        assert!(fields.close.is_none());
        assert_eq!(fields.usd_leg.as_deref(), Some("asset_1"));
        assert_eq!(fields.subject_open.as_deref(), Some("1.5"));
        assert_eq!(fields.subject_close.as_deref(), Some("1.5"));
    }

    #[test]
    fn asset_0_and_legacy_keep_factory_ohlc() {
        let legacy = project_candle_api(&row(Some("1.5"), None));
        assert_eq!(legacy.open.as_deref(), Some("1.5"));
        assert!(legacy.usd_leg.is_none());
        assert!(legacy.subject_open.is_none());
        let tagged = project_candle_api(&row(Some("1.5"), Some("asset_0")));
        assert_eq!(tagged.open.as_deref(), Some("1.5"));
        assert_eq!(tagged.usd_leg.as_deref(), Some("asset_0"));
        assert!(tagged.subject_open.is_none());
    }

    #[test]
    fn neither_catalog_omits_usd() {
        let fields = project_candle_api(&row(None, None));
        assert!(fields.open.is_none());
        assert!(fields.usd_leg.is_none());
        assert!(fields.subject_open.is_none());
    }
}
