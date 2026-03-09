use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use dex_common::types::{AssetInfo, FeeConfig};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct PairInfoState {
    pub asset_infos: [AssetInfo; 2],
    pub lp_token: Addr,
    pub factory: Addr,
}

pub const PAIR_INFO: Item<PairInfoState> = Item::new("pair_info");
pub const RESERVES: Item<(Uint128, Uint128)> = Item::new("reserves");
pub const FEE_CONFIG: Item<FeeConfig> = Item::new("fee_config");
pub const HOOKS: Item<Vec<Addr>> = Item::new("hooks");
pub const TOTAL_LP_SUPPLY: Item<Uint128> = Item::new("total_lp_supply");
