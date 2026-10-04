use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "bridge-debt-token";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// CW20 tickers are often capped at 12 characters. Bridge-debt symbols such as
/// `USDT-Bridge-Debt` are 16, so this token allows 3–32.
pub const SYMBOL_MAX_LEN: usize = 32;

#[cw_serde]
pub struct MinterData {
    pub minter: Addr,
    pub cap: Option<Uint128>,
}

#[cw_serde]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: MinterData,
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");
pub const DEBTOR: Item<Addr> = Item::new("debtor");
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balance");
