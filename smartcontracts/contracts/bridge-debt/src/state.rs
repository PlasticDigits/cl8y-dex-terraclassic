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
    #[serde(default)]
    pub pending_owner: Option<Addr>,
}

#[cw_serde]
pub struct Market {
    pub underlying: Addr,
    pub debt_token: Addr,
    pub debtor: Addr,
    pub decimals: u8,
    pub outstanding: Uint128,
    #[serde(default)]
    pub pins: Option<MarketPins>,
    #[serde(default)]
    pub limits: Option<IssuanceLimits>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub epoch_issued: Uint128,
    #[serde(default)]
    pub pending_upgrade: Option<UpgradePins>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKETS: Map<&Addr, Market> = Map::new("markets");
/// Debt-token address → unit. Prevents one receipt from tracking two assets.
pub const DEBT_TOKENS: Map<&Addr, Empty> = Map::new("debt_tokens");

#[cw_serde]
pub struct UnderlyingPolicy {
    pub multi_minter: bool,
}

#[cw_serde]
pub struct MarketPins {
    pub debt_code_id: u64,
    pub underlying_code_id: u64,
    pub multi_minter: bool,
    pub minter: Addr,
    pub minter_cap: Option<Uint128>,
    pub additional_minters: Vec<Addr>,
}

#[cw_serde]
pub struct IssuanceLimits {
    pub max_per_issue: Uint128,
    pub max_outstanding: Uint128,
    pub epoch_seconds: u64,
    pub max_per_epoch: Uint128,
}

#[cw_serde]
pub struct UpgradePins {
    pub underlying_supply: Uint128,
    pub debt_code_id: u64,
    pub underlying_code_id: u64,
}

pub const UNDERLYING_CODES: Map<u64, UnderlyingPolicy> = Map::new("underlying_codes");
