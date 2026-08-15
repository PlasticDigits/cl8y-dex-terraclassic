use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

/// Basis-point cap shared by swap and limit-placement discounts.
pub const MAX_DISCOUNT_BPS: u16 = 10_000;

/// Swap-leg `discount_bps` → limit-order placement discount on the standard ladder
/// ([GitLab #514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)).
/// Each self-register tier inherits the next tier’s remaining-fee; tier 9 (`9500`) → `10000`.
/// Unknown / governance values pass through so custom test tiers are unchanged.
pub fn standard_shifted_limit_discount_bps(discount_bps: u16) -> u16 {
    match discount_bps {
        250 => 1_000,
        1_000 => 2_000,
        2_000 => 3_500,
        3_500 => 5_000,
        5_000 => 6_000,
        6_000 => 7_500,
        7_500 => 8_500,
        8_500 => 9_500,
        9_500 => 10_000,
        other => other,
    }
}

/// Stored `limit_discount_bps` if set; otherwise the swap `discount_bps` (no extra break).
pub fn resolve_limit_discount_bps(limit_discount_bps: Option<u16>, discount_bps: u16) -> u16 {
    limit_discount_bps.unwrap_or(discount_bps)
}

/// Integer pair fee after a discount: `fee_bps * (10000 - discount_bps) / 10000`.
pub fn effective_fee_bps(fee_bps: u16, discount_bps: u16) -> u16 {
    let discounted = (fee_bps as u32) * (10000u32.saturating_sub(discount_bps as u32)) / 10000u32;
    discounted as u16
}

/// Maker placement bps: `floor(limit_effective / 2)` using the resolved limit discount.
pub fn maker_placement_fee_bps(fee_bps: u16, limit_discount_bps: u16) -> u16 {
    effective_fee_bps(fee_bps, limit_discount_bps) / 2
}

#[cw_serde]
pub struct Tier {
    pub min_cl8y_balance: Uint128,
    pub discount_bps: u16,
    /// Placement-only discount ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)).
    /// `None` / omitted JSON → use `discount_bps` (swap and placement stay aligned).
    #[serde(default)]
    pub limit_discount_bps: Option<u16>,
    pub governance_only: bool,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(DiscountResponse)]
    GetDiscount { trader: String, sender: String },
    #[returns(TierResponse)]
    GetTier { tier_id: u8 },
    #[returns(TiersResponse)]
    GetTiers {},
    #[returns(RegistrationResponse)]
    GetRegistration { trader: String },
    #[returns(IsTrustedRouterResponse)]
    IsTrustedRouter { addr: String },
}

#[cw_serde]
pub enum ExecuteMsg {
    DeregisterWallet { wallet: String, epoch: Option<u64> },
}

#[cw_serde]
pub struct ConfigResponse {
    pub governance: Addr,
    pub cl8y_token: Addr,
}

#[cw_serde]
pub struct DiscountResponse {
    pub discount_bps: u16,
    /// Resolved limit-order placement discount. Omitted on pre-#514 registries → treat as `discount_bps`.
    #[serde(default)]
    pub limit_discount_bps: Option<u16>,
    pub needs_deregister: bool,
    pub registration_epoch: Option<u64>,
}

impl DiscountResponse {
    pub fn resolved_limit_discount_bps(&self) -> u16 {
        resolve_limit_discount_bps(self.limit_discount_bps, self.discount_bps)
    }
}

#[cw_serde]
pub struct TierResponse {
    pub tier_id: u8,
    pub tier: Tier,
}

#[cw_serde]
pub struct TierEntry {
    pub tier_id: u8,
    pub tier: Tier,
}

#[cw_serde]
pub struct TiersResponse {
    pub tiers: Vec<TierEntry>,
}

#[cw_serde]
pub struct RegistrationResponse {
    pub registered: bool,
    pub tier_id: Option<u8>,
    pub tier: Option<Tier>,
}

#[cw_serde]
pub struct IsTrustedRouterResponse {
    pub is_trusted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_shift_moves_each_rung_and_zeros_tier_9() {
        assert_eq!(standard_shifted_limit_discount_bps(250), 1_000);
        assert_eq!(standard_shifted_limit_discount_bps(1_000), 2_000);
        assert_eq!(standard_shifted_limit_discount_bps(2_000), 3_500);
        assert_eq!(standard_shifted_limit_discount_bps(3_500), 5_000);
        assert_eq!(standard_shifted_limit_discount_bps(5_000), 6_000);
        assert_eq!(standard_shifted_limit_discount_bps(6_000), 7_500);
        assert_eq!(standard_shifted_limit_discount_bps(7_500), 8_500);
        assert_eq!(standard_shifted_limit_discount_bps(8_500), 9_500);
        assert_eq!(standard_shifted_limit_discount_bps(9_500), 10_000);
        assert_eq!(standard_shifted_limit_discount_bps(10_000), 10_000);
        assert_eq!(standard_shifted_limit_discount_bps(0), 0);
        assert_eq!(standard_shifted_limit_discount_bps(2_500), 2_500);
    }

    #[test]
    fn placement_targets_at_180_bps_pair() {
        // Confirmed on GitLab #514: T9 placement 4 → 0; unregistered stays 90.
        assert_eq!(maker_placement_fee_bps(180, 1_000), 81);
        assert_eq!(maker_placement_fee_bps(180, 2_000), 72);
        assert_eq!(maker_placement_fee_bps(180, 3_500), 58);
        assert_eq!(maker_placement_fee_bps(180, 5_000), 45);
        assert_eq!(maker_placement_fee_bps(180, 6_000), 36);
        assert_eq!(maker_placement_fee_bps(180, 7_500), 22);
        assert_eq!(maker_placement_fee_bps(180, 8_500), 13);
        assert_eq!(maker_placement_fee_bps(180, 9_500), 4);
        assert_eq!(maker_placement_fee_bps(180, 10_000), 0);
        assert_eq!(maker_placement_fee_bps(180, 0), 90);
        assert_eq!(effective_fee_bps(180, 9_500), 9);
    }

    #[test]
    fn resolve_limit_falls_back_to_swap_discount() {
        assert_eq!(resolve_limit_discount_bps(None, 9_500), 9_500);
        assert_eq!(resolve_limit_discount_bps(Some(10_000), 9_500), 10_000);
    }
}
