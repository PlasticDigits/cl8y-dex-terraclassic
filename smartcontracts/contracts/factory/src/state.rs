use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
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
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const WHITELISTED_CODE_IDS: Map<u64, bool> = Map::new("whitelisted_code_ids");

/// Maps canonical pair key string to PairInfo.
pub const PAIRS: Map<&str, PairInfo> = Map::new("pairs");

pub const PAIR_COUNT: Item<u64> = Item::new("pair_count");

/// Sequential index -> PairInfo for paginated enumeration.
pub const PAIR_INDEX: Map<u64, PairInfo> = Map::new("pair_index");

/// Temporary storage for the asset pair being created, read in the reply handler.
pub const PENDING_PAIR: Item<[AssetInfo; 2]> = Item::new("pending_pair");

/// Block height of the last `CreatePair` that proceeded past validation and entered
/// the instantiate/reply flow. Enforces at most one such creation per block.
pub const PAIR_CREATION_BLOCK: Item<u64> = Item::new("pair_creation_block");

pub const REPLY_INSTANTIATE_PAIR: u64 = 1;
