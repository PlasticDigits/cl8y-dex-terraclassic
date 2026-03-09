use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub governance: Addr,
    pub cl8y_token: Addr,
}

#[cw_serde]
pub struct Tier {
    pub min_cl8y_balance: Uint128,
    pub discount_bps: u16,
    pub governance_only: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const TIERS: Map<u8, Tier> = Map::new("tiers");
pub const REGISTRATIONS: Map<&str, u8> = Map::new("registrations");
pub const TRUSTED_ROUTERS: Map<&str, bool> = Map::new("trusted_routers");
