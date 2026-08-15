//! Post-migrate wrap-mapper stand-in: `Config` has `fee_wrap_bps` / `fee_unwrap_bps`
//! and **no** `fee_bps` (ustr-cmm#9 / GitLab #523).
//!
//! `Receive` accepts the router unwrap Send so settlement can complete. It does
//! not InstantWithdraw native — use this for Config deserialize + R3 fee pick.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw20::Cw20ReceiveMsg;
use cw_multi_test::{Contract, ContractWrapper};
use cw_storage_plus::Item;

#[cw_serde]
pub struct InstantiateMsg {
    pub governance: String,
    pub treasury: String,
    pub fee_wrap_bps: u16,
    pub fee_unwrap_bps: u16,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
}

/// Split-only shape — serializing this must not emit `fee_bps`.
#[cw_serde]
pub struct SplitConfigResponse {
    pub governance: Addr,
    pub treasury: Addr,
    pub paused: bool,
    pub fee_wrap_bps: u16,
    pub fee_unwrap_bps: u16,
}

#[cw_serde]
struct State {
    governance: Addr,
    treasury: Addr,
    fee_wrap_bps: u16,
    fee_unwrap_bps: u16,
}

const STATE: Item<State> = Item::new("s");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    STATE.save(
        deps.storage,
        &State {
            governance: deps.api.addr_validate(&msg.governance)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            fee_wrap_bps: msg.fee_wrap_bps,
            fee_unwrap_bps: msg.fee_unwrap_bps,
        },
    )?;
    Ok(Response::default())
}

pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(_) => Ok(Response::new().add_attribute("action", "mock_unwrap")),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => {
            let s = STATE.load(deps.storage)?;
            to_json_binary(&SplitConfigResponse {
                governance: s.governance,
                treasury: s.treasury,
                paused: false,
                fee_wrap_bps: s.fee_wrap_bps,
                fee_unwrap_bps: s.fee_unwrap_bps,
            })
        }
    }
}

pub fn mock_split_fee_wrap_mapper_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query))
}
