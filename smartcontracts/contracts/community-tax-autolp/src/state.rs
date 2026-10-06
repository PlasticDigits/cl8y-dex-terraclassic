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
/// Reentrancy lock through swap AND provide replies; blocks nested skim/config writes.
/// Skim never runs inside token Transfer/Send (T592-10).
pub const SKIMMING: Item<bool> = Item::new("skim");

/// Manager-approved reserves/share identity, refreshed only by explicitly binding a pair.
/// Keeping this outside Config preserves old Config decoding; missing legacy policy fails closed.
#[cw_serde]
pub struct LpPolicy {
    pub pair: Addr,
    pub quote: Addr,
    pub lp_token: Addr,
    pub tax_reserve: Uint128,
    pub quote_reserve: Uint128,
    pub total_share: Uint128,
}

#[cw_serde]
pub struct PendingSkim {
    pub pair: Addr,
    pub token: Addr,
    pub quote: Addr,
    pub lp_token: Addr,
    pub recipient: Addr,
    pub lp_before: Uint128,
    pub min_lp: Uint128,
    pub tolerance: Decimal,
}

pub const LP_POLICY: Item<LpPolicy> = Item::new("lp_policy");
pub const PENDING_SKIM: Item<PendingSkim> = Item::new("pending_skim");
