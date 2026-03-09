use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{BurnHookConfig, CONFIG};
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
        ExecuteMsg::Hook(hook_msg) => match hook_msg {
            HookExecuteMsg::AfterSwap {
                pair: _,
                sender: _,
                input_token: _,
                input_amount: _,
                output_token,
                output_amount,
                fee_amount: _,
            } => execute_after_swap(deps, env, output_token.to_string(), output_amount),
        },
        ExecuteMsg::UpdateConfig {
            burn_token,
            burn_percentage_bps,
        } => execute_update_config(deps, info, burn_token, burn_percentage_bps),
    }
}

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
        .checked_mul(Uint128::from(config.burn_percentage_bps as u128))
        .unwrap_or(Uint128::zero())
        / Uint128::from(10_000u128);

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
        config.burn_percentage_bps = bps;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("burn_token", config.burn_token)
        .add_attribute("burn_percentage_bps", config.burn_percentage_bps.to_string()))
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
