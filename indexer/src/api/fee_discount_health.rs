//! Narrow fee-discount registry health surface (GitLab #373).

use axum::extract::State;
use axum::Json;

use crate::indexer::fee_discount_registry_health::FeeDiscountRegistryHealthSnapshot;

use super::AppState;

/// `GET /api/v1/health/fee-discount` — cached LCD probe state (no per-trader data).
#[utoipa::path(
    get,
    path = "/api/v1/health/fee-discount",
    responses(
        (status = 200, description = "Cached fee-discount registry LCD probe. No per-trader data.", body = FeeDiscountRegistryHealthSnapshot),
    ),
    tag = "Health"
)]
pub async fn get_fee_discount_health(
    State(state): State<AppState>,
) -> Json<FeeDiscountRegistryHealthSnapshot> {
    Json(state.fee_discount_registry_health.snapshot().await)
}
