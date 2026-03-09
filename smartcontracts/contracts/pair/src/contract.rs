use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

use crate::error::ContractError;
use crate::msg::{
    Cw20HookMsg, ExecuteMsg, FeeConfigResponse, HooksResponse, InstantiateMsg, PairInfoResponse,
    QueryMsg, ReservesResponse, SimulateSwapResponse,
};
use crate::state::{PairInfoState, FEE_CONFIG, HOOKS, PAIR_INFO, RESERVES, TOTAL_LP_SUPPLY};
use dex_common::hook::HookExecuteMsg;
use dex_common::types::{FeeConfig, PairInfo};

const CONTRACT_NAME: &str = "cl8y-dex-pair";
const CONTRACT_VERSION: &str = "1.0.0";
const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;

fn isqrt(n: Uint128) -> Uint128 {
    if n.is_zero() {
        return Uint128::zero();
    }
    let mut x = n;
    let mut y = (x + Uint128::one()) / Uint128::new(2);
    while y < x {
        x = y;
        y = (x + n / x) / Uint128::new(2);
    }
    x
}

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let pair_info = PairInfoState {
        token_a: msg.token_a.clone(),
        token_b: msg.token_b.clone(),
        lp_token: Addr::unchecked(""),
        factory: msg.factory.clone(),
    };
    PAIR_INFO.save(deps.storage, &pair_info)?;

    let fee_config = FeeConfig {
        fee_bps: msg.fee_bps,
        treasury: msg.treasury,
    };
    FEE_CONFIG.save(deps.storage, &fee_config)?;

    RESERVES.save(deps.storage, &(Uint128::zero(), Uint128::zero()))?;
    HOOKS.save(deps.storage, &vec![])?;
    TOTAL_LP_SUPPLY.save(deps.storage, &Uint128::zero())?;

    let instantiate_lp_msg = cw20_base::msg::InstantiateMsg {
        name: "CL8Y DEX LP Token".to_string(),
        symbol: "CLDY-LP".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: env.contract.address.to_string(),
            cap: None,
        }),
        marketing: None,
    };

    let sub_msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: None,
            code_id: msg.lp_token_code_id,
            msg: to_json_binary(&instantiate_lp_msg)?,
            funds: vec![],
            label: "CL8Y DEX LP Token".to_string(),
        },
        INSTANTIATE_LP_TOKEN_REPLY_ID,
    );

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "instantiate")
        .add_attribute("token_a", msg.token_a)
        .add_attribute("token_b", msg.token_b))
}

pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_LP_TOKEN_REPLY_ID => {
            let res = cw_utils::parse_reply_instantiate_data(msg)
                .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;
            let lp_token_addr = deps.api.addr_validate(&res.contract_address)?;

            PAIR_INFO.update(deps.storage, |mut info| -> StdResult<_> {
                info.lp_token = lp_token_addr.clone();
                Ok(info)
            })?;

            Ok(Response::new().add_attribute("lp_token", lp_token_addr))
        }
        _ => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "unknown reply id",
        ))),
    }
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => execute_receive(deps, env, info, cw20_msg),
        ExecuteMsg::AddLiquidity {
            token_a_amount,
            token_b_amount,
            min_lp_tokens,
            slippage_tolerance: _,
        } => execute_add_liquidity(deps, env, info, token_a_amount, token_b_amount, min_lp_tokens),
        ExecuteMsg::UpdateFee { fee_bps } => execute_update_fee(deps, info, fee_bps),
        ExecuteMsg::UpdateHooks { hooks } => execute_update_hooks(deps, info, hooks),
    }
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook_msg: Cw20HookMsg = cosmwasm_std::from_json(&cw20_msg.msg)?;
    let token_sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match hook_msg {
        Cw20HookMsg::Swap { min_output, to } => {
            execute_swap(deps, env, info, token_sender, cw20_msg.amount, min_output, to)
        }
        Cw20HookMsg::RemoveLiquidity { min_a, min_b } => {
            execute_remove_liquidity(deps, env, info, token_sender, cw20_msg.amount, min_a, min_b)
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    input_amount: Uint128,
    min_output: Option<Uint128>,
    to: Option<String>,
) -> Result<Response, ContractError> {
    if input_amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let (input_reserve, output_reserve, input_token, output_token) =
        if info.sender == pair_info.token_a {
            (reserve_a, reserve_b, pair_info.token_a.clone(), pair_info.token_b.clone())
        } else if info.sender == pair_info.token_b {
            (reserve_b, reserve_a, pair_info.token_b.clone(), pair_info.token_a.clone())
        } else {
            return Err(ContractError::InvalidToken {});
        };

    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    let k = input_reserve.checked_mul(output_reserve)?;
    let new_input_reserve = input_reserve.checked_add(input_amount)?;
    let new_output_reserve = k.checked_div(new_input_reserve)?;
    let gross_output = output_reserve.checked_sub(new_output_reserve)?;

    let fee_amount = gross_output
        .checked_mul(Uint128::new(fee_config.fee_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    let net_output = gross_output.checked_sub(fee_amount)?;

    if let Some(min) = min_output {
        if net_output < min {
            return Err(ContractError::MinimumOutputNotMet {
                min: min.to_string(),
                actual: net_output.to_string(),
            });
        }
    }

    let (new_reserve_a, new_reserve_b) = if info.sender == pair_info.token_a {
        (
            reserve_a.checked_add(input_amount)?,
            reserve_b.checked_sub(gross_output)?,
        )
    } else {
        (
            reserve_a.checked_sub(gross_output)?,
            reserve_b.checked_add(input_amount)?,
        )
    };
    RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

    let recipient = match to {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => sender.clone(),
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    if !fee_amount.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: output_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.treasury.to_string(),
                amount: fee_amount,
            })?,
            funds: vec![],
        }));
    }

    if !net_output.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: output_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: net_output,
            })?,
            funds: vec![],
        }));
    }

    let hooks = HOOKS.load(deps.storage)?;
    for hook in hooks {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.to_string(),
            msg: to_json_binary(&HookExecuteMsg::AfterSwap {
                pair: env.contract.address.clone(),
                sender: sender.clone(),
                input_token: input_token.clone(),
                input_amount,
                output_token: output_token.clone(),
                output_amount: net_output,
                fee_amount,
            })?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "swap")
        .add_attribute("sender", sender)
        .add_attribute("input_token", input_token)
        .add_attribute("input_amount", input_amount)
        .add_attribute("output_token", output_token)
        .add_attribute("output_amount", net_output)
        .add_attribute("fee_amount", fee_amount))
}

