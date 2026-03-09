use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

use crate::error::ContractError;
use crate::msg::{
    Cw20HookMsg, ExecuteMsg, FeeConfigResponse, HooksResponse, InstantiateMsg, ObserveResponse,
    OracleInfoResponse, PoolResponse, QueryMsg, ReverseSimulationResponse, SimulationResponse,
};
use crate::state::{
    OracleState, PairInfoState, FEE_CONFIG, HOOKS, OBSERVATIONS, ORACLE_STATE, PAIR_INFO,
    PAUSED, RESERVES, TOTAL_LP_SUPPLY,
};
use dex_common::hook::HookExecuteMsg;
use dex_common::oracle::{
    log2_ratio_q64, Observation, DEFAULT_OBSERVATION_CARDINALITY, MAX_OBSERVATION_CARDINALITY,
};
use dex_common::types::{Asset, AssetInfo, FeeConfig};

const CONTRACT_NAME: &str = "cl8y-dex-pair";
const CONTRACT_VERSION: &str = "1.2.0";
const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;
const MINIMUM_LIQUIDITY: u128 = 1_000;

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

fn token_addr(info: &AssetInfo) -> &str {
    match info {
        AssetInfo::Token { contract_addr } => contract_addr.as_str(),
        AssetInfo::NativeToken { .. } => unreachable!("native tokens not supported"),
    }
}

fn assert_deadline(env: &Env, deadline: Option<u64>) -> Result<(), ContractError> {
    if let Some(dl) = deadline {
        let current = env.block.time.seconds();
        if current > dl {
            return Err(ContractError::DeadlineExceeded {
                deadline: dl,
                current,
            });
        }
    }
    Ok(())
}

