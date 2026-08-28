//! Read-only compliance helpers — factory trading blacklist (GitLab #308).
//!
//! Per-request cost (#694 / RE-03): comma-split `tokens` / `pairs` are capped
//! before the factory LCD query. Oversize lists are **400** (fail-closed, no
//! silent truncate).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::lcd_gateway_err;
use super::AppState;

/// Max CW20 addresses after trim / empty-drop (GitLab #694 / RE-03).
pub const MAX_BLACKLIST_TOKENS: usize = 16;
/// Max pair addresses after trim / empty-drop (GitLab #694 / RE-03).
pub const MAX_BLACKLIST_PAIRS: usize = 8;

fn split_csv_addrs(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

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
        (status = 400, description = "tokens or pairs list exceeds cap"),
        (status = 502, description = "LCD unavailable"),
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

    let tokens = split_csv_addrs(params.tokens.as_deref());
    let pairs = split_csv_addrs(params.pairs.as_deref());

    if tokens.len() > MAX_BLACKLIST_TOKENS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("tokens list exceeds max {MAX_BLACKLIST_TOKENS} entries"),
        ));
    }
    if pairs.len() > MAX_BLACKLIST_PAIRS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("pairs list exceeds max {MAX_BLACKLIST_PAIRS} entries"),
        ));
    }

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
        .map_err(lcd_gateway_err)?;

    Ok(axum::Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_csv_drops_empties_and_trims() {
        assert!(split_csv_addrs(None).is_empty());
        assert_eq!(
            split_csv_addrs(Some(" a, ,b,")),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(split_csv_addrs(Some("")).len(), 0);
    }

    #[test]
    fn list_caps_are_16_and_8() {
        assert_eq!(MAX_BLACKLIST_TOKENS, 16);
        assert_eq!(MAX_BLACKLIST_PAIRS, 8);
    }
}
