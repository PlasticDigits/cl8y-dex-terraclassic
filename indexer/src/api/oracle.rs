use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};
use crate::db::queries::oracle as db_oracle;

#[derive(Serialize, ToSchema)]
pub struct OracleSourcePrice {
    pub source: String,
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OraclePriceResponse {
    pub price_usd: Option<String>,
    pub sources: Vec<OracleSourcePrice>,
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/price",
    responses(
        (status = 200, description = "Latest USTC/USD oracle price", body = OraclePriceResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_price(
    State(state): State<AppState>,
) -> Result<Json<OraclePriceResponse>, (StatusCode, String)> {
    let current = state.ustc_price.read().await.clone();

    let source_rows = db_oracle::get_latest_prices_by_source(&state.pool)
        .await
        .map_err(internal_err)?;

    let sources: Vec<OracleSourcePrice> = source_rows
        .into_iter()
        .map(|r| OracleSourcePrice {
            source: r.source,
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OraclePriceResponse {
        price_usd: current.map(|p| p.to_string()),
        sources,
    }))
}

#[derive(Deserialize, IntoParams)]
pub struct OracleHistoryQuery {
    /// Start time (RFC 3339)
    pub from: Option<String>,
    /// End time (RFC 3339)
    pub to: Option<String>,
    /// Max results (capped at 1000)
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct OracleHistoryEntry {
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OracleHistoryResponse {
    pub prices: Vec<OracleHistoryEntry>,
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/history",
    params(OracleHistoryQuery),
    responses(
        (status = 200, description = "USTC/USD oracle price history", body = OracleHistoryResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_history(
    State(state): State<AppState>,
    Query(q): Query<OracleHistoryQuery>,
) -> Result<Json<OracleHistoryResponse>, (StatusCode, String)> {
    let now = Utc::now();
    let from = q
        .from
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(|| now - chrono::Duration::hours(24));
    let to =
        q.to.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or(now);
    let limit = q.limit.unwrap_or(200).min(1000);

    let rows = db_oracle::get_price_history(&state.pool, from, to, limit)
        .await
        .map_err(internal_err)?;

    let prices: Vec<OracleHistoryEntry> = rows
        .into_iter()
        .map(|r| OracleHistoryEntry {
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OracleHistoryResponse { prices }))
}
