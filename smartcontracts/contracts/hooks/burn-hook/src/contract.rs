use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{BurnHookConfig, ALLOWED_PAIRS, CONFIG};
use dex_common::hook::HookExecuteMsg;

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-burn-hook";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.burn_percentage_bps > 10000 {
        return Err(ContractError::InvalidBps {
            value: msg.burn_percentage_bps,
        });
    }

    let config = BurnHookConfig {
        burn_token: deps.api.addr_validate(&msg.burn_token)?,
        burn_percentage_bps: msg.burn_percentage_bps,
        admin: deps.api.addr_validate(&msg.admin)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("burn_token", config.burn_token)
        .add_attribute("burn_percentage_bps", config.burn_percentage_bps.to_string())
        .add_attribute("admin", config.admin))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
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
                } => execute_after_swap(deps, env, return_asset.info.to_string(), return_asset.amount),
            }
        }
        ExecuteMsg::UpdateConfig {
            burn_token,
            burn_percentage_bps,
        } => execute_update_config(deps, info, burn_token, burn_percentage_bps),
        ExecuteMsg::UpdateAllowedPairs { add, remove } => {
            execute_update_allowed_pairs(deps, info, add, remove)
        }
    }
}

/// Reject callers that are not registered as allowed pairs.
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

/// Burn a percentage of the output token from this contract's balance.
/// Skips gracefully if the output token doesn't match `burn_token`,
/// the calculated amount is zero, or the balance is insufficient.
fn execute_after_swap(
    deps: DepsMut,
    env: Env,
    output_token: String,
    output_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if output_token != config.burn_token {
        return Ok(Response::new()
            .add_attribute("action", "after_swap_burn_hook")
            .add_attribute("skipped", "output_token does not match burn_token"));
    }

    let burn_amount = output_amount
        .checked_mul(Uint128::from(config.burn_percentage_bps as u128))?
        .checked_div(Uint128::new(10_000))?;

    if burn_amount.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "after_swap_burn_hook")
            .add_attribute("skipped", "burn_amount is zero"));
    }

    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.burn_token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    if balance.balance < burn_amount {
        return Ok(Response::new()
            .add_attribute("action", "after_swap_burn_hook")
            .add_attribute("warning", "insufficient balance to burn")
            .add_attribute("required", burn_amount)
            .add_attribute("available", balance.balance));
    }

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.burn_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn {
            amount: burn_amount,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(burn_msg)
        .add_attribute("action", "after_swap_burn_hook")
        .add_attribute("burn_token", config.burn_token)
        .add_attribute("burn_amount", burn_amount))
}

/// Update burn hook configuration. Admin only.
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    burn_token: Option<String>,
    burn_percentage_bps: Option<u16>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(token) = burn_token {
        config.burn_token = deps.api.addr_validate(&token)?;
    }
    if let Some(bps) = burn_percentage_bps {
        if bps > 10000 {
            return Err(ContractError::InvalidBps { value: bps });
        }
        config.burn_percentage_bps = bps;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("burn_token", config.burn_token)
        .add_attribute("burn_percentage_bps", config.burn_percentage_bps.to_string()))
}

/// Add or remove pair contracts from the allowed callers list. Admin only.
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
        burn_token: config.burn_token,
        burn_percentage_bps: config.burn_percentage_bps,
        admin: config.admin,
    })
}
