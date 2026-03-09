use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::types::{AssetInfo, PairInfo};

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
    /// TerraSwap-compatible: create a pair from two AssetInfo values.
    /// Only `AssetInfo::Token` is accepted; `NativeToken` is rejected.
    CreatePair {
        asset_infos: [AssetInfo; 2],
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
    /// Set the fee discount registry for a specific pair. Governance only.
    SetDiscountRegistry {
        pair: String,
        registry: Option<String>,
    },
    /// Set the fee discount registry for all pairs. Governance only.
    SetDiscountRegistryAll {
        registry: Option<String>,
    },
    /// Emergency pause/unpause a pair. Only governance can call this.
    SetPairPaused {
        pair: String,
        paused: bool,
    },
    /// Recover tokens accidentally sent to a pair contract. Only governance
    /// can call this. Sweeps the excess (actual balance minus pool reserves)
    /// to `recipient`.
    SweepPair {
        pair: String,
        token: String,
        recipient: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// TerraSwap-compatible query name.
    #[returns(ConfigResponse)]
    Config {},
    /// TerraSwap-compatible: look up a pair by its two AssetInfos.
    #[returns(PairResponse)]
    Pair { asset_infos: [AssetInfo; 2] },
    /// TerraSwap-compatible: paginated list of pairs.
    #[returns(PairsResponse)]
    Pairs {
        start_after: Option<[AssetInfo; 2]>,
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

/// TerraSwap-compatible response: flat list of pairs, no cursor.
#[cw_serde]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
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
