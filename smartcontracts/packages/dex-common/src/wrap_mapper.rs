use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct RateLimitConfig {
    pub max_amount_per_window: Uint128,
    pub window_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    NotifyDeposit {
        depositor: String,
        denom: String,
        amount: Uint128,
    },
    Receive(Cw20ReceiveMsg),
    SetDenomMapping {
        denom: String,
        cw20_addr: String,
    },
    RemoveDenomMapping {
        denom: String,
    },
    SetRateLimit {
        denom: String,
        config: RateLimitConfig,
    },
    RemoveRateLimit {
        denom: String,
    },
    ProposeGovernanceTransfer {
        new_governance: String,
    },
    AcceptGovernanceTransfer {},
    CancelGovernanceTransfer {},
    SetPaused {
        paused: bool,
    },
    SetFeeBps {
        fee_bps: u16,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Unwrap { recipient: Option<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(DenomMappingResponse)]
    DenomMapping { denom: String },
    #[returns(AllDenomMappingsResponse)]
    AllDenomMappings {},
    #[returns(RateLimitResponse)]
    RateLimit { denom: String },
    #[returns(PendingGovernanceResponse)]
    PendingGovernance {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub governance: Addr,
    pub treasury: Addr,
    pub paused: bool,
    pub fee_bps: u16,
}

#[cw_serde]
pub struct PendingGovernanceResponse {
    pub new_governance: Option<Addr>,
    pub execute_after: Option<Timestamp>,
}

#[cw_serde]
pub struct DenomMappingResponse {
    pub denom: String,
    pub cw20_addr: Addr,
}

#[cw_serde]
pub struct DenomMappingEntry {
    pub denom: String,
    pub cw20_addr: Addr,
}

#[cw_serde]
pub struct AllDenomMappingsResponse {
    pub mappings: Vec<DenomMappingEntry>,
}

#[cw_serde]
pub struct RateLimitResponse {
    pub config: Option<RateLimitConfig>,
    pub current_window_start: Option<Timestamp>,
    pub amount_used: Uint128,
}

/// Treasury execute messages needed by the router and wrap-mapper integration.
#[cw_serde]
pub enum TreasuryExecuteMsg {
    WrapDeposit {},
    InstantWithdraw {
        recipient: String,
        denom: String,
        amount: Uint128,
    },
    SetDenomWrapper {
        denom: String,
        wrapper: String,
    },
    RemoveDenomWrapper {
        denom: String,
    },
}
