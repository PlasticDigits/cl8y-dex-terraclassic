use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{LpBurnHookConfig, ALLOWED_PAIRS, CONFIG};
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

    if msg.percentage_bps > 10000 {
        return Err(ContractError::InvalidBps {
            value: msg.percentage_bps,
        });
    }

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
        ExecuteMsg::Hook(hook_msg) => {
            assert_allowed_pair(deps.as_ref(), &info)?;
            match hook_msg {
                HookExecuteMsg::AfterSwap {
                    pair: _,
                    sender: _,
                    offer_asset: _,
                    return_asset,
                    commission_amount: _,
                    spread_amount: _,
                } => execute_after_swap(deps, return_asset.amount),
            }
        }
        ExecuteMsg::UpdateConfig {
            target_pair,
            percentage_bps,
        } => execute_update_config(deps, info, target_pair, percentage_bps),
        ExecuteMsg::UpdateAllowedPairs { add, remove } => {
            execute_update_allowed_pairs(deps, info, add, remove)
        }
    }
}

fn assert_allowed_pair(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    if !ALLOWED_PAIRS
        .may_load(deps.storage, info.sender.as_str())?
        .unwrap_or(false)
    {
        return Err(ContractError::UnauthorizedHookCaller {
            sender: info.sender.to_string(),
        });
    }
    Ok(())
}

fn execute_after_swap(
    deps: DepsMut,
    output_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let lp_burn_amount = output_amount
        .checked_mul(Uint128::from(config.percentage_bps as u128))?
        .checked_div(Uint128::new(10_000))?;

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
        if bps > 10000 {
            return Err(ContractError::InvalidBps { value: bps });
        }
        config.percentage_bps = bps;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("percentage_bps", config.percentage_bps.to_string()))
}

fn execute_update_allowed_pairs(
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<String>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    for pair in &add {
        let addr = deps.api.addr_validate(pair)?;
        ALLOWED_PAIRS.save(deps.storage, addr.as_str(), &true)?;
    }
    for pair in &remove {
        let addr = deps.api.addr_validate(pair)?;
        ALLOWED_PAIRS.remove(deps.storage, addr.as_str());
    }

    Ok(Response::new()
        .add_attribute("action", "update_allowed_pairs")
        .add_attribute("added", add.len().to_string())
        .add_attribute("removed", remove.len().to_string()))
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
