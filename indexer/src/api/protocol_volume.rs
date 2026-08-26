//! `GET /api/v1/protocol/volume/daily` — UTC Protocol volume series (GitLab #652 / #668).
//!
//! **Alias:** `days=7|30` with no `grain` → UTC-day series (P652-3). Unknown → **400**.
//! **Grain:** `grain=hourly|daily|monthly` + integer `limit` capped per grain
//! (hourly ≤ 168, daily ≤ 90, monthly ≤ 24). `from` / `to` → **400**.
//! 60s cache keyed by allowlisted `(days)` or `(grain, limit)` only — extra junk
//! cannot bust the cache. Reads rollup tables — never `swap_events` / Llama.

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
use crate::db::queries::protocol_volume as vol_q;
use crate::indexer::defillama::utc_day_start;

const DAILY_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum VolumeCacheKey {
    AliasDays(i32),
    Grain { grain: vol_q::VolumeGrain, limit: i32 },
}

fn volume_cache() -> &'static Mutex<HashMap<VolumeCacheKey, (ProtocolVolumeDailyResponse, Instant)>>
{
    static CACHE: OnceLock<
        Mutex<HashMap<VolumeCacheKey, (ProtocolVolumeDailyResponse, Instant)>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize)]
pub struct ProtocolVolumeDailyQuery {
    pub days: Option<String>,
    pub grain: Option<String>,
    pub limit: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolVolumeDailyPoint {
    /// UTC hour start `YYYY-MM-DDTHH` when `grain=hourly`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_hour: Option<String>,
    /// `YYYY-MM-DD` UTC calendar day (alias + `grain=daily`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_day: Option<String>,
    /// UTC calendar month `YYYY-MM` when `grain=monthly`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_month: Option<String>,
    /// `"0"` idle; JSON `null` when the bucket has trades but no priced USD.
    pub volume_usd: Option<String>,
    pub trade_count: i64,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolVolumeDailyResponse {
    /// Present on the `days=7|30` alias (GitLab #652).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i32>,
    /// Present when `grain=` is set (GitLab #668).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    pub timezone: String,
    /// Protocol catalog (overview `total_volume_*_usd`). Not DeFiLlama gem-exclude.
    pub methodology: String,
    pub series: Vec<ProtocolVolumeDailyPoint>,
}

/// Drop the 60s daily-volume response cache (tests).
pub fn reset_protocol_volume_cache() {
    if let Ok(mut guard) = volume_cache().lock() {
        guard.clear();
    }
}

fn bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

#[utoipa::path(
    get,
    path = "/api/v1/protocol/volume/daily",
    params(
        ("days" = Option<String>, Query, description = "UTC-day alias: 7 or 30 (no grain)"),
        ("grain" = Option<String>, Query, description = "hourly | daily | monthly"),
        ("limit" = Option<String>, Query, description = "Point count, capped per grain")
    ),
    responses(
        (status = 200, description = "UTC Protocol volume series", body = ProtocolVolumeDailyResponse),
        (status = 400, description = "Invalid days / grain / limit"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_protocol_volume_daily(
    State(state): State<AppState>,
    Query(q): Query<ProtocolVolumeDailyQuery>,
) -> Result<Json<ProtocolVolumeDailyResponse>, (StatusCode, String)> {
    if q.from.is_some() || q.to.is_some() {
        return Err(bad_request("from/to are not allowed"));
    }

    let key = if q.grain.is_some() {
        let grain = vol_q::parse_volume_grain(q.grain.as_deref())
            .map_err(|_| bad_request("Invalid grain, expected hourly, daily, or monthly"))?;
        let limit = vol_q::parse_volume_limit(q.limit.as_deref(), grain)
            .map_err(|_| bad_request("Invalid limit for grain"))?;
        VolumeCacheKey::Grain { grain, limit }
    } else {
        let days = vol_q::parse_protocol_volume_days(q.days.as_deref())
            .map_err(|_| bad_request("Invalid days, expected 7 or 30"))?;
        VolumeCacheKey::AliasDays(days)
    };

    if let Ok(guard) = volume_cache().lock() {
        if let Some((resp, at)) = guard.get(&key) {
            if Instant::now().duration_since(*at) <= DAILY_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let resp = match key {
        VolumeCacheKey::AliasDays(days) => {
            build_daily_series(&state.pool, days, Some(days), None).await?
        }
        VolumeCacheKey::Grain { grain, limit } => match grain {
            vol_q::VolumeGrain::Daily => {
                build_daily_series(&state.pool, limit, None, Some((grain, limit))).await?
            }
            vol_q::VolumeGrain::Hourly => build_hourly_series(&state.pool, grain, limit).await?,
            vol_q::VolumeGrain::Monthly => build_monthly_series(&state.pool, grain, limit).await?,
        },
    };

    if let Ok(mut guard) = volume_cache().lock() {
        guard.insert(key, (resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}

async fn build_daily_series(
    pool: &sqlx::PgPool,
    points: i32,
    days_field: Option<i32>,
    grain: Option<(vol_q::VolumeGrain, i32)>,
) -> Result<ProtocolVolumeDailyResponse, (StatusCode, String)> {
    let today = utc_day_start(Utc::now()).date_naive();
    let from = today - ChronoDuration::days(i64::from(points) - 1);
    let rows = vol_q::get_daily_rows(pool, from, today)
        .await
        .map_err(internal_err)?;

    let mut by_day = HashMap::new();
    for row in rows {
        by_day.insert(row.utc_day, row);
    }

    let mut series = Vec::with_capacity(points as usize);
    for offset in (0..points).rev() {
        let day = today - ChronoDuration::days(i64::from(offset));
        let point = match by_day.get(&day) {
            Some(row) => ProtocolVolumeDailyPoint {
                utc_hour: None,
                utc_day: Some(day.to_string()),
                utc_month: None,
                volume_usd: row.volume_usd.as_ref().map(|v| v.to_string()),
                trade_count: row.trade_count,
            },
            None => ProtocolVolumeDailyPoint {
                utc_hour: None,
                utc_day: Some(day.to_string()),
                utc_month: None,
                volume_usd: Some("0".to_string()),
                trade_count: 0,
            },
        };
        series.push(point);
    }

    Ok(wrap_response(days_field, grain, series))
}

async fn build_hourly_series(
    pool: &sqlx::PgPool,
    grain: vol_q::VolumeGrain,
    limit: i32,
) -> Result<ProtocolVolumeDailyResponse, (StatusCode, String)> {
    let now_hour = vol_q::utc_hour_start(Utc::now());
    let from = now_hour - ChronoDuration::hours(i64::from(limit) - 1);
    let rows = vol_q::get_hourly_rows(pool, from, now_hour)
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
            Some(row) => ProtocolVolumeDailyPoint {
                utc_hour: Some(vol_q::format_utc_hour(hour)),
                utc_day: None,
                utc_month: None,
                volume_usd: row.volume_usd.as_ref().map(|v| v.to_string()),
                trade_count: row.trade_count,
            },
            None => ProtocolVolumeDailyPoint {
                utc_hour: Some(vol_q::format_utc_hour(hour)),
                utc_day: None,
                utc_month: None,
                volume_usd: Some("0".to_string()),
                trade_count: 0,
            },
        };
        series.push(point);
    }

    Ok(wrap_response(None, Some((grain, limit)), series))
}

async fn build_monthly_series(
    pool: &sqlx::PgPool,
    grain: vol_q::VolumeGrain,
    limit: i32,
) -> Result<ProtocolVolumeDailyResponse, (StatusCode, String)> {
    let this_month = vol_q::utc_month_start(Utc::now());
    let from = this_month
        .checked_sub_months(Months::new((limit as u32).saturating_sub(1)))
        .expect("month window in range");
    let rows = vol_q::get_monthly_rows(pool, from, this_month)
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
            Some(row) => ProtocolVolumeDailyPoint {
                utc_hour: None,
                utc_day: None,
                utc_month: Some(vol_q::format_utc_month(month)),
                volume_usd: row.volume_usd.as_ref().map(|v| v.to_string()),
                trade_count: row.trade_count,
            },
            None => ProtocolVolumeDailyPoint {
                utc_hour: None,
                utc_day: None,
                utc_month: Some(vol_q::format_utc_month(month)),
                volume_usd: Some("0".to_string()),
                trade_count: 0,
            },
        };
        series.push(point);
    }

    Ok(wrap_response(None, Some((grain, limit)), series))
}

fn wrap_response(
    days: Option<i32>,
    grain: Option<(vol_q::VolumeGrain, i32)>,
    series: Vec<ProtocolVolumeDailyPoint>,
) -> ProtocolVolumeDailyResponse {
    ProtocolVolumeDailyResponse {
        days,
        grain: grain.map(|(g, _)| g.as_str().to_string()),
        limit: grain.map(|(_, n)| n),
        timezone: "UTC".to_string(),
        methodology: "protocol_catalog".to_string(),
        series,
    }
}
