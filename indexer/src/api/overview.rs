use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::{assets, volume};

// GitLab #281 / #333: /overview reads materialized global_stats_24h on cache miss;
// cache the whole response for 1 minute so a request burst can't hammer the DB.
const OVERVIEW_CACHE_TTL: Duration = Duration::from_secs(60);

fn overview_cache() -> &'static Mutex<Option<(OverviewResponse, Instant)>> {
    static CACHE: OnceLock<Mutex<Option<(OverviewResponse, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Serialize, ToSchema, Clone)]
pub struct OverviewResponse {
    pub total_volume_24h: String,
    pub total_volume_24h_usd: String,
    pub total_trades_24h: i64,
    pub pair_count: i64,
    pub token_count: i64,
    pub ustc_price_usd: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/overview",
    responses(
        (status = 200, description = "Global DEX statistics", body = OverviewResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    if let Ok(guard) = overview_cache().lock() {
        if let Some((resp, at)) = guard.as_ref() {
            if Instant::now().duration_since(*at) <= OVERVIEW_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let global = volume::get_global_stats(&state.pool)
        .await
        .map_err(internal_err)?;

    let token_count = assets::get_all_assets(&state.pool)
        .await
        .map_err(internal_err)?
        .len() as i64;

    let ustc_price = state.ustc_price.read().await.clone();

    let resp = OverviewResponse {
        total_volume_24h: global.total_volume_24h.to_string(),
        total_volume_24h_usd: global.total_volume_24h_usd.to_string(),
        total_trades_24h: global.total_trades_24h,
        pair_count: global.pair_count,
        token_count,
        ustc_price_usd: ustc_price.map(|p| p.to_string()),
    };

    if let Ok(mut guard) = overview_cache().lock() {
        *guard = Some((resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}
