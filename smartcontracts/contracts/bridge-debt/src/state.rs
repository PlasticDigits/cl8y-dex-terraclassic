use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "bridge-debt";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    /// Code id of `bridge-debt-token`. Registration rejects every other wasm.
    pub debt_code_id: Option<u64>,
}

#[cw_serde]
pub struct Market {
    pub underlying: Addr,
    pub debt_token: Addr,
    pub debtor: Addr,
    pub decimals: u8,
    pub outstanding: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKETS: Map<&Addr, Market> = Map::new("markets");
/// Debt-token address → unit. Prevents one receipt from tracking two assets.
pub const DEBT_TOKENS: Map<&Addr, Empty> = Map::new("debt_tokens");
