use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct HookEventsQuery {
    pub hook_address: Option<String>,
    pub limit: Option<i64>,
}

#[derive(FromRow)]
struct HookEventRow {
    id: i64,
    tx_hash: String,
    hook_address: String,
    action: String,
    amount: Option<sqlx::types::BigDecimal>,
    token: Option<String>,
    skipped: Option<String>,
    warning: Option<String>,
    block_height: i64,
    block_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct HookEventResponse {
    pub id: i64,
    pub tx_hash: String,
    pub hook_address: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    pub token: Option<String>,
    pub skipped: Option<String>,
    pub warning: Option<String>,
    pub block_height: i64,
    pub block_time: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/hooks",
    params(HookEventsQuery),
    responses(
        (status = 200, description = "Recent hook execution events", body = Vec<HookEventResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Hooks"
)]
pub async fn get_hook_events(
    State(state): State<AppState>,
    Query(params): Query<HookEventsQuery>,
) -> Result<Json<Vec<HookEventResponse>>, (StatusCode, String)> {
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

    let rows = rows.map_err(internal_err)?;
    let out: Vec<HookEventResponse> = rows
        .into_iter()
        .map(|r| HookEventResponse {
            id: r.id,
            tx_hash: r.tx_hash,
            hook_address: r.hook_address,
            action: r.action,
            amount: r.amount.map(|a| a.normalized().to_string()),
            token: r.token,
            skipped: r.skipped,
            warning: r.warning,
            block_height: r.block_height,
            block_time: r.block_time.to_rfc3339(),
        })
        .collect();
    Ok(Json(out))
}