fn execute_add_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_a_amount: Uint128,
    token_b_amount: Uint128,
    min_lp_tokens: Option<Uint128>,
) -> Result<Response, ContractError> {
    if token_a_amount.is_zero() || token_b_amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let total_supply = TOTAL_LP_SUPPLY.load(deps.storage)?;

    let lp_tokens = if reserve_a.is_zero() && reserve_b.is_zero() {
        isqrt(token_a_amount.checked_mul(token_b_amount)?)
    } else {
        let lp_a = token_a_amount
            .checked_mul(total_supply)?
            .checked_div(reserve_a)?;
        let lp_b = token_b_amount
            .checked_mul(total_supply)?
            .checked_div(reserve_b)?;
        std::cmp::min(lp_a, lp_b)
    };

    if lp_tokens.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    if let Some(min) = min_lp_tokens {
        if lp_tokens < min {
            return Err(ContractError::InsufficientLpTokens {
                min: min.to_string(),
                actual: lp_tokens.to_string(),
            });
        }
    }

    let new_reserve_a = reserve_a.checked_add(token_a_amount)?;
    let new_reserve_b = reserve_b.checked_add(token_b_amount)?;
    RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

    let new_total_supply = total_supply.checked_add(lp_tokens)?;
    TOTAL_LP_SUPPLY.save(deps.storage, &new_total_supply)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.token_a.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: token_a_amount,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.token_b.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: token_b_amount,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: info.sender.to_string(),
            amount: lp_tokens,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "add_liquidity")
        .add_attribute("sender", info.sender)
        .add_attribute("token_a_amount", token_a_amount)
        .add_attribute("token_b_amount", token_b_amount)
        .add_attribute("lp_tokens_minted", lp_tokens))
}

