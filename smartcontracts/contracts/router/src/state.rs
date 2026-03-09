use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

use crate::msg::SwapOperation;

pub const FACTORY: Item<Addr> = Item::new("factory");

#[cw_serde]
pub struct SwapState {
    pub sender: Addr,
    pub recipient: Addr,
    pub remaining_operations: Vec<SwapOperation>,
    pub minimum_receive: Option<Uint128>,
    pub output_token: Addr,
}

pub const SWAP_STATE: Item<SwapState> = Item::new("swap_state");
