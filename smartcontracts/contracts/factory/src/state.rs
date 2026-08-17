use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use dex_common::types::{AssetInfo, PairInfo};

/// Factory-level configuration stored once on instantiation and updatable
/// by governance.
#[cw_serde]
pub struct Config {
    /// Address that controls all admin operations.
    pub governance: Addr,
    /// Address that receives swap commissions from all pairs.
    pub treasury: Addr,
    /// Default swap fee (bps) assigned to new pairs.
    pub default_fee_bps: u16,
    /// Code ID used to instantiate new Pair contracts.
    pub pair_code_id: u64,
    /// Code ID used by Pair contracts to instantiate LP tokens.
    pub lp_token_code_id: u64,
    /// Default max batch/ladder rungs for newly created pairs.
    #[serde(default = "default_limit_batch_max_rungs_config")]
    pub default_limit_batch_max_rungs: u32,
    /// uluna fee to create a pair, forwarded to treasury (GitLab #276). `#[serde(default)]` so
    /// 1.3.x stored configs migrate to the 100-LUNC default on first read.
    #[serde(default = "default_pair_creation_fee_config")]
    pub pair_creation_fee_uluna: Uint128,
    /// Canonical fee-discount registry inherited by `CreatePair` (GitLab #536).
    /// `#[serde(default)]` so pre-1.8.0 stored configs load as `None`.
    #[serde(default)]
    pub discount_registry: Option<Addr>,
}

fn default_limit_batch_max_rungs_config() -> u32 {
    dex_common::pair::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS
}

fn default_pair_creation_fee_config() -> Uint128 {
    Uint128::new(dex_common::factory::DEFAULT_PAIR_CREATION_FEE_ULUNA)
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const WHITELISTED_CODE_IDS: Map<u64, bool> = Map::new("whitelisted_code_ids");

/// Maps canonical pair key string to PairInfo.
pub const PAIRS: Map<&str, PairInfo> = Map::new("pairs");

pub const PAIR_COUNT: Item<u64> = Item::new("pair_count");

/// Sequential index -> PairInfo for paginated enumeration.
pub const PAIR_INDEX: Map<u64, PairInfo> = Map::new("pair_index");

/// Canonical pair key → sequential index for O(1) `Pairs` pagination cursor resolve.
/// **Invariant:** For every entry in `PAIR_INDEX` at `idx`, `PAIR_KEY_INDEX[pair_key(asset_infos)] == idx`.
/// Maintained in `reply_instantiate_pair`; legacy deployments backfill via migrate (`1.2.0` → `1.3.0`).
pub const PAIR_KEY_INDEX: Map<&str, u64> = Map::new("pair_key_idx");

/// Pair contract address → membership marker for O(1) registry checks.
/// **Invariant:** For every entry in `PAIR_INDEX` at indices `0..PAIR_COUNT`,
/// `PAIR_ADDR_REGISTERED` contains `contract_addr → true`. Maintained in
/// `reply_instantiate_pair`; legacy deployments backfill via migrate (`1.0.0` → `1.1.0`).
pub const PAIR_ADDR_REGISTERED: Map<Addr, bool> = Map::new("pair_addr_reg");

/// Temporary storage for the asset pair being created, read in the reply handler.
pub const PENDING_PAIR: Item<[AssetInfo; 2]> = Item::new("pending_pair");

/// Block height of the last `CreatePair` that proceeded past validation and entered
/// the instantiate/reply flow. Enforces at most one such creation per block.
pub const PAIR_CREATION_BLOCK: Item<u64> = Item::new("pair_creation_block");

pub const REPLY_INSTANTIATE_PAIR: u64 = 1;

/// Wallets blocked from all pair/router protocol actions (GitLab #308).
pub const BLACKLISTED_WALLETS: Map<&Addr, bool> = Map::new("bl_wallets");
/// CW20 tokens blocked from trading on any pair (both directions).
pub const BLACKLISTED_TOKENS: Map<&Addr, bool> = Map::new("bl_tokens");
/// Pair contracts blocked from swaps, LP, and limit actions.
pub const BLACKLISTED_PAIRS: Map<&Addr, bool> = Map::new("bl_pairs");

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;

    #[test]
    fn config_missing_discount_registry_defaults_none() {
        let json = br#"{
            "governance": "terra1gov",
            "treasury": "terra1tre",
            "default_fee_bps": 30,
            "pair_code_id": 1,
            "lp_token_code_id": 2,
            "default_limit_batch_max_rungs": 20,
            "pair_creation_fee_uluna": "0"
        }"#;
        let c: Config = from_json(json).unwrap();
        assert!(
            c.discount_registry.is_none(),
            "pre-1.8.0 factory Config JSON must migrate to None"
        );
    }
}
