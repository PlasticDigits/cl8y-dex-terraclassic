//! Manager-approved AutoLP economics (#1398 / #1429). Wire/storage identity is
//! shared so recovery approves the exact anchor, never just mutable spot price.
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct LpPolicy {
    pub pair: Addr,
    pub quote: Addr,
    pub lp_token: Addr,
    pub tax_reserve: Uint128,
    pub quote_reserve: Uint128,
    pub total_share: Uint128,
}