fn assert_not_paused(storage: &dyn cosmwasm_std::Storage) -> Result<(), ContractError> {
    if PAUSED.may_load(storage)?.unwrap_or(false) {
        return Err(ContractError::Paused {});
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// TWAP Oracle — internal helpers
// ---------------------------------------------------------------------------
// The oracle samples the *current* reserves (before mutation) on every
// state-changing action. This is critical for manipulation resistance:
// an attacker's trade in this block does NOT influence the observation
// recorded for this block.

/// Write a new observation into the ring buffer if the block timestamp has
/// advanced since the last write. Called at the **top** of every execute
/// path that mutates reserves.
fn oracle_update(
    storage: &mut dyn cosmwasm_std::Storage,
    block_time: u64,
    reserve_a: Uint128,
    reserve_b: Uint128,
) -> Result<(), ContractError> {
    if reserve_a.is_zero() || reserve_b.is_zero() {
        return Ok(());
    }

    let mut state = ORACLE_STATE.load(storage)?;
    let last_obs = OBSERVATIONS.may_load(storage, state.index)?;

    let (last_ts, last_tick_cum) = match last_obs {
        Some(obs) => (obs.timestamp, obs.tick_cumulative),
        None => (0u64, 0i128),
    };

    if block_time <= last_ts {
        return Ok(());
    }

    let dt = (block_time - last_ts) as i128;
    let tick = log2_ratio_q64(reserve_b, reserve_a).map_err(|e| ContractError::Oracle {
        reason: e.to_string(),
    })?;
    let new_tick_cumulative = last_tick_cum.wrapping_add(tick.wrapping_mul(dt));

    let new_index = if state.cardinality_initialized < state.cardinality {
        state.cardinality_initialized
    } else {
        (state.index + 1) % state.cardinality
    };

    OBSERVATIONS.save(
        storage,
        new_index,
        &Observation {
            timestamp: block_time,
            tick_cumulative: new_tick_cumulative,
        },
    )?;

    state.index = new_index;
    if state.cardinality_initialized < state.cardinality {
        state.cardinality_initialized += 1;
    }
    ORACLE_STATE.save(storage, &state)?;

    Ok(())
}

/// Binary search the ring buffer for the two observations bracketing the
/// target timestamp, then linearly interpolate the cumulative tick.
fn oracle_observe_single(
    storage: &dyn cosmwasm_std::Storage,
    block_time: u64,
    seconds_ago: u32,
    state: &OracleState,
    latest_obs: &Observation,
    reserve_a: Uint128,
    reserve_b: Uint128,
) -> Result<i128, ContractError> {
    let target = block_time - seconds_ago as u64;

    if seconds_ago == 0 || target >= latest_obs.timestamp {
        if target == latest_obs.timestamp {
            return Ok(latest_obs.tick_cumulative);
        }
        // Extrapolate forward from latest observation to `target` using current reserves.
        if reserve_a.is_zero() || reserve_b.is_zero() {
            return Ok(latest_obs.tick_cumulative);
        }
        let dt = (target - latest_obs.timestamp) as i128;
        let tick =
            log2_ratio_q64(reserve_b, reserve_a).map_err(|e| ContractError::Oracle {
                reason: e.to_string(),
            })?;
        return Ok(latest_obs.tick_cumulative.wrapping_add(tick.wrapping_mul(dt)));
    }

    let n = state.cardinality_initialized;
    if n < 2 {
        return Err(ContractError::Oracle {
            reason: "not enough observations for the requested window".into(),
        });
    }

    // Oldest observation in the ring buffer.
    let oldest_idx = if n < state.cardinality {
        0u16
    } else {
        (state.index + 1) % state.cardinality
    };
    let oldest = OBSERVATIONS.load(storage, oldest_idx)?;
    if target < oldest.timestamp {
        return Err(ContractError::Oracle {
            reason: format!(
                "observation window too old: requested {}s ago but oldest is {}s ago",
                seconds_ago,
                block_time - oldest.timestamp
            ),
        });
    }

    // Binary search for the two observations bracketing `target`.
    let mut lo: u16 = 0;
    let mut hi: u16 = n - 1;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        let mid_idx = (oldest_idx + mid) % state.cardinality;
        let obs = OBSERVATIONS.load(storage, mid_idx)?;
        if obs.timestamp <= target {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    let before_idx = (oldest_idx + lo) % state.cardinality;
    let before = OBSERVATIONS.load(storage, before_idx)?;

    if before.timestamp == target {
        return Ok(before.tick_cumulative);
    }

    let after_idx = (oldest_idx + lo + 1) % state.cardinality;
    let after = if (lo + 1) < n {
        OBSERVATIONS.load(storage, after_idx)?
    } else {
        latest_obs.clone()
    };

    let time_span = (after.timestamp - before.timestamp) as i128;
    let tick_span = after.tick_cumulative.wrapping_sub(before.tick_cumulative);
    let dt = (target - before.timestamp) as i128;

    Ok(before
        .tick_cumulative
        .wrapping_add(tick_span.wrapping_mul(dt) / time_span))
}

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    for ai in &msg.asset_infos {
        ai.assert_is_token()
            .map_err(ContractError::Std)?;
    }

    let pair_info = PairInfoState {
        asset_infos: msg.asset_infos.clone(),
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
    PAUSED.save(deps.storage, &false)?;

    ORACLE_STATE.save(
        deps.storage,
        &OracleState {
            cardinality: DEFAULT_OBSERVATION_CARDINALITY,
            index: 0,
            cardinality_initialized: 0,
        },
    )?;

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
        .add_attribute("pair", format!("{}-{}", msg.asset_infos[0], msg.asset_infos[1])))
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

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => {
            assert_not_paused(deps.storage)?;
            execute_receive(deps, env, info, cw20_msg)
        }
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            receiver,
            deadline,
        } => {
            assert_not_paused(deps.storage)?;
            assert_deadline(&env, deadline)?;
            execute_provide_liquidity(deps, env, info, assets, slippage_tolerance, receiver)
        }
        ExecuteMsg::Swap {
            offer_asset,
            belief_price: _,
            max_spread: _,
            to: _,
            deadline: _,
        } => {
            offer_asset.info.assert_is_token()
                .map_err(|_| ContractError::NativeTokenNotSupported {})?;
            Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Direct Swap execute is not supported for CW20 tokens; use CW20 Send with Cw20HookMsg::Swap instead"
            )))
        }
        ExecuteMsg::UpdateFee { fee_bps } => execute_update_fee(deps, info, fee_bps),
        ExecuteMsg::UpdateHooks { hooks } => execute_update_hooks(deps, info, hooks),
        ExecuteMsg::IncreaseObservationCardinality { new_cardinality } => {
            execute_increase_observation_cardinality(deps, new_cardinality)
        }
        ExecuteMsg::SetPaused { paused } => execute_set_paused(deps, info, paused),
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
        Cw20HookMsg::Swap {
            belief_price,
            max_spread,
            to,
            deadline,
        } => {
            assert_deadline(&env, deadline)?;
            execute_swap(
                deps,
                env,
                info,
                token_sender,
                cw20_msg.amount,
                belief_price,
                max_spread,
                to,
            )
        }
        Cw20HookMsg::WithdrawLiquidity {} => {
            execute_withdraw_liquidity(deps, env, info, token_sender, cw20_msg.amount)
        }
    }
}

fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_amount: Uint128,
    return_amount: Uint128,
    spread_amount: Uint128,
    commission_amount: Uint128,
) -> Result<(), ContractError> {
    let default_spread = Decimal::percent(1);
    let max_allowed = max_spread.unwrap_or(default_spread);

    if let Some(bp) = belief_price {
        let expected_return = offer_amount * (Decimal::one() / bp);
        let actual_return = return_amount + commission_amount;
        let spread = if expected_return > actual_return {
            expected_return - actual_return
        } else {
            Uint128::zero()
        };

        if expected_return > Uint128::zero()
            && Decimal::from_ratio(spread, expected_return) > max_allowed
        {
            return Err(ContractError::MaxSpreadAssertion {
                max: max_allowed.to_string(),
                actual: Decimal::from_ratio(spread, expected_return).to_string(),
            });
        }
    } else {
        let total_return = return_amount + commission_amount;
        if total_return > Uint128::zero()
            && Decimal::from_ratio(spread_amount, total_return) > max_allowed
        {
            return Err(ContractError::MaxSpreadAssertion {
                max: max_allowed.to_string(),
                actual: Decimal::from_ratio(spread_amount, total_return).to_string(),
            });
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    input_amount: Uint128,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<String>,
) -> Result<Response, ContractError> {
    if input_amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    // Record observation BEFORE reserves change — critical for manipulation resistance.
    oracle_update(
        deps.storage,
        env.block.time.seconds(),
        reserve_a,
        reserve_b,
    )?;

    let offer_token_addr = info.sender.to_string();
    let token_a_addr = token_addr(&pair_info.asset_infos[0]);
    let token_b_addr = token_addr(&pair_info.asset_infos[1]);

    let (input_reserve, output_reserve, offer_asset_info, ask_asset_info) =
        if offer_token_addr == token_a_addr {
            (
                reserve_a,
                reserve_b,
                pair_info.asset_infos[0].clone(),
                pair_info.asset_infos[1].clone(),
            )
        } else if offer_token_addr == token_b_addr {
            (
                reserve_b,
                reserve_a,
                pair_info.asset_infos[1].clone(),
                pair_info.asset_infos[0].clone(),
            )
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

    let commission_amount = gross_output
        .checked_mul(Uint128::new(fee_config.fee_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    let return_amount = gross_output.checked_sub(commission_amount)?;

    let ideal_output = input_amount
        .checked_mul(output_reserve)?
        .checked_div(input_reserve)?;
    let spread_amount = if ideal_output > gross_output {
        ideal_output.checked_sub(gross_output)?
    } else {
        Uint128::zero()
    };

    assert_max_spread(
        belief_price,
        max_spread,
        input_amount,
        return_amount,
        spread_amount,
        commission_amount,
    )?;

    let (new_reserve_a, new_reserve_b) = if offer_token_addr == token_a_addr {
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

    let receiver = match to {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => sender.clone(),
    };

    let ask_token_addr = token_addr(&ask_asset_info);

    let mut messages: Vec<CosmosMsg> = vec![];

    if !commission_amount.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ask_token_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.treasury.to_string(),
                amount: commission_amount,
            })?,
            funds: vec![],
        }));
    }

    if !return_amount.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ask_token_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: receiver.to_string(),
                amount: return_amount,
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
                offer_asset: Asset {
                    info: offer_asset_info.clone(),
                    amount: input_amount,
                },
                return_asset: Asset {
                    info: ask_asset_info.clone(),
                    amount: return_amount,
                },
                commission_amount,
                spread_amount,
            })?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "swap")
        .add_attribute("sender", sender)
        .add_attribute("receiver", receiver)
        .add_attribute("offer_asset", offer_asset_info.to_string())
        .add_attribute("ask_asset", ask_asset_info.to_string())
        .add_attribute("offer_amount", input_amount)
        .add_attribute("return_amount", return_amount)
        .add_attribute("spread_amount", spread_amount)
        .add_attribute("commission_amount", commission_amount))
}

