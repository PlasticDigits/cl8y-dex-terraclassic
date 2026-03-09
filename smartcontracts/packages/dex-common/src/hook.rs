use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub enum HookExecuteMsg {
    AfterSwap {
        pair: Addr,
        sender: Addr,
        input_token: Addr,
        input_amount: Uint128,
        output_token: Addr,
        output_amount: Uint128,
        fee_amount: Uint128,
    },
}
