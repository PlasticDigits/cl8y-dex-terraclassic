//! Test-only LP CW20 that enforces classic Terraswap ticker rules (GitLab #518).
//!
//! columbus-5 classic `lp_token_code_id` rejects digits (`[a-zA-Z\-]{3,12}`).
//! Workspace `cw20-mintable` allows `[a-zA-Z0-9\-]`. This contract reproduces
//! the pre-upgrade revert so tests prove the factory LP code-id rotation.

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};
use cw_multi_test::{Contract, ContractWrapper};
use cw_storage_plus::Item;
use dex_common::lp_symbol::is_classic_cw20_lp_symbol;

const TOKEN_INFO: Item<TokenInfoResponse> = Item::new("t");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: cw20_mintable::msg::InstantiateMsg,
) -> StdResult<Response> {
    if !is_classic_cw20_lp_symbol(&msg.symbol) {
        return Err(StdError::generic_err(
            "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
        ));
    }
    let name_len = msg.name.as_bytes().len();
    if name_len < 3 || name_len > 50 {
        return Err(StdError::generic_err(
            "Name is not in the expected format (3-50 UTF-8 bytes)",
        ));
    }
    TOKEN_INFO.save(
        deps.storage,
        &TokenInfoResponse {
            name: msg.name,
            symbol: msg.symbol,
            decimals: msg.decimals,
            total_supply: Uint128::zero(),
        },
    )?;
    Ok(Response::new())
}

pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Err(StdError::generic_err(
        "classic_lp_cw20 execute is unused in #518 create_pair tests",
    ))
}

pub fn query(deps: Deps, _env: Env, msg: Cw20QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw20QueryMsg::TokenInfo {} => to_json_binary(&TOKEN_INFO.load(deps.storage)?),
        _ => Err(StdError::generic_err("unsupported classic_lp_cw20 query")),
    }
}

pub fn classic_lp_cw20_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query))
}
