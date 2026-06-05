//! Permissionless limit book clean config and caps (GitLab #263).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

/// Hard ceiling on orders parked per `CleanLimitBook` execute (gas safety; GitLab #263).
pub const MAX_LIMIT_CLEAN_ORDERS_HARD_CAP: u32 = 100;

/// Hard ceiling on nodes *visited* per `CleanLimitBook` walk (gas safety; GitLab #274). Mirrors
/// the matcher's `MAX_SCAN_STEPS` (~19k-gas/iter sizing). The park cap bounds writes; this bounds
/// traversal, so a book of healthy or zero-remaining orders can't be walked unboundedly.
pub const MAX_CLEAN_SCAN_STEPS: u32 = 500;

#[cw_serde]
pub struct LimitCleanConfigResponse {
    /// Min remaining token0 notional for asks; `0` disables force-clean on asks.
    pub min_remaining_token0: Uint128,
    /// Min remaining token1 notional for bids; `0` disables force-clean on bids.
    pub min_remaining_token1: Uint128,
}

/// Clamp caller `max_orders` to the on-chain hard cap.
pub fn clamp_max_clean_orders(max_orders: u32) -> u32 {
    max_orders.clamp(1, MAX_LIMIT_CLEAN_ORDERS_HARD_CAP)
}

/// Clamp caller `max_steps` to the on-chain traversal cap. `0` (or an absent msg field) means
/// "use the full cap" rather than zero steps, so a default-constructed call walks normally.
pub fn clamp_max_clean_scan_steps(max_steps: u32) -> u32 {
    if max_steps == 0 {
        MAX_CLEAN_SCAN_STEPS
    } else {
        max_steps.min(MAX_CLEAN_SCAN_STEPS)
    }
}
