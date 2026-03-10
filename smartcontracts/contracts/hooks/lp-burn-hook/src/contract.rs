//! # LP Burn Hook
//!
//! Post-swap hook that permanently burns LP tokens held by this contract,
//! proportional to each swap's output volume. Governance or the treasury
//! pre-funds the hook with LP tokens for the `target_pair`; the hook
//! gradually burns them, locking the underlying reserves in the pool and
//! increasing the value of all remaining LP shares.
//!
//! ## Mechanism
//!
//! On every `AfterSwap` callback from an allowed pair:
//!
//! 1. `burn_amount = output_amount × percentage_bps / 10_000`
//! 2. Query the hook's own LP token balance.
//! 3. Burn `min(burn_amount, balance)` LP tokens via CW20 Burn.
//! 4. If balance is zero, emit a warning attribute and skip gracefully.
//!
//! Burning LP tokens without withdrawing the underlying assets effectively
//! donates those reserves to all remaining LP holders — a deflationary
//! mechanism that deepens liquidity over time.

use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{LpBurnHookConfig, ALLOWED_PAIRS, CONFIG};
use dex_common::hook::HookExecuteMsg;

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-lp-burn-hook";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the LP burn hook with a target pair, its LP token, the burn
/// percentage, and an admin address for config updates.
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
        lp_token: deps.api.addr_validate(&msg.lp_token)?,
        percentage_bps: msg.percentage_bps,
        admin: deps.api.addr_validate(&msg.admin)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("lp_token", config.lp_token)
        .add_attribute("percentage_bps", config.percentage_bps.to_string())
        .add_attribute("admin", config.admin))
}

/// Route execute messages to their handlers. `Hook` messages are gated
/// behind the allowed-pairs check; config mutations require admin auth.
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
                } => execute_after_swap(deps, env, return_asset.amount),
            }
        }
        ExecuteMsg::UpdateConfig {
            target_pair,
            lp_token,
            percentage_bps,
        } => execute_update_config(deps, info, target_pair, lp_token, percentage_bps),
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

/// Core hook logic: burn LP tokens proportional to swap output volume.
///
/// Burns `min(output_amount * percentage_bps / 10_000, lp_balance)` LP
/// tokens from this contract's own holdings. If the hook has no LP tokens
/// left, it succeeds silently with a warning attribute so swaps are never
/// blocked by an exhausted hook.
fn execute_after_swap(
    deps: DepsMut,
    env: Env,
    output_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let target_burn = output_amount
        .checked_mul(Uint128::from(config.percentage_bps as u128))?
        .checked_div(Uint128::new(10_000))?;

    if target_burn.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "after_swap_lp_burn_hook")
            .add_attribute("skipped", "calculated burn amount is zero"));
    }

    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.lp_token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    if balance.balance.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "after_swap_lp_burn_hook")
            .add_attribute("warning", "no LP tokens available to burn")
            .add_attribute("target_burn", target_burn));
    }

    let actual_burn = std::cmp::min(target_burn, balance.balance);

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn {
            amount: actual_burn,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(burn_msg)
        .add_attribute("action", "after_swap_lp_burn_hook")
        .add_attribute("target_pair", config.target_pair)
        .add_attribute("lp_token", config.lp_token)
        .add_attribute("target_burn", target_burn)
        .add_attribute("actual_burn", actual_burn)
        .add_attribute("remaining_balance", balance.balance - actual_burn))
}

/// Update hook configuration. Admin only.
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    target_pair: Option<String>,
    lp_token: Option<String>,
    percentage_bps: Option<u16>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(pair) = target_pair {
        config.target_pair = deps.api.addr_validate(&pair)?;
    }
    if let Some(token) = lp_token {
        config.lp_token = deps.api.addr_validate(&token)?;
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
        .add_attribute("lp_token", config.lp_token)
        .add_attribute("percentage_bps", config.percentage_bps.to_string()))
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
        target_pair: config.target_pair,
        lp_token: config.lp_token,
        percentage_bps: config.percentage_bps,
        admin: config.admin,
    })
}
