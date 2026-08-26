//! `GET /api/v1/protocol/volume/daily` — UTC-day Protocol volume series (GitLab #652).
//!
//! `days` is allowlisted (`7` \| `30`). Unknown / injection → **400**.
//! Whole-response cache 60s keyed by allowlisted `days` only. Reads
//! `protocol_daily_volume` — never `swap_events` / `defillama_daily_stats`.
//! Missing rollup rows are idle `"0"` (documented). Newest-last series.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::protocol_volume as vol_q;
use crate::indexer::defillama::utc_day_start;

const DAILY_CACHE_TTL: Duration = Duration::from_secs(60);

fn daily_cache() -> &'static Mutex<HashMap<i32, (ProtocolVolumeDailyResponse, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<i32, (ProtocolVolumeDailyResponse, Instant)>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize)]
pub struct ProtocolVolumeDailyQuery {
    pub days: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolVolumeDailyPoint {
    /// `YYYY-MM-DD` UTC calendar day.
    pub utc_day: String,
    /// `"0"` idle; JSON `null` when the day has trades but no priced USD.
    pub volume_usd: Option<String>,
    pub trade_count: i64,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolVolumeDailyResponse {
    pub days: i32,
    pub timezone: String,
    /// Protocol catalog (overview `total_volume_*_usd`). Not DeFiLlama gem-exclude.
    pub methodology: String,
    pub series: Vec<ProtocolVolumeDailyPoint>,
}

/// Drop the 60s daily-volume response cache (tests).
pub fn reset_protocol_volume_cache() {
    if let Ok(mut guard) = daily_cache().lock() {
        guard.clear();
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/protocol/volume/daily",
    params(
        ("days" = String, Query, description = "UTC-day count: 7 or 30")
    ),
    responses(
        (status = 200, description = "UTC-day Protocol volume series", body = ProtocolVolumeDailyResponse),
        (status = 400, description = "Invalid days"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_protocol_volume_daily(
    State(state): State<AppState>,
    Query(q): Query<ProtocolVolumeDailyQuery>,
) -> Result<Json<ProtocolVolumeDailyResponse>, (StatusCode, String)> {
    let days = match vol_q::parse_protocol_volume_days(q.days.as_deref()) {
        Ok(d) => d,
        Err(()) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid days, expected 7 or 30".to_string(),
            ));
        }
    };

    if let Ok(guard) = daily_cache().lock() {
        if let Some((resp, at)) = guard.get(&days) {
            if Instant::now().duration_since(*at) <= DAILY_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let today = utc_day_start(Utc::now()).date_naive();
    let from = today - ChronoDuration::days(i64::from(days) - 1);
    let rows = vol_q::get_daily_rows(&state.pool, from, today)
        .await
        .map_err(internal_err)?;

    let mut by_day = HashMap::new();
    for row in rows {
        by_day.insert(row.utc_day, row);
    }

    let mut series = Vec::with_capacity(days as usize);
    for offset in (0..days).rev() {
        let day = today - ChronoDuration::days(i64::from(offset));
        let point = match by_day.get(&day) {
            Some(row) => ProtocolVolumeDailyPoint {
                utc_day: day.to_string(),
                volume_usd: row.volume_usd.as_ref().map(|v| v.to_string()),
                trade_count: row.trade_count,
            },
            None => ProtocolVolumeDailyPoint {
                utc_day: day.to_string(),
                volume_usd: Some("0".to_string()),
                trade_count: 0,
            },
        };
        series.push(point);
    }

    let resp = ProtocolVolumeDailyResponse {
        days,
        timezone: "UTC".to_string(),
        methodology: "protocol_catalog".to_string(),
        series,
    };

    if let Ok(mut guard) = daily_cache().lock() {
        guard.insert(days, (resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}
