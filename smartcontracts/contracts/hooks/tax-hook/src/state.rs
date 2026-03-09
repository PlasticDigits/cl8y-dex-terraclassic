use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct TaxHookConfig {
    pub recipient: Addr,
    pub tax_percentage_bps: u16,
    pub tax_token: Addr,
    pub admin: Addr,
}

pub const CONFIG: Item<TaxHookConfig> = Item::new("config");
