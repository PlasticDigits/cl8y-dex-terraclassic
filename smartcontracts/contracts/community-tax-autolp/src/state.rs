use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub token: Addr,
    pub manager: Addr,
    /// Immutable CL8Y factory (launcher pin). Not writable via `UpdateConfig`.
    pub factory: Addr,
    pub router: Option<Addr>,
    pub pair: Option<Addr>,
    pub quote_token: Option<Addr>,
    pub threshold: Uint128,
    pub lp_recipient: Addr,
    pub skim_max_spread: Decimal,
    pub skim_min_return: Option<Uint128>,
}

pub const CONFIG: Item<Config> = Item::new("cfg");
/// Reentrancy lock — skim never runs inside token Transfer/Send (T592-10).
pub const SKIMMING: Item<bool> = Item::new("skim");