fn execute_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;

    let (amount_a, amount_b) = match_asset_amounts(&pair_info.asset_infos, &assets)?;

    if amount_a.is_zero() || amount_b.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let total_supply = TOTAL_LP_SUPPLY.load(deps.storage)?;

    oracle_update(
        deps.storage,
        env.block.time.seconds(),
        reserve_a,
        reserve_b,
    )?;

    let is_first_deposit = reserve_a.is_zero() && reserve_b.is_zero();

    let lp_tokens_total = if is_first_deposit {
        isqrt(amount_a.checked_mul(amount_b)?)
    } else {
        let lp_a = amount_a
            .checked_mul(total_supply)?
            .checked_div(reserve_a)?;
        let lp_b = amount_b
            .checked_mul(total_supply)?
            .checked_div(reserve_b)?;
        std::cmp::min(lp_a, lp_b)
    };

    if lp_tokens_total.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    let (lp_to_user, lp_to_burn) = if is_first_deposit {
        let min_liq = Uint128::new(MINIMUM_LIQUIDITY);
        if lp_tokens_total <= min_liq {
            return Err(ContractError::InsufficientLiquidity {});
        }
        (lp_tokens_total.checked_sub(min_liq)?, min_liq)
    } else {
        (lp_tokens_total, Uint128::zero())
    };

    if let Some(tolerance) = slippage_tolerance {
        if !is_first_deposit {
            let expected_lp_a = amount_a
                .checked_mul(total_supply)?
                .checked_div(reserve_a)?;
            let expected_lp_b = amount_b
                .checked_mul(total_supply)?
                .checked_div(reserve_b)?;
            let expected_lp = std::cmp::max(expected_lp_a, expected_lp_b);

            if expected_lp > Uint128::zero() {
                let min_lp = expected_lp * (Decimal::one() - tolerance);
                if lp_to_user < min_lp {
                    return Err(ContractError::SlippageExceeded {
                        min_lp: min_lp.to_string(),
                        actual_lp: lp_to_user.to_string(),
                    });
                }
            }
        }
    }

    let new_reserve_a = reserve_a.checked_add(amount_a)?;
    let new_reserve_b = reserve_b.checked_add(amount_b)?;
    RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

    let new_total_supply = total_supply.checked_add(lp_tokens_total)?;
    TOTAL_LP_SUPPLY.save(deps.storage, &new_total_supply)?;

    let token_a_addr = token_addr(&pair_info.asset_infos[0]);
    let token_b_addr = token_addr(&pair_info.asset_infos[1]);

    let lp_receiver = match receiver {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => info.sender.clone(),
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_a_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: amount_a,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_b_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: amount_b,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: lp_receiver.to_string(),
            amount: lp_to_user,
        })?,
        funds: vec![],
    }));

    if !lp_to_burn.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.lp_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: env.contract.address.to_string(),
                amount: lp_to_burn,
            })?,
            funds: vec![],
        }));
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.lp_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: lp_to_burn,
            })?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "provide_liquidity")
        .add_attribute("sender", info.sender)
        .add_attribute("receiver", lp_receiver)
        .add_attribute("assets", format!("{}, {}", assets[0], assets[1]))
        .add_attribute("share", lp_to_user)
        .add_attribute("minimum_liquidity_burned", lp_to_burn))
}

fn execute_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    lp_amount: Uint128,
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

    // Record observation BEFORE reserves change.
    oracle_update(
        deps.storage,
        env.block.time.seconds(),
        reserve_a,
        reserve_b,
    )?;

    if total_supply.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    let amount_a = lp_amount
        .checked_mul(reserve_a)?
        .checked_div(total_supply)?;
    let amount_b = lp_amount
        .checked_mul(reserve_b)?
        .checked_div(total_supply)?;

    let new_reserve_a = reserve_a.checked_sub(amount_a)?;
    let new_reserve_b = reserve_b.checked_sub(amount_b)?;
    RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

    let new_total_supply = total_supply.checked_sub(lp_amount)?;
    TOTAL_LP_SUPPLY.save(deps.storage, &new_total_supply)?;

    let token_a_addr = token_addr(&pair_info.asset_infos[0]);
    let token_b_addr = token_addr(&pair_info.asset_infos[1]);

    let refund_asset_a = Asset {
        info: pair_info.asset_infos[0].clone(),
        amount: amount_a,
    };
    let refund_asset_b = Asset {
        info: pair_info.asset_infos[1].clone(),
        amount: amount_b,
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair_info.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount: lp_amount })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_a_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender.to_string(),
            amount: amount_a,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_b_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender.to_string(),
            amount: amount_b,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw_liquidity")
        .add_attribute("sender", sender)
        .add_attribute("withdrawn_share", lp_amount)
        .add_attribute(
            "refund_assets",
            format!("{}, {}", refund_asset_a, refund_asset_b),
        ))
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

