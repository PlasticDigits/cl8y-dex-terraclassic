use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use super::AppState;
use crate::db::queries::{assets, volume};

#[derive(Serialize)]
pub struct OverviewResponse {
    pub total_volume_24h: String,
    pub total_trades_24h: i64,
    pub pair_count: i64,
    pub token_count: i64,
}

pub async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    let global = volume::get_global_stats(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token_count = assets::get_all_assets(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .len() as i64;

    Ok(Json(OverviewResponse {
        total_volume_24h: global.total_volume_24h.to_string(),
        total_trades_24h: global.total_trades_24h,
        pair_count: global.pair_count,
        token_count,
    }))
}
