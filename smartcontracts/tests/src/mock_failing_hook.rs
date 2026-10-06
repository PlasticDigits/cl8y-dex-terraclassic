//! Post-swap hook that always errors after caller check — for atomicity / griefing regressions.
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult};
use cw_multi_test::{Contract, ContractWrapper};
use cw_storage_plus::Item;
use dex_common::hook::HookCallMsg;

const ALLOWED_PAIR: Item<String> = Item::new("p");

#[cw_serde]
pub struct InstantiateMsg {
    /// Only this pair may invoke `Hook`; invocation always fails (simulates reverting hook).
    pub pair: String,
}

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    deps.api.addr_validate(&msg.pair)?;
    ALLOWED_PAIR.save(deps.storage, &msg.pair)?;
    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HookCallMsg,
) -> StdResult<Response> {
    let allowed = ALLOWED_PAIR.load(deps.storage)?;
    if info.sender.as_str() != allowed.as_str() {
        return Err(StdError::generic_err("unauthorized mock hook caller"));
    }
    match msg {
        HookCallMsg::Hook(_) => Err(StdError::generic_err("mock hook failure")),
    }
}

pub fn query(deps: Deps, _env: Env, _msg: cl8y_dex_burn_hook::msg::QueryMsg) -> StdResult<Binary> {
    cosmwasm_std::to_json_binary(&cl8y_dex_burn_hook::msg::ConfigResponse {
        burn_token: deps.api.addr_validate(&ALLOWED_PAIR.load(deps.storage)?)?,
        burn_percentage_bps: 0,
        admin: cosmwasm_std::Addr::unchecked("admin"),
    })
}

pub fn mock_failing_hook_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query))
}
