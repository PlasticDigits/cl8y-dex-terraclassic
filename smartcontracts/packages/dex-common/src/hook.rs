use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use crate::types::Asset;

#[cw_serde]
pub enum HookExecuteMsg {
    AfterSwap {
        pair: Addr,
        sender: Addr,
        offer_asset: Asset,
        return_asset: Asset,
        commission_amount: Uint128,
        spread_amount: Uint128,
    },
}
