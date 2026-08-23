use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub token_code_id: u64,
    pub autolp_code_id: Option<u64>,
    pub ust1: Addr,
    pub cmm_treasury: Addr,
    pub cmm_governance: Addr,
    pub factory: Addr,
    pub router: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("cfg");
