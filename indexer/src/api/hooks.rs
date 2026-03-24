use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::{internal_err, AppState};

#[derive(Deserialize)]
pub struct HookEventsQuery {
    pub hook_address: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct HookEventRow {
    pub id: i64,
    pub tx_hash: String,
    pub hook_address: String,
    pub action: String,
    pub amount: Option<sqlx::types::BigDecimal>,
    pub token: Option<String>,
    pub skipped: Option<String>,
    pub warning: Option<String>,
    pub block_height: i64,
    pub block_time: chrono::DateTime<chrono::Utc>,
}

pub async fn get_hook_events(
    State(state): State<AppState>,
    Query(params): Query<HookEventsQuery>,
) -> Result<Json<Vec<HookEventRow>>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);

    let rows = if let Some(addr) = &params.hook_address {
        sqlx::query_as::<_, HookEventRow>(
            "SELECT id, tx_hash, hook_address, action, amount, token, skipped, warning, block_height, block_time
             FROM hook_events WHERE hook_address = $1 ORDER BY block_time DESC LIMIT $2",
        )
        .bind(addr)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, HookEventRow>(
            "SELECT id, tx_hash, hook_address, action, amount, token, skipped, warning, block_height, block_time
             FROM hook_events ORDER BY block_time DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    rows.map(Json).map_err(internal_err)
}
