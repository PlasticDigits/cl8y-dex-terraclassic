use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::types::PairInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub governance: String,
    pub treasury: String,
    pub default_fee_bps: u16,
    pub pair_code_id: u64,
    pub lp_token_code_id: u64,
    pub whitelisted_code_ids: Vec<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        token_a: String,
        token_b: String,
    },
    AddWhitelistedCodeId {
        code_id: u64,
    },
    RemoveWhitelistedCodeId {
        code_id: u64,
    },
    SetPairFee {
        pair: String,
        fee_bps: u16,
    },
    SetPairHooks {
        pair: String,
        hooks: Vec<String>,
    },
    UpdateConfig {
        governance: Option<String>,
        treasury: Option<String>,
        default_fee_bps: Option<u16>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(PairResponse)]
    GetPair { token_a: String, token_b: String },
    #[returns(PairsResponse)]
    GetAllPairs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(CodeIdsResponse)]
    GetWhitelistedCodeIds {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(PairCountResponse)]
    GetPairCount {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub governance: Addr,
    pub treasury: Addr,
    pub default_fee_bps: u16,
    pub pair_code_id: u64,
    pub lp_token_code_id: u64,
}

#[cw_serde]
pub struct PairResponse {
    pub pair: PairInfo,
}

#[cw_serde]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
    pub next: Option<String>,
}

#[cw_serde]
pub struct CodeIdsResponse {
    pub code_ids: Vec<u64>,
    pub next: Option<u64>,
}

#[cw_serde]
pub struct PairCountResponse {
    pub count: u64,
}