fn execute_increase_observation_cardinality(
    deps: DepsMut,
    new_cardinality: u16,
) -> Result<Response, ContractError> {
    if new_cardinality > MAX_OBSERVATION_CARDINALITY {
        return Err(ContractError::Oracle {
            reason: format!(
                "cardinality exceeds maximum ({})",
                MAX_OBSERVATION_CARDINALITY
            ),
        });
    }

    let mut state = ORACLE_STATE.load(deps.storage)?;
    if new_cardinality <= state.cardinality {
        return Err(ContractError::Oracle {
            reason: "new cardinality must be greater than current".into(),
        });
    }

    state.cardinality = new_cardinality;
    ORACLE_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "increase_observation_cardinality")
        .add_attribute("new_cardinality", new_cardinality.to_string()))
}

fn execute_set_paused(
    deps: DepsMut,
    info: MessageInfo,
    paused: bool,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    PAUSED.save(deps.storage, &paused)?;

    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string()))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Pair {} => to_json_binary(&query_pair(deps, &env)?),
        QueryMsg::Pool {} => to_json_binary(&query_pool(deps)?),
        QueryMsg::Simulation { offer_asset } => {
            to_json_binary(&query_simulation(deps, offer_asset)?)
        }
        QueryMsg::ReverseSimulation { ask_asset } => {
            to_json_binary(&query_reverse_simulation(deps, ask_asset)?)
        }
        QueryMsg::GetFeeConfig {} => to_json_binary(&query_fee_config(deps)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::Observe { seconds_ago } => to_json_binary(
            &query_observe(deps, &env, seconds_ago)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?,
        ),
        QueryMsg::OracleInfo {} => to_json_binary(
            &query_oracle_info(deps)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?,
        ),
    }
}

fn query_pair(deps: Deps, env: &Env) -> StdResult<dex_common::types::PairInfo> {
    let state = PAIR_INFO.load(deps.storage)?;
    Ok(dex_common::types::PairInfo {
        asset_infos: state.asset_infos,
        contract_addr: env.contract.address.clone(),
        liquidity_token: state.lp_token,
    })
}

fn query_pool(deps: Deps) -> StdResult<PoolResponse> {
    let state = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let total_share = TOTAL_LP_SUPPLY.load(deps.storage)?;

    Ok(PoolResponse {
        assets: [
            Asset {
                info: state.asset_infos[0].clone(),
                amount: reserve_a,
            },
            Asset {
                info: state.asset_infos[1].clone(),
                amount: reserve_b,
            },
        ],
        total_share,
    })
}

