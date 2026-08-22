//! Protocol fee event insert + ~5 min rollup (GitLab #586).
//!
//! GET `/overview` and GET `/protocol/fees` read `global_stats_24h` / child rollup tables only.
//! `OVERVIEW_GLOBAL_STATS_LIVE=1` must not 60d-SUM `protocol_fee_events` on the request path.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::pair_price_usd::fits_numeric_38_18;
use crate::indexer::protocol_fees::{
    flow_change_pct, FeeEventDraft, FeeSource,
};

const TOKEN_CAP: i64 = 8;

#[derive(Debug, Clone, Default)]
pub struct ProtocolFeeRollup {
    pub total_fees_24h_usd: Option<BigDecimal>,
    pub total_fees_7d_usd: Option<BigDecimal>,
    pub total_fees_30d_usd: Option<BigDecimal>,
    pub fees_change_24h_pct: Option<BigDecimal>,
    pub fees_change_7d_pct: Option<BigDecimal>,
    pub fees_change_30d_pct: Option<BigDecimal>,
    pub fee_event_count_24h: i64,
    pub fee_event_count_7d: i64,
    pub fee_event_count_30d: i64,
    pub wrap_mapper_configured: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeeSourceStatRow {
    pub window: String,
    pub source: String,
    pub amount_usd: Option<BigDecimal>,
    pub event_count: i64,
    pub share_pct: Option<BigDecimal>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeeTokenStatRow {
    pub window: String,
    pub asset_id: Option<i32>,
    pub amount_human: Option<BigDecimal>,
    pub amount_usd: Option<BigDecimal>,
    pub share_pct: Option<BigDecimal>,
    pub rank: i32,
    pub is_other: bool,
    pub symbol: Option<String>,
    pub contract_address: Option<String>,
    pub denom: Option<String>,
}

/// Insert one fee row. `ON CONFLICT DO NOTHING` so poller replay does not double-count.
pub async fn insert_fee_event(pool: &PgPool, draft: &FeeEventDraft) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"INSERT INTO protocol_fee_events
           (block_height, block_timestamp, tx_hash, source, ordinal,
            asset_id, amount_raw, decimals, fee_usd)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (tx_hash, source, ordinal) DO NOTHING"#,
    )
    .bind(draft.block_height)
    .bind(draft.block_timestamp)
    .bind(&draft.tx_hash)
    .bind(draft.source.as_str())
    .bind(draft.ordinal)
    .bind(draft.asset_id)
    .bind(&draft.amount_raw)
    .bind(draft.decimals)
    .bind(draft.fee_usd.as_ref())
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

fn window_usd(count: i64, priced_sum: Option<BigDecimal>) -> Option<BigDecimal> {
    if count <= 0 {
        return Some(BigDecimal::from(0));
    }
    match priced_sum {
        Some(v) if v > BigDecimal::from(0) => Some(v),
        _ => None,
    }
}

fn clamp_usd(v: Option<BigDecimal>) -> Option<BigDecimal> {
    let v = v?;
    if v <= BigDecimal::from(0) || !fits_numeric_38_18(&v) {
        None
    } else {
        Some(v)
    }
}

/// Rebuild fee scalars on `global_stats_24h` and capped breakdown tables.
/// Trailing windows: now−24h / 7d / 30d; prior = previous equal window (V3 / #576).
pub async fn refresh_protocol_fees(
    pool: &PgPool,
    wrap_mapper_configured: bool,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let c24 = now - chrono::Duration::hours(24);
    let c48 = now - chrono::Duration::hours(48);
    let c7 = now - chrono::Duration::days(7);
    let c14 = now - chrono::Duration::days(14);
    let c30 = now - chrono::Duration::days(30);
    let c60 = now - chrono::Duration::days(60);

    #[derive(FromRow)]
    struct Agg {
        n24: i64,
        usd24: Option<BigDecimal>,
        n7: i64,
        usd7: Option<BigDecimal>,
        n30: i64,
        usd30: Option<BigDecimal>,
        n24_prior: i64,
        usd24_prior: Option<BigDecimal>,
        n7_prior: i64,
        usd7_prior: Option<BigDecimal>,
        n30_prior: i64,
        usd30_prior: Option<BigDecimal>,
    }

    let agg = sqlx::query_as::<_, Agg>(
        r#"SELECT
            COUNT(*) FILTER (WHERE block_timestamp >= $1) AS n24,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $1 AND fee_usd IS NOT NULL) AS usd24,
            COUNT(*) FILTER (WHERE block_timestamp >= $2) AS n7,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $2 AND fee_usd IS NOT NULL) AS usd7,
            COUNT(*) FILTER (WHERE block_timestamp >= $3) AS n30,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $3 AND fee_usd IS NOT NULL) AS usd30,
            COUNT(*) FILTER (WHERE block_timestamp >= $4 AND block_timestamp < $1) AS n24_prior,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $4 AND block_timestamp < $1 AND fee_usd IS NOT NULL) AS usd24_prior,
            COUNT(*) FILTER (WHERE block_timestamp >= $5 AND block_timestamp < $2) AS n7_prior,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $5 AND block_timestamp < $2 AND fee_usd IS NOT NULL) AS usd7_prior,
            COUNT(*) FILTER (WHERE block_timestamp >= $6 AND block_timestamp < $3) AS n30_prior,
            SUM(fee_usd) FILTER (WHERE block_timestamp >= $6 AND block_timestamp < $3 AND fee_usd IS NOT NULL) AS usd30_prior
         FROM protocol_fee_events
         WHERE block_timestamp >= $6"#,
    )
    .bind(c24)
    .bind(c7)
    .bind(c30)
    .bind(c48)
    .bind(c14)
    .bind(c60)
    .fetch_one(pool)
    .await?;

    let usd24 = window_usd(agg.n24, clamp_usd(agg.usd24));
    let usd7 = window_usd(agg.n7, clamp_usd(agg.usd7));
    let usd30 = window_usd(agg.n30, clamp_usd(agg.usd30));
    let prior24 = if agg.n24_prior <= 0 {
        None
    } else {
        clamp_usd(agg.usd24_prior)
    };
    let prior7 = if agg.n7_prior <= 0 {
        None
    } else {
        clamp_usd(agg.usd7_prior)
    };
    let prior30 = if agg.n30_prior <= 0 {
        None
    } else {
        clamp_usd(agg.usd30_prior)
    };

    let chg24 = flow_change_pct(usd24.as_ref(), prior24.as_ref());
    let chg7 = flow_change_pct(usd7.as_ref(), prior7.as_ref());
    let chg30 = flow_change_pct(usd30.as_ref(), prior30.as_ref());

    // UPDATE-only so a fee refresh cannot INSERT a volume-zero row (#550/#569 class of bug).
    sqlx::query(
        r#"UPDATE global_stats_24h SET
               total_fees_24h_usd = $1,
               total_fees_7d_usd = $2,
               total_fees_30d_usd = $3,
               fees_change_24h_pct = $4,
               fees_change_7d_pct = $5,
               fees_change_30d_pct = $6,
               fee_event_count_24h = $7,
               fee_event_count_7d = $8,
               fee_event_count_30d = $9,
               wrap_mapper_configured = $10
           WHERE id = 1"#,
    )
    .bind(usd24.as_ref())
    .bind(usd7.as_ref())
    .bind(usd30.as_ref())
    .bind(chg24.as_ref())
    .bind(chg7.as_ref())
    .bind(chg30.as_ref())
    .bind(agg.n24)
    .bind(agg.n7)
    .bind(agg.n30)
    .bind(wrap_mapper_configured)
    .execute(pool)
    .await?;

    refresh_source_breakdown(pool, "24h", c24, wrap_mapper_configured).await?;
    refresh_source_breakdown(pool, "7d", c7, wrap_mapper_configured).await?;
    refresh_source_breakdown(pool, "30d", c30, wrap_mapper_configured).await?;
    refresh_token_breakdown(pool, "24h", c24).await?;
    refresh_token_breakdown(pool, "7d", c7).await?;
    refresh_token_breakdown(pool, "30d", c30).await?;

    Ok(())
}

