use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use dex_common::hook::{ComputeSwapFeeResponse, HookExecuteMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub target_pair: String,
    /// CW20 LP token contract for `target_pair`.
    pub lp_token: String,
    pub percentage_bps: u16,
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Hook(HookExecuteMsg),
    UpdateConfig {
        target_pair: Option<String>,
        lp_token: Option<String>,
        percentage_bps: Option<u16>,
    },
    UpdateAllowedPairs {
        add: Vec<String>,
        remove: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(ComputeSwapFeeResponse)]
    ComputeSwapFee {
        output_token: String,
        output_amount: Uint128,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub target_pair: Addr,
    pub lp_token: Addr,
    pub percentage_bps: u16,
    pub admin: Addr,
}

#[cw_serde]
pub struct MigrateMsg {}
