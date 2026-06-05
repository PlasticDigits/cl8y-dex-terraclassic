use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::types::{AssetInfo, PairInfo};

/// Default pair-creation fee in uluna (100 LUNC), transferred to the treasury on `CreatePair`
/// to make permissionless pair-spam costly (GitLab #276). Governance-settable per deployment.
pub const DEFAULT_PAIR_CREATION_FEE_ULUNA: u128 = 100_000_000;

#[cw_serde]
pub struct InstantiateMsg {
    pub governance: String,
    pub treasury: String,
    pub default_fee_bps: u16,
    pub pair_code_id: u64,
    pub lp_token_code_id: u64,
    pub whitelisted_code_ids: Vec<u64>,
    /// Default max batch/ladder rungs for new pairs (clamped to hard cap on-chain).
    #[serde(default = "default_limit_batch_max_rungs_instantiate")]
    pub default_limit_batch_max_rungs: u32,
    /// uluna required (and forwarded to treasury) to create a pair (GitLab #276). Absent →
    /// [`DEFAULT_PAIR_CREATION_FEE_ULUNA`]; governance-settable after instantiate.
    #[serde(default = "default_pair_creation_fee")]
    pub pair_creation_fee_uluna: Uint128,
}

fn default_limit_batch_max_rungs_instantiate() -> u32 {
    crate::limit_placement::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS
}

fn default_pair_creation_fee() -> Uint128 {
    Uint128::new(DEFAULT_PAIR_CREATION_FEE_ULUNA)
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
    /// Set the uluna fee required to create a pair (forwarded to treasury). Governance only (GitLab #276).
    SetPairCreationFee {
        fee_uluna: Uint128,
    },
    SetPairHooks {
        pair: String,
        hooks: Vec<String>,
    },
    UpdateConfig {
        governance: Option<String>,
        treasury: Option<String>,
        default_fee_bps: Option<u16>,
        default_limit_batch_max_rungs: Option<u32>,
    },
    /// Set max batch/ladder rungs on one pair. Governance only.
    SetPairLimitBatchMax {
        pair: String,
        max_rungs: u32,
    },
    /// Set limit book force-clean dust thresholds on one pair. Governance only (GitLab #263).
    SetPairLimitCleanConfig {
        pair: String,
        min_remaining_token0: cosmwasm_std::Uint128,
        min_remaining_token1: cosmwasm_std::Uint128,
    },
    /// Set the fee discount registry for a specific pair. Governance only.
    SetDiscountRegistry {
        pair: String,
        registry: Option<String>,
    },
    /// Set the fee discount registry for all pairs in one transaction (governance only).
    ///
    /// **Bounded:** fails when `PAIR_COUNT` exceeds the default pagination cap (**10** pairs;
    /// see [`crate::pagination`]). For larger factories use
    /// [`SetDiscountRegistryBatch`](ExecuteMsg::SetDiscountRegistryBatch)
    /// (GitLab [#123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123),
    /// [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)).
    SetDiscountRegistryAll {
        registry: Option<String>,
    },
    /// Paginated/chunked rollout of the same registry update across indexed pairs (`PAIR_INDEX`).
    ///
    /// - `start_after`: exclusive numeric index cursor into `PAIR_INDEX` (`None` ⇒ start before `0`,
    ///   i.e. first index handled is `0`).
    /// - `limit`: max pair messages emitted this execution; omit for default clamp (see [`crate::pagination`]).
    ///
    /// **Invariants:** New pairs append at indices `< PAIR_COUNT`. If pairs are registered between
    /// batches, rerun until the response reports no further work (`has_more = false`).
    SetDiscountRegistryBatch {
        registry: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
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
    pub default_limit_batch_max_rungs: u32,
    pub pair_code_id: u64,
    pub lp_token_code_id: u64,
    /// uluna required to create a pair, forwarded to treasury (GitLab #276).
    pub pair_creation_fee_uluna: Uint128,
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
