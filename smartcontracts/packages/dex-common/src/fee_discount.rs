use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct Tier {
    pub min_cl8y_balance: Uint128,
    pub discount_bps: u16,
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
    DeregisterWallet {
        wallet: String,
        epoch: Option<u64>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub governance: Addr,
    pub cl8y_token: Addr,
}

#[cw_serde]
pub struct DiscountResponse {
    pub discount_bps: u16,
    pub needs_deregister: bool,
    pub registration_epoch: Option<u64>,
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