fn execute_remove_liquidity(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Addr,
    lp_amount: Uint128,
    min_a: Option<Uint128>,
    min_b: Option<Uint128>,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;

    if info.sender != pair_info.lp_token {
        return Err(ContractError::InvalidToken {});
    }

    if lp_amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let total_supply = TOTAL_LP_SUPPLY.load(deps.storage)?;

    if total_supply.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    let amount_a = lp_amount
        .checked_mul(reserve_a)?
        .checked_div(total_supply)?;
    let amount_b = lp_amount
        .checked_mul(reserve_b)?
        .checked_div(total_supply)?;

    if let Some(min) = min_a {
        if amount_a < min {
            return Err(ContractError::MinimumOutputNotMet {
                min: min.to_string(),
                actual: amount_a.to_string(),
            });
        }
    }
    if let Some(min) = min_b {
        if amount_b < min {
            return Err(ContractError::MinimumOutputNotMet {
                min: min.to_string(),
                actual: amount_b.to_string(),
            });
        }
    }

    let new_reserve_a = reserve_a.checked_sub(amount_a)?;
    let new_reserve_b = reserve_b.checked_sub(amount_b)?;
    RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

    let new_total_supply = total_supply.checked_sub(lp_amount)?;
    TOTAL_LP_SUPPLY.save(deps.storage, &new_total_supply)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount: lp_amount })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.token_a.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender.to_string(),
            amount: amount_a,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.token_b.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender.to_string(),
            amount: amount_b,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "remove_liquidity")
        .add_attribute("sender", sender)
        .add_attribute("lp_burned", lp_amount)
        .add_attribute("token_a_returned", amount_a)
        .add_attribute("token_b_returned", amount_b))
}

fn execute_update_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee_bps: u16,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    if fee_bps > 10000 {
        return Err(ContractError::InvalidFee {
            reason: "fee_bps must be <= 10000".to_string(),
        });
    }

    FEE_CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.fee_bps = fee_bps;
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_fee")
        .add_attribute("fee_bps", fee_bps.to_string()))
}

fn execute_update_hooks(
    deps: DepsMut,
    info: MessageInfo,
    hooks: Vec<String>,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    let validated_hooks: Vec<Addr> = hooks
        .iter()
        .map(|h| deps.api.addr_validate(h))
        .collect::<StdResult<Vec<Addr>>>()?;

    HOOKS.save(deps.storage, &validated_hooks)?;

    Ok(Response::new()
        .add_attribute("action", "update_hooks")
        .add_attribute("hooks_count", validated_hooks.len().to_string()))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPairInfo {} => to_json_binary(&query_pair_info(deps)?),
        QueryMsg::GetReserves {} => to_json_binary(&query_reserves(deps)?),
        QueryMsg::GetFeeConfig {} => to_json_binary(&query_fee_config(deps)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::SimulateSwap {
            offer_token,
            offer_amount,
        } => to_json_binary(&query_simulate_swap(deps, offer_token, offer_amount)?),
    }
}

fn query_pair_info(deps: Deps) -> StdResult<PairInfoResponse> {
    let state = PAIR_INFO.load(deps.storage)?;
    Ok(PairInfoResponse {
        pair: PairInfo {
            token_a: state.token_a,
            token_b: state.token_b,
            pair_contract: Addr::unchecked(""),
            lp_token: state.lp_token,
        },
    })
}

fn query_reserves(deps: Deps) -> StdResult<ReservesResponse> {
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    Ok(ReservesResponse {
        reserve_a,
        reserve_b,
    })
}

fn query_fee_config(deps: Deps) -> StdResult<FeeConfigResponse> {
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    Ok(FeeConfigResponse { fee_config })
}

fn query_hooks(deps: Deps) -> StdResult<HooksResponse> {
    let hooks = HOOKS.load(deps.storage)?;
    Ok(HooksResponse { hooks })
}

fn query_simulate_swap(
    deps: Deps,
    offer_token: String,
    offer_amount: Uint128,
) -> StdResult<SimulateSwapResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let offer_addr = deps.api.addr_validate(&offer_token)?;

    let (input_reserve, output_reserve) = if offer_addr == pair_info.token_a {
        (reserve_a, reserve_b)
    } else if offer_addr == pair_info.token_b {
        (reserve_b, reserve_a)
    } else {
        return Err(cosmwasm_std::StdError::generic_err("Invalid offer token"));
    };

    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Ok(SimulateSwapResponse {
            return_amount: Uint128::zero(),
            fee_amount: Uint128::zero(),
            spread_amount: Uint128::zero(),
        });
    }

    let k = input_reserve.checked_mul(output_reserve)?;
    let new_input_reserve = input_reserve.checked_add(offer_amount)?;
    let new_output_reserve = k.checked_div(new_input_reserve)?;
    let gross_output = output_reserve.checked_sub(new_output_reserve)?;

    let fee_amount = gross_output
        .checked_mul(Uint128::new(fee_config.fee_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    let net_output = gross_output.checked_sub(fee_amount)?;

    let ideal_output = offer_amount
        .checked_mul(output_reserve)?
        .checked_div(input_reserve)?;
    let spread_amount = if ideal_output > gross_output {
        ideal_output.checked_sub(gross_output)?
    } else {
        Uint128::zero()
    };

    Ok(SimulateSwapResponse {
        return_amount: net_output,
        fee_amount,
        spread_amount,
    })
}
