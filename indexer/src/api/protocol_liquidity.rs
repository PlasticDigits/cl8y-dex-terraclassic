//! `GET /api/v1/protocol/liquidity/daily` — UTC Protocol liquidity **stock** (GitLab #689).
//!
//! `grain=hourly|daily|monthly` + integer `limit` capped per grain (same as volume).
//! `from` / `to` / `window` / `days` / `metric` / `ticker` → **400**. 60s cache keyed by
//! allowlisted `(grain, limit)` only. Reads grain tables — never snapshots / pair_reserves
//! / Llama. Missing bucket → JSON `null` (unknown ≠ `$0`). Newest-last.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration as ChronoDuration, Months, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::protocol_liquidity as liq_q;
use crate::db::queries::protocol_volume as vol_q;
use crate::indexer::defillama::utc_day_start;

const CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct LiquidityCacheKey {
    grain: vol_q::VolumeGrain,
    limit: i32,
}

fn liquidity_cache()
-> &'static Mutex<HashMap<LiquidityCacheKey, (ProtocolLiquidityDailyResponse, Instant)>> {
    static CACHE: OnceLock<
        Mutex<HashMap<LiquidityCacheKey, (ProtocolLiquidityDailyResponse, Instant)>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize)]
pub struct ProtocolLiquidityDailyQuery {
    pub grain: Option<String>,
    pub limit: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub window: Option<String>,
    pub days: Option<String>,
    pub metric: Option<String>,
    pub ticker: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolLiquidityDailyPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_hour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_month: Option<String>,
    /// Last snapshot USD in the bucket. JSON `null` when no sample (not `"0"`).
    pub liquidity_usd: Option<String>,
    pub priced_pair_count: i32,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolLiquidityDailyResponse {
    pub grain: String,
    pub limit: i32,
    pub timezone: String,
    /// Protocol catalog pool TVL (P569). Not DeFiLlama / CG `liquidity_in_usd`.
    pub methodology: String,
    pub series: Vec<ProtocolLiquidityDailyPoint>,
}

/// Drop the 60s liquidity-series response cache (tests).
pub fn reset_protocol_liquidity_cache() {
    if let Ok(mut guard) = liquidity_cache().lock() {
        guard.clear();
    }
}

fn bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

fn reject_disallowed(q: &ProtocolLiquidityDailyQuery) -> Result<(), (StatusCode, String)> {
    if q.from.is_some() || q.to.is_some() {
        return Err(bad_request("from/to are not allowed"));
    }
    if q.window.is_some() || q.days.is_some() || q.metric.is_some() || q.ticker.is_some() {
        return Err(bad_request("window/days/metric/ticker are not allowed"));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/protocol/liquidity/daily",
    params(
        ("grain" = Option<String>, Query, description = "hourly | daily | monthly"),
        ("limit" = Option<String>, Query, description = "Point count, capped per grain")
    ),
    responses(
        (status = 200, description = "UTC Protocol liquidity stock series", body = ProtocolLiquidityDailyResponse),
        (status = 400, description = "Invalid grain / limit / disallowed query"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_protocol_liquidity_daily(
    State(state): State<AppState>,
    Query(q): Query<ProtocolLiquidityDailyQuery>,
) -> Result<Json<ProtocolLiquidityDailyResponse>, (StatusCode, String)> {
    reject_disallowed(&q)?;
    let grain = vol_q::parse_volume_grain(q.grain.as_deref())
        .map_err(|_| bad_request("Invalid grain, expected hourly, daily, or monthly"))?;
    let limit = vol_q::parse_volume_limit(q.limit.as_deref(), grain)
        .map_err(|_| bad_request("Invalid limit for grain"))?;
    let key = LiquidityCacheKey { grain, limit };

    if let Ok(guard) = liquidity_cache().lock() {
        if let Some((resp, at)) = guard.get(&key) {
            if Instant::now().duration_since(*at) <= CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let resp = match grain {
        vol_q::VolumeGrain::Daily => build_daily_series(&state.pool, grain, limit).await?,
        vol_q::VolumeGrain::Hourly => build_hourly_series(&state.pool, grain, limit).await?,
        vol_q::VolumeGrain::Monthly => build_monthly_series(&state.pool, grain, limit).await?,
    };

    if let Ok(mut guard) = liquidity_cache().lock() {
        guard.insert(key, (resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}

fn wrap(
    grain: vol_q::VolumeGrain,
    limit: i32,
    series: Vec<ProtocolLiquidityDailyPoint>,
) -> ProtocolLiquidityDailyResponse {
    ProtocolLiquidityDailyResponse {
        grain: grain.as_str().to_string(),
        limit,
        timezone: "UTC".to_string(),
        methodology: "protocol_catalog".to_string(),
        series,
    }
}

async fn build_daily_series(
    pool: &sqlx::PgPool,
    grain: vol_q::VolumeGrain,
    limit: i32,
) -> Result<ProtocolLiquidityDailyResponse, (StatusCode, String)> {
    let today = utc_day_start(Utc::now()).date_naive();
    let from = today - ChronoDuration::days(i64::from(limit) - 1);
    let rows = liq_q::get_daily_rows(pool, from, today)
        .await
        .map_err(internal_err)?;
    let mut by_day = HashMap::new();
    for row in rows {
        by_day.insert(row.utc_day, row);
    }
    let mut series = Vec::with_capacity(limit as usize);
    for offset in (0..limit).rev() {
        let day = today - ChronoDuration::days(i64::from(offset));
        let point = match by_day.get(&day) {
            Some(row) => ProtocolLiquidityDailyPoint {
                utc_hour: None,
                utc_day: Some(day.to_string()),
                utc_month: None,
                liquidity_usd: row.liquidity_usd.as_ref().map(|v| v.to_string()),
                priced_pair_count: row.priced_pair_count,
            },
            None => ProtocolLiquidityDailyPoint {
                utc_hour: None,
                utc_day: Some(day.to_string()),
                utc_month: None,
                liquidity_usd: None,
                priced_pair_count: 0,
            },
        };
        series.push(point);
    }
    Ok(wrap(grain, limit, series))
}

async fn build_hourly_series(
    pool: &sqlx::PgPool,
    grain: vol_q::VolumeGrain,
    limit: i32,
) -> Result<ProtocolLiquidityDailyResponse, (StatusCode, String)> {
    let now_hour = vol_q::utc_hour_start(Utc::now());
    let from = now_hour - ChronoDuration::hours(i64::from(limit) - 1);
    let rows = liq_q::get_hourly_rows(pool, from, now_hour)
        .await
        .map_err(internal_err)?;
    let mut by_hour = HashMap::new();
    for row in rows {
        by_hour.insert(row.utc_hour, row);
    }
    let mut series = Vec::with_capacity(limit as usize);
    for offset in (0..limit).rev() {
        let hour = now_hour - ChronoDuration::hours(i64::from(offset));
        let point = match by_hour.get(&hour) {
            Some(row) => ProtocolLiquidityDailyPoint {
                utc_hour: Some(vol_q::format_utc_hour(hour)),
                utc_day: None,
                utc_month: None,
                liquidity_usd: row.liquidity_usd.as_ref().map(|v| v.to_string()),
                priced_pair_count: row.priced_pair_count,
            },
            None => ProtocolLiquidityDailyPoint {
                utc_hour: Some(vol_q::format_utc_hour(hour)),
                utc_day: None,
                utc_month: None,
                liquidity_usd: None,
                priced_pair_count: 0,
            },
        };
        series.push(point);
    }
    Ok(wrap(grain, limit, series))
}

async fn build_monthly_series(
    pool: &sqlx::PgPool,
    grain: vol_q::VolumeGrain,
    limit: i32,
) -> Result<ProtocolLiquidityDailyResponse, (StatusCode, String)> {
    let this_month = vol_q::utc_month_start(Utc::now());
    let from = this_month
        .checked_sub_months(Months::new((limit as u32).saturating_sub(1)))
        .expect("month window in range");
    let rows = liq_q::get_monthly_rows(pool, from, this_month)
        .await
        .map_err(internal_err)?;
    let mut by_month = HashMap::new();
    for row in rows {
        by_month.insert(row.utc_month, row);
    }
    let mut series = Vec::with_capacity(limit as usize);
    for offset in (0..limit).rev() {
        let month = this_month
            .checked_sub_months(Months::new(offset as u32))
            .expect("month offset in range");
        let point = match by_month.get(&month) {
            Some(row) => ProtocolLiquidityDailyPoint {
                utc_hour: None,
                utc_day: None,
                utc_month: Some(vol_q::format_utc_month(month)),
                liquidity_usd: row.liquidity_usd.as_ref().map(|v| v.to_string()),
                priced_pair_count: row.priced_pair_count,
            },
            None => ProtocolLiquidityDailyPoint {
                utc_hour: None,
                utc_day: None,
                utc_month: Some(vol_q::format_utc_month(month)),
                liquidity_usd: None,
                priced_pair_count: 0,
            },
        };
        series.push(point);
    }
    Ok(wrap(grain, limit, series))
}
