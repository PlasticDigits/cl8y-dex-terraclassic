use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct LpBurnHookConfig {
    pub target_pair: Addr,
    pub percentage_bps: u16,
    pub admin: Addr,
}

pub const CONFIG: Item<LpBurnHookConfig> = Item::new("config");
