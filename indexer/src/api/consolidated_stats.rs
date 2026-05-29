//! Consolidated CG/CMC extension fields for hybrid + pool-only v2 swap reporting (GitLab #189).

use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::db::queries::swap_events::{self, SwapEventRow};

/// CL8Y-specific extension block on CG/CMC responses. Standard Peatio fields remain unchanged;
/// aggregators may ignore this object. Volumes in parent fields are **consolidated** totals.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Cl8yConsolidatedExtensions {
    /// `true` when parent `base_volume` / `quote_volume` include hybrid and pool-only swaps once.
    pub consolidated: bool,
    pub hybrid_trade_count_24h: String,
    pub pool_only_trade_count_24h: String,
    /// Sum of indexed `book_return_amount` over 24h (ask / quote side of the pair).
    pub book_leg_volume_quote_24h: String,
    /// Sum of indexed `pool_return_amount` over 24h (ask / quote side).
    pub pool_leg_volume_quote_24h: String,
}

pub async fn fetch_consolidated_extensions(
    pool: &PgPool,
    pair_id: i32,
) -> Result<Cl8yConsolidatedExtensions, sqlx::Error> {
    let b = swap_events::get_24h_hybrid_breakdown(pool, pair_id).await?;
    Ok(Cl8yConsolidatedExtensions {
        consolidated: true,
        hybrid_trade_count_24h: b.hybrid_trade_count.to_string(),
        pool_only_trade_count_24h: b.pool_only_trade_count.to_string(),
        book_leg_volume_quote_24h: b.book_leg_volume_quote.to_string(),
        pool_leg_volume_quote_24h: b.pool_leg_volume_quote.to_string(),
    })
}

/// Optional per-trade hybrid leg volumes when indexed on the swap event.
pub fn hybrid_leg_volumes(trade: &SwapEventRow) -> (Option<String>, Option<String>) {
    if trade.pool_return_amount.is_none() && trade.book_return_amount.is_none() {
        return (None, None);
    }
    (
        trade.pool_return_amount.as_ref().map(|v| v.to_string()),
        trade.book_return_amount.as_ref().map(|v| v.to_string()),
    )
}
