use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{LpBurnHookConfig, CONFIG};
use dex_common::hook::HookExecuteMsg;

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-lp-burn-hook";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = LpBurnHookConfig {
        target_pair: deps.api.addr_validate(&msg.target_pair)?,
        percentage_bps: msg.percentage_bps,
        admin: deps.api.addr_validate(&msg.admin)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("percentage_bps", config.percentage_bps.to_string())
        .add_attribute("admin", config.admin))
}

pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Hook(hook_msg) => match hook_msg {
            HookExecuteMsg::AfterSwap {
                pair: _,
                sender: _,
                input_token: _,
                input_amount: _,
                output_token: _,
                output_amount,
                fee_amount: _,
            } => execute_after_swap(deps, output_amount),
        },
        ExecuteMsg::UpdateConfig {
            target_pair,
            percentage_bps,
        } => execute_update_config(deps, info, target_pair, percentage_bps),
    }
}

// TODO: Full implementation will acquire both tokens of the target pair,
// add proportional liquidity, and burn the resulting LP tokens for permanent liquidity.
// V1 emits an event recording the hook invocation.
fn execute_after_swap(
    deps: DepsMut,
    output_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let lp_burn_amount = output_amount
        .checked_mul(Uint128::from(config.percentage_bps as u128))
        .unwrap_or(Uint128::zero())
        / Uint128::from(10_000u128);

    Ok(Response::new()
        .add_attribute("action", "after_swap_lp_burn_hook")
        .add_attribute("status", "v1_stub")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("calculated_amount", lp_burn_amount)
        .add_attribute("note", "full lp-add-and-burn not yet implemented"))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    target_pair: Option<String>,
    percentage_bps: Option<u16>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(pair) = target_pair {
        config.target_pair = deps.api.addr_validate(&pair)?;
    }
    if let Some(bps) = percentage_bps {
        config.percentage_bps = bps;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("percentage_bps", config.percentage_bps.to_string()))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        target_pair: config.target_pair,
        percentage_bps: config.percentage_bps,
        admin: config.admin,
    })
}
