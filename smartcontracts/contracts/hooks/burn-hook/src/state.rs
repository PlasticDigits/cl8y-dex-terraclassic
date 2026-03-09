use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct BurnHookConfig {
    pub burn_token: Addr,
    pub burn_percentage_bps: u16,
    pub admin: Addr,
}

pub const CONFIG: Item<BurnHookConfig> = Item::new("config");
