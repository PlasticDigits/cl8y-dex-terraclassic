//! Narrow fee-discount registry reachability probe (GitLab #365 / #374).

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use super::AppState;

#[derive(Debug, Serialize)]
pub struct FeeDiscountHealthResponse {
    pub configured: bool,
    pub fee_discount_registry_ok: Option<bool>,
}

/// `GET /api/v1/health/fee-discount` — LCD `config` probe; no per-trader data.
pub async fn get_fee_discount_health(State(state): State<AppState>) -> Json<FeeDiscountHealthResponse> {
    let addr = match state
        .fee_discount_address
        .as_ref()
        .filter(|a| !a.is_empty())
    {
        Some(addr) => addr,
        None => {
            return Json(FeeDiscountHealthResponse {
                configured: false,
                fee_discount_registry_ok: None,
            });
        }
    };

    let ok = state
        .lcd
        .query_contract::<serde_json::Value>(addr, &serde_json::json!({ "config": {} }))
        .await
        .is_ok();

    Json(FeeDiscountHealthResponse {
        configured: true,
        fee_discount_registry_ok: Some(ok),
    })
}
