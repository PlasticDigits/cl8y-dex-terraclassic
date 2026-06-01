//! Permissionless limit book clean config and caps (GitLab #263).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

/// Hard ceiling on orders parked per `CleanLimitBook` execute (gas safety; GitLab #263).
pub const MAX_LIMIT_CLEAN_ORDERS_HARD_CAP: u32 = 100;

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
