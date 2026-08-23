use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
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

/// In-flight AutoLP instantiate after token (or EnableFeature) reply (#605).
#[cw_serde]
pub struct PendingAutolp {
    pub token: Addr,
    pub manager: String,
    pub threshold: Uint128,
    pub lp_recipient: String,
}

pub const CONFIG: Item<Config> = Item::new("cfg");
pub const PENDING_AUTOLP: Item<PendingAutolp> = Item::new("pending_autolp");
