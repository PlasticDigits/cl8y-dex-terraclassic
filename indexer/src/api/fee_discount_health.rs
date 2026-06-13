//! Narrow fee-discount registry health surface (GitLab #373).

use axum::Json;
use axum::extract::State;

use crate::api::AppState;
use crate::indexer::fee_discount_registry_health::FeeDiscountRegistryHealthSnapshot;

pub async fn get_fee_discount_health(
    State(state): State<AppState>,
) -> Json<FeeDiscountRegistryHealthSnapshot> {
    Json(state.fee_discount_registry_health.snapshot().await)
}