fn query_simulation(deps: Deps, offer_asset: Asset) -> StdResult<SimulationResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let (input_reserve, output_reserve) =
        if offer_asset.info.equal(&pair_info.asset_infos[0]) {
            (reserve_a, reserve_b)
        } else if offer_asset.info.equal(&pair_info.asset_infos[1]) {
            (reserve_b, reserve_a)
        } else {
            return Err(cosmwasm_std::StdError::generic_err(
                "Invalid offer asset: does not match pair assets",
            ));
        };

    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Ok(SimulationResponse {
            return_amount: Uint128::zero(),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        });
    }

    let offer_amount = offer_asset.amount;
    let k = input_reserve.checked_mul(output_reserve)?;
    let new_input_reserve = input_reserve.checked_add(offer_amount)?;
    let new_output_reserve = k.checked_div(new_input_reserve)?;
    let gross_output = output_reserve.checked_sub(new_output_reserve)?;

    let commission_amount = gross_output
        .checked_mul(Uint128::new(fee_config.fee_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    let return_amount = gross_output.checked_sub(commission_amount)?;

    let ideal_output = offer_amount
        .checked_mul(output_reserve)?
        .checked_div(input_reserve)?;
    let spread_amount = if ideal_output > gross_output {
        ideal_output.checked_sub(gross_output)?
    } else {
        Uint128::zero()
    };

    Ok(SimulationResponse {
        return_amount,
        spread_amount,
        commission_amount,
    })
}

fn query_reverse_simulation(
    deps: Deps,
    ask_asset: Asset,
) -> StdResult<ReverseSimulationResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let (input_reserve, output_reserve) =
        if ask_asset.info.equal(&pair_info.asset_infos[1]) {
            (reserve_a, reserve_b)
        } else if ask_asset.info.equal(&pair_info.asset_infos[0]) {
            (reserve_b, reserve_a)
        } else {
            return Err(cosmwasm_std::StdError::generic_err(
                "Invalid ask asset: does not match pair assets",
            ));
        };

    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Ok(ReverseSimulationResponse {
            offer_amount: Uint128::zero(),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        });
    }

    let ask_amount = ask_asset.amount;

    if fee_config.fee_bps >= 10000 {
        return Err(cosmwasm_std::StdError::generic_err(
            "Cannot reverse-simulate with 100% fee",
        ));
    }

    let fee_denom = 10000u128 - fee_config.fee_bps as u128;
    let gross_needed = ask_amount
        .checked_mul(Uint128::new(10000))?
        .checked_div(Uint128::new(fee_denom))?
        .checked_add(Uint128::one())?;
    let commission_amount = gross_needed.checked_sub(ask_amount)?;

    if gross_needed >= output_reserve {
        return Err(cosmwasm_std::StdError::generic_err(
            "Insufficient liquidity for reverse simulation",
        ));
    }

    let denom = output_reserve.checked_sub(gross_needed)?;
    let offer_amount = input_reserve
        .checked_mul(gross_needed)?
        .checked_div(denom)?
        .checked_add(Uint128::one())?;

    let ideal_output = offer_amount
        .checked_mul(output_reserve)?
        .checked_div(input_reserve)?;
    let spread_amount = if ideal_output > gross_needed {
        ideal_output.checked_sub(gross_needed)?
    } else {
        Uint128::zero()
    };

    Ok(ReverseSimulationResponse {
        offer_amount,
        spread_amount,
        commission_amount,
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

fn query_observe(
    deps: Deps,
    env: &Env,
    seconds_ago: Vec<u32>,
) -> Result<ObserveResponse, ContractError> {
    let state = ORACLE_STATE.load(deps.storage)?;

    if state.cardinality_initialized == 0 {
        return Err(ContractError::Oracle {
            reason: "no observations recorded yet".into(),
        });
    }

    let latest = OBSERVATIONS.load(deps.storage, state.index)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let block_time = env.block.time.seconds();

    let mut tick_cumulatives = Vec::with_capacity(seconds_ago.len());
    for &sa in &seconds_ago {
        let tc = oracle_observe_single(
            deps.storage,
            block_time,
            sa,
            &state,
            &latest,
            reserve_a,
            reserve_b,
        )?;
        tick_cumulatives.push(tc);
    }

    Ok(ObserveResponse { tick_cumulatives })
}

fn query_oracle_info(deps: Deps) -> Result<OracleInfoResponse, ContractError> {
    let state = ORACLE_STATE.load(deps.storage)?;

    if state.cardinality_initialized == 0 {
        return Ok(OracleInfoResponse {
            observation_cardinality: state.cardinality,
            observation_index: state.index,
            oldest_observation_timestamp: 0,
            newest_observation_timestamp: 0,
        });
    }

    let latest = OBSERVATIONS.load(deps.storage, state.index)?;
    let oldest_idx = if state.cardinality_initialized < state.cardinality {
        0u16
    } else {
        (state.index + 1) % state.cardinality
    };
    let oldest = OBSERVATIONS.load(deps.storage, oldest_idx)?;

    Ok(OracleInfoResponse {
        observation_cardinality: state.cardinality,
        observation_index: state.index,
        oldest_observation_timestamp: oldest.timestamp,
        newest_observation_timestamp: latest.timestamp,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn match_asset_amounts(
    pair_asset_infos: &[AssetInfo; 2],
    provided: &[Asset; 2],
) -> Result<(Uint128, Uint128), ContractError> {
    if provided[0].info.equal(&pair_asset_infos[0])
        && provided[1].info.equal(&pair_asset_infos[1])
    {
        Ok((provided[0].amount, provided[1].amount))
    } else if provided[0].info.equal(&pair_asset_infos[1])
        && provided[1].info.equal(&pair_asset_infos[0])
    {
        Ok((provided[1].amount, provided[0].amount))
    } else {
        Err(ContractError::AssetMismatch {})
    }
}