async fn refresh_source_breakdown(
    pool: &PgPool,
    window: &str,
    cutoff: DateTime<Utc>,
    wrap_mapper_configured: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(r#"DELETE FROM protocol_fee_stats_by_source WHERE "window" = $1"#)
        .bind(window)
        .execute(pool)
        .await?;

    #[derive(FromRow)]
    struct Src {
        source: String,
        event_count: i64,
        amount_usd: Option<BigDecimal>,
    }

    let rows = sqlx::query_as::<_, Src>(
        r#"SELECT source,
                  COUNT(*) AS event_count,
                  SUM(fee_usd) FILTER (WHERE fee_usd IS NOT NULL) AS amount_usd
           FROM protocol_fee_events
           WHERE block_timestamp >= $1
           GROUP BY source"#,
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let priced_total: BigDecimal = rows
        .iter()
        .filter_map(|r| r.amount_usd.clone())
        .fold(BigDecimal::from(0), |a, b| a + b);

    for source in FeeSource::ALL {
        if source.is_wrap_family() && !wrap_mapper_configured {
            continue;
        }
        let found = rows.iter().find(|r| r.source == source.as_str());
        let event_count = found.map(|r| r.event_count).unwrap_or(0);
        let amount_usd = match found {
            None => Some(BigDecimal::from(0)),
            Some(r) if r.event_count <= 0 => Some(BigDecimal::from(0)),
            Some(r) => clamp_usd(r.amount_usd.clone()),
        };
        let share = match (&amount_usd, &priced_total) {
            (Some(usd), tot) if *tot > BigDecimal::from(0) => {
                let pct = usd / tot * BigDecimal::from(100);
                if fits_numeric_38_18(&pct) {
                    Some(pct)
                } else {
                    None
                }
            }
            _ => None,
        };
        sqlx::query(
            r#"INSERT INTO protocol_fee_stats_by_source
               ("window", source, amount_usd, event_count, share_pct, updated_at)
               VALUES ($1, $2, $3, $4, $5, NOW())"#,
        )
        .bind(window)
        .bind(source.as_str())
        .bind(amount_usd.as_ref())
        .bind(event_count)
        .bind(share.as_ref())
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn refresh_token_breakdown(
    pool: &PgPool,
    window: &str,
    cutoff: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(r#"DELETE FROM protocol_fee_stats_by_token WHERE "window" = $1"#)
        .bind(window)
        .execute(pool)
        .await?;

    #[derive(FromRow)]
    struct Tok {
        asset_id: i32,
        amount_human: Option<BigDecimal>,
        amount_usd: Option<BigDecimal>,
    }

    let rows = sqlx::query_as::<_, Tok>(
        r#"SELECT asset_id,
                  SUM(amount_raw / POWER(10::numeric, decimals))
                    FILTER (WHERE decimals BETWEEN 0 AND 38) AS amount_human,
                  SUM(fee_usd) FILTER (WHERE fee_usd IS NOT NULL) AS amount_usd
           FROM protocol_fee_events
           WHERE block_timestamp >= $1
           GROUP BY asset_id
           ORDER BY amount_usd DESC NULLS LAST, asset_id ASC"#,
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let priced_total: BigDecimal = rows
        .iter()
        .filter_map(|r| r.amount_usd.clone())
        .fold(BigDecimal::from(0), |a, b| a + b);

    let (head, tail) = if rows.len() as i64 > TOKEN_CAP {
        rows.split_at(TOKEN_CAP as usize)
    } else {
        (&rows[..], &[][..])
    };

    let mut rank = 1i32;
    for row in head {
        let amount_usd = clamp_usd(row.amount_usd.clone());
        let share = match (&amount_usd, &priced_total) {
            (Some(usd), tot) if *tot > BigDecimal::from(0) => {
                let pct = usd / tot * BigDecimal::from(100);
                if fits_numeric_38_18(&pct) {
                    Some(pct)
                } else {
                    None
                }
            }
            _ => None,
        };
        sqlx::query(
            r#"INSERT INTO protocol_fee_stats_by_token
               ("window", asset_id, amount_human, amount_usd, share_pct, rank, is_other, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, FALSE, NOW())"#,
        )
        .bind(window)
        .bind(row.asset_id)
        .bind(row.amount_human.as_ref())
        .bind(amount_usd.as_ref())
        .bind(share.as_ref())
        .bind(rank)
        .execute(pool)
        .await?;
        rank += 1;
    }

    if !tail.is_empty() {
        let other_usd: BigDecimal = tail
            .iter()
            .filter_map(|r| r.amount_usd.clone())
            .fold(BigDecimal::from(0), |a, b| a + b);
        let amount_usd = clamp_usd(Some(other_usd.clone()));
        let share = match (&amount_usd, &priced_total) {
            (Some(usd), tot) if *tot > BigDecimal::from(0) => {
                let pct = usd / tot * BigDecimal::from(100);
                if fits_numeric_38_18(&pct) {
                    Some(pct)
                } else {
                    None
                }
            }
            _ => None,
        };
        sqlx::query(
            r#"INSERT INTO protocol_fee_stats_by_token
               ("window", asset_id, amount_human, amount_usd, share_pct, rank, is_other, updated_at)
               VALUES ($1, NULL, NULL, $2, $3, $4, TRUE, NOW())"#,
        )
        .bind(window)
        .bind(amount_usd.as_ref())
        .bind(share.as_ref())
        .bind(rank)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_fee_rollup(pool: &PgPool) -> Result<ProtocolFeeRollup, sqlx::Error> {
    #[derive(FromRow)]
    struct Row {
        total_fees_24h_usd: Option<BigDecimal>,
        total_fees_7d_usd: Option<BigDecimal>,
        total_fees_30d_usd: Option<BigDecimal>,
        fees_change_24h_pct: Option<BigDecimal>,
        fees_change_7d_pct: Option<BigDecimal>,
        fees_change_30d_pct: Option<BigDecimal>,
        fee_event_count_24h: i64,
        fee_event_count_7d: i64,
        fee_event_count_30d: i64,
        wrap_mapper_configured: bool,
    }

    let row = sqlx::query_as::<_, Row>(
        r#"SELECT total_fees_24h_usd, total_fees_7d_usd, total_fees_30d_usd,
                  fees_change_24h_pct, fees_change_7d_pct, fees_change_30d_pct,
                  fee_event_count_24h, fee_event_count_7d, fee_event_count_30d,
                  wrap_mapper_configured
           FROM global_stats_24h WHERE id = 1"#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => ProtocolFeeRollup {
            total_fees_24h_usd: r.total_fees_24h_usd,
            total_fees_7d_usd: r.total_fees_7d_usd,
            total_fees_30d_usd: r.total_fees_30d_usd,
            fees_change_24h_pct: r.fees_change_24h_pct,
            fees_change_7d_pct: r.fees_change_7d_pct,
            fees_change_30d_pct: r.fees_change_30d_pct,
            fee_event_count_24h: r.fee_event_count_24h,
            fee_event_count_7d: r.fee_event_count_7d,
            fee_event_count_30d: r.fee_event_count_30d,
            wrap_mapper_configured: r.wrap_mapper_configured,
        },
        None => ProtocolFeeRollup::default(),
    })
}

pub fn parse_fee_window(raw: Option<&str>) -> Result<&'static str, ()> {
    match raw.unwrap_or("24h") {
        "24h" => Ok("24h"),
        "7d" => Ok("7d"),
        "30d" => Ok("30d"),
        _ => Err(()),
    }
}

pub async fn get_fees_by_source(
    pool: &PgPool,
    window: &str,
) -> Result<Vec<FeeSourceStatRow>, sqlx::Error> {
    sqlx::query_as::<_, FeeSourceStatRow>(
        r#"SELECT "window", source, amount_usd, event_count, share_pct
           FROM protocol_fee_stats_by_source
           WHERE "window" = $1
           ORDER BY source"#,
    )
    .bind(window)
    .fetch_all(pool)
    .await
}

pub async fn get_fees_by_token(
    pool: &PgPool,
    window: &str,
) -> Result<Vec<FeeTokenStatRow>, sqlx::Error> {
    sqlx::query_as::<_, FeeTokenStatRow>(
        r#"SELECT t."window", t.asset_id, t.amount_human, t.amount_usd, t.share_pct,
                  t.rank, t.is_other,
                  a.symbol, a.contract_address, a.denom
           FROM protocol_fee_stats_by_token t
           LEFT JOIN assets a ON a.id = t.asset_id
           WHERE t."window" = $1
           ORDER BY t.rank"#,
    )
    .bind(window)
    .fetch_all(pool)
    .await
}
