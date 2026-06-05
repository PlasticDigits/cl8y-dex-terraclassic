//! Read-only compliance helpers — factory trading blacklist (GitLab #308).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::internal_err;
use super::AppState;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct BlacklistCheckParams {
    pub wallet: Option<String>,
    /// Comma-separated CW20 contract addresses.
    pub tokens: Option<String>,
    pub pair: Option<String>,
    /// Comma-separated pair contract addresses (multihop).
    pub pairs: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BlacklistCheckApiResponse {
    pub blocked: bool,
    pub wallet_blacklisted: bool,
    pub blacklisted_tokens: Vec<String>,
    pub pair_blacklisted: bool,
    pub blacklisted_pairs: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/compliance/blacklist-check",
    params(BlacklistCheckParams),
    responses(
        (status = 200, description = "Factory blacklist probe", body = BlacklistCheckApiResponse),
        (status = 503, description = "LCD unavailable"),
    ),
    tag = "Compliance"
)]
pub async fn blacklist_check(
    State(state): State<AppState>,
    Query(params): Query<BlacklistCheckParams>,
) -> Result<axum::Json<BlacklistCheckApiResponse>, (StatusCode, String)> {
    let factory = state
        .factory_address
        .as_deref()
        .ok_or((StatusCode::NOT_FOUND, "Factory address not configured".to_string()))?;

    let tokens: Vec<String> = params
        .tokens
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let pairs: Vec<String> = params
        .pairs
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let query = serde_json::json!({
        "blacklist_check": {
            "wallet": params.wallet,
            "tokens": tokens,
            "pair": params.pair,
            "pairs": pairs,
        }
    });

    let resp: BlacklistCheckApiResponse = state
        .lcd
        .query_contract(factory, &query)
        .await
        .map_err(|e| internal_err(e))?;

    Ok(axum::Json(resp))
}
