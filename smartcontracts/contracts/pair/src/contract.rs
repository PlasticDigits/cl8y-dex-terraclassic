use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, Event, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, MinterResponse};

use crate::error::ContractError;
use crate::msg::{
    Cw20HookMsg, ExecuteMsg, ExpiredLimitRefundResponse, FeeConfigResponse, HooksResponse,
    InstantiateMsg, LimitOrderResponse, ObserveResponse, OracleInfoResponse, PoolResponse,
    QueryMsg, ReverseSimulationResponse, SimulationResponse,
};
use crate::orderbook;
use crate::state::{
    OracleState, PairInfoState, DISCOUNT_REGISTRY, EXPIRED_LIMIT_CLAIMS, FEE_CONFIG, HOOKS,
    OBSERVATIONS, ORACLE_STATE, ORDER_NEXT_ID, PAIR_INFO, PAUSED, PENDING_ESCROW_TOKEN0,
    PENDING_ESCROW_TOKEN1, RESERVES, TOTAL_LP_SUPPLY,
};
use dex_common::fee_discount;
use dex_common::hook::{HookCallMsg, HookExecuteMsg};
use dex_common::oracle::{
    price_times_dt, Observation, DEFAULT_OBSERVATION_CARDINALITY, MAX_OBSERVATION_CARDINALITY,
};
use dex_common::pair::{
    HybridReverseSimulationResponse, HybridSimulationResponse, HybridSwapParams, LimitOrderSide,
    LP_TOKEN_DECIMALS, MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP,
};
use dex_common::types::{Asset, AssetInfo, FeeConfig};

const CONTRACT_NAME: &str = "cl8y-dex-pair";
const CONTRACT_VERSION: &str = "1.5.0";
const INSTANTIATE_LP_TOKEN_REPLY_ID: u64 = 1;
/// First 1000 LP tokens are permanently burned on the initial deposit
/// to prevent share-inflation griefing attacks where an attacker donates
/// tokens to make subsequent depositors receive 0 LP shares.
const MINIMUM_LIQUIDITY: u128 = 1_000;

/// Fee discount for a wallet placing a limit order (`trader` == `sender` == order owner).
fn effective_fee_bps_with_discount_msgs(
    deps: Deps,
    fee_bps: u16,
    discount_registry: Option<Addr>,
    owner: Addr,
) -> Result<(u16, Vec<CosmosMsg>), ContractError> {
    let trader = owner.to_string();
    let sender = trader.clone();
    let mut deregister_msgs: Vec<CosmosMsg> = vec![];
    let effective_fee_bps = match discount_registry {
        Some(ref registry) => {
            let discount_result: StdResult<fee_discount::DiscountResponse> =
                deps.querier.query_wasm_smart(
                    registry.to_string(),
                    &fee_discount::QueryMsg::GetDiscount {
                        trader: trader.clone(),
                        sender,
                    },
                );
            match discount_result {
                Ok(discount) => {
                    if discount.needs_deregister {
                        deregister_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: registry.to_string(),
                            msg: to_json_binary(&fee_discount::ExecuteMsg::DeregisterWallet {
                                wallet: trader,
                                epoch: discount.registration_epoch,
                            })?,
                            funds: vec![],
                        }));
                    }
                    let discounted = (fee_bps as u32)
                        * (10000u32.saturating_sub(discount.discount_bps as u32))
                        / 10000u32;
                    discounted as u16
                }
                Err(_) => fee_bps,
            }
        }
        None => fee_bps,
    };
    Ok((effective_fee_bps, deregister_msgs))
}

/// Integer square root via Newton's method. Returns floor(√n).
///
/// Used to compute initial LP token supply as `sqrt(amount_a * amount_b)`,
/// following the Uniswap V2 approach. The caller validates correctness
/// with two-sided bounds: `result² ≤ n < (result+1)²`.
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

/// Ceiling division: ceil(a / b). Guarantees result * b >= a, so the pool
/// never loses value from integer rounding during swaps.
fn ceil_div(numerator: Uint128, denominator: Uint128) -> Uint128 {
    let d = numerator / denominator;
    if d * denominator < numerator {
        d + Uint128::one()
    } else {
        d
    }
}

/// Extract the CW20 contract address from an `AssetInfo`. Panics on
/// native tokens — the caller must validate inputs before reaching here.
fn token_addr(info: &AssetInfo) -> &str {
    match info {
        AssetInfo::Token { contract_addr } => contract_addr.as_str(),
        AssetInfo::NativeToken { .. } => unreachable!("native tokens not supported"),
    }
}

/// First LP mint uses `Uint128 amount_a × amount_b`. CW20 decimals above
/// [`MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`] are rejected (#124).
fn assert_pair_assets_decimals_allow_initial_mint(
    deps: Deps,
    asset_infos: &[AssetInfo; 2],
) -> Result<(), ContractError> {
    let max = MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP;
    let info0: cw20::TokenInfoResponse = deps.querier.query_wasm_smart(
        token_addr(&asset_infos[0]).to_string(),
        &Cw20QueryMsg::TokenInfo {},
    )?;
    let info1: cw20::TokenInfoResponse = deps.querier.query_wasm_smart(
        token_addr(&asset_infos[1]).to_string(),
        &Cw20QueryMsg::TokenInfo {},
    )?;
    if info0.decimals > max || info1.decimals > max {
        return Err(ContractError::PairAssetDecimalsTooHigh {
            decimals0: info0.decimals,
            decimals1: info1.decimals,
            max,
        });
    }
    Ok(())
}

/// Revert if the current block time exceeds the user-supplied deadline.
/// Protects against transaction delays that could result in execution at
/// stale prices (e.g., pending mempool transactions or delayed blocks).
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

/// Gate for pause-protected operations. When paused, all swaps, liquidity
/// provision, and withdrawals are blocked. Only the factory (via governance)
/// can pause/unpause.
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

    let (last_ts, last_cum_a, last_cum_b) = match last_obs {
        Some(obs) => (
            obs.timestamp,
            obs.price_a_cumulative,
            obs.price_b_cumulative,
        ),
        None => {
            // First observation: seed with current timestamp and zero cumulatives.
            // No dt to accumulate — just record the starting point.
            OBSERVATIONS.save(
                storage,
                state.index,
                &Observation {
                    timestamp: block_time,
                    price_a_cumulative: Uint128::zero(),
                    price_b_cumulative: Uint128::zero(),
                },
            )?;
            state.cardinality_initialized = 1;
            ORACLE_STATE.save(storage, &state)?;
            return Ok(());
        }
    };

    if block_time <= last_ts {
        return Ok(());
    }

    let dt = block_time - last_ts;
    let price_a = Decimal::from_ratio(reserve_b, reserve_a);
    let price_b = Decimal::from_ratio(reserve_a, reserve_b);

    let delta_a = price_times_dt(price_a, dt).map_err(|e| ContractError::Oracle {
        reason: e.to_string(),
    })?;
    let delta_b = price_times_dt(price_b, dt).map_err(|e| ContractError::Oracle {
        reason: e.to_string(),
    })?;

    let new_cum_a = last_cum_a
        .checked_add(delta_a)
        .map_err(|e| ContractError::Oracle {
            reason: format!("price_a overflow: {}", e),
        })?;
    let new_cum_b = last_cum_b
        .checked_add(delta_b)
        .map_err(|e| ContractError::Oracle {
            reason: format!("price_b overflow: {}", e),
        })?;

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
            price_a_cumulative: new_cum_a,
            price_b_cumulative: new_cum_b,
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
/// target timestamp, then linearly interpolate the cumulative prices.
fn oracle_observe_single(
    storage: &dyn cosmwasm_std::Storage,
    block_time: u64,
    seconds_ago: u32,
    state: &OracleState,
    latest_obs: &Observation,
    reserve_a: Uint128,
    reserve_b: Uint128,
) -> Result<(Uint128, Uint128), ContractError> {
    let target = block_time - seconds_ago as u64;

    if seconds_ago == 0 || target >= latest_obs.timestamp {
        if target == latest_obs.timestamp {
            return Ok((latest_obs.price_a_cumulative, latest_obs.price_b_cumulative));
        }
        if reserve_a.is_zero() || reserve_b.is_zero() {
            return Ok((latest_obs.price_a_cumulative, latest_obs.price_b_cumulative));
        }
        let dt = target - latest_obs.timestamp;
        let price_a = Decimal::from_ratio(reserve_b, reserve_a);
        let price_b = Decimal::from_ratio(reserve_a, reserve_b);
        let delta_a = price_times_dt(price_a, dt).map_err(|e| ContractError::Oracle {
            reason: e.to_string(),
        })?;
        let delta_b = price_times_dt(price_b, dt).map_err(|e| ContractError::Oracle {
            reason: e.to_string(),
        })?;
        let cum_a = latest_obs
            .price_a_cumulative
            .checked_add(delta_a)
            .map_err(|e| ContractError::Oracle {
                reason: e.to_string(),
            })?;
        let cum_b = latest_obs
            .price_b_cumulative
            .checked_add(delta_b)
            .map_err(|e| ContractError::Oracle {
                reason: e.to_string(),
            })?;
        return Ok((cum_a, cum_b));
    }

    let n = state.cardinality_initialized;
    if n < 2 {
        return Err(ContractError::Oracle {
            reason: "not enough observations for the requested window".into(),
        });
    }

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

    let mut lo: u16 = 0;
    let mut hi: u16 = n - 1;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
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
        return Ok((before.price_a_cumulative, before.price_b_cumulative));
    }

    let after_idx = (oldest_idx + lo + 1) % state.cardinality;
    let after = if (lo + 1) < n {
        OBSERVATIONS.load(storage, after_idx)?
    } else {
        latest_obs.clone()
    };

    let time_span = after.timestamp - before.timestamp;
    let dt = target - before.timestamp;

    let diff_a = after.price_a_cumulative - before.price_a_cumulative;
    let diff_b = after.price_b_cumulative - before.price_b_cumulative;

    let interp_a = before.price_a_cumulative + diff_a.multiply_ratio(dt as u128, time_span as u128);
    let interp_b = before.price_b_cumulative + diff_b.multiply_ratio(dt as u128, time_span as u128);

    Ok((interp_a, interp_b))
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
        ai.assert_is_token().map_err(ContractError::Std)?;
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
    DISCOUNT_REGISTRY.save(deps.storage, &None)?;
    ORDER_NEXT_ID.save(deps.storage, &1u64)?;
    PENDING_ESCROW_TOKEN0.save(deps.storage, &Uint128::zero())?;
    PENDING_ESCROW_TOKEN1.save(deps.storage, &Uint128::zero())?;

    ORACLE_STATE.save(
        deps.storage,
        &OracleState {
            cardinality: DEFAULT_OBSERVATION_CARDINALITY,
            index: 0,
            cardinality_initialized: 0,
        },
    )?;

    let (lp_name, lp_symbol, lp_label) = match msg.token_symbols {
        Some([ref a, ref b]) => {
            let short_a: String = a.chars().take(4).collect();
            let short_b: String = b.chars().take(4).collect();
            (
                format!("{}-{} CL8YDEX LP", a, b),
                format!("{}-{}-LP", short_a, short_b),
                format!("{}-{} cl8ydex lp", a, b),
            )
        }
        None => (
            "CL8Y DEX LP Token".to_string(),
            "CL8Y-LP".to_string(),
            "CL8Y DEX LP Token".to_string(),
        ),
    };

    let instantiate_lp_msg = cw20_mintable::msg::InstantiateMsg {
        name: lp_name,
        symbol: lp_symbol,
        decimals: LP_TOKEN_DECIMALS,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: env.contract.address.to_string(),
            cap: None,
        }),
        marketing: None,
    };

    let sub_msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: Some(env.contract.address.to_string()),
            code_id: msg.lp_token_code_id,
            msg: to_json_binary(&instantiate_lp_msg)?,
            funds: vec![],
            label: lp_label,
        },
        INSTANTIATE_LP_TOKEN_REPLY_ID,
    );

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "instantiate")
        .add_attribute(
            "pair",
            format!("{}-{}", msg.asset_infos[0], msg.asset_infos[1]),
        ))
}

pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_LP_TOKEN_REPLY_ID => {
            let res = cw_utils::parse_reply_instantiate_data(msg).map_err(|e| {
                ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string()))
            })?;
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
            offer_asset
                .info
                .assert_is_token()
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
        ExecuteMsg::SetDiscountRegistry { registry } => {
            execute_set_discount_registry(deps, info, registry)
        }
        ExecuteMsg::SetPaused { paused } => execute_set_paused(deps, info, paused),
        ExecuteMsg::Sweep { token, recipient } => execute_sweep(deps, env, info, token, recipient),
        ExecuteMsg::SetLpAdmin { admin } => execute_set_lp_admin(deps, info, admin),
        ExecuteMsg::CancelLimitOrder { order_id } => {
            assert_not_paused(deps.storage)?;
            execute_cancel_limit_order(deps, env, info, order_id)
        }
        ExecuteMsg::ClaimExpiredLimitOrder { order_id } => {
            execute_claim_expired_limit_order(deps, env, info, order_id)
        }
        ExecuteMsg::UpdateLimitOrderPrice {
            order_id,
            price,
            hint_after_order_id,
            max_adjust_steps,
        } => {
            assert_not_paused(deps.storage)?;
            execute_update_limit_order_price(
                deps,
                env,
                info,
                order_id,
                price,
                hint_after_order_id,
                max_adjust_steps,
            )
        }
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
            trader,
            hybrid,
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
                trader,
                hybrid,
            )
        }
        Cw20HookMsg::PlaceLimitOrder {
            side,
            price,
            hint_after_order_id,
            max_adjust_steps,
            expires_at,
        } => execute_place_limit_order(
            deps,
            env,
            info,
            token_sender,
            cw20_msg.amount,
            side,
            price,
            hint_after_order_id,
            max_adjust_steps,
            expires_at,
        ),
        Cw20HookMsg::WithdrawLiquidity { min_assets } => {
            execute_withdraw_liquidity(deps, env, info, token_sender, cw20_msg.amount, min_assets)
        }
    }
}

/// Difference between the spot-linear estimate `floor(offer × out / in)` and the
/// constant-product `gross_output`. Integer rounding on the two paths can make
/// this exceed `gross_output` at extreme reserve ratios even though the swap is
/// economically fine; [`assert_max_spread`] caps it when comparing to `max_spread`.
fn spot_linear_spread_over_gross(
    pool_input: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    gross_output: Uint128,
) -> StdResult<Uint128> {
    if input_reserve.is_zero() {
        return Err(StdError::generic_err(
            "spot_linear_spread_over_gross: input_reserve is zero",
        ));
    }
    let ideal_output = pool_input
        .checked_mul(output_reserve)
        .map_err(StdError::from)?
        .checked_div(input_reserve)
        .map_err(StdError::from)?;
    if ideal_output > gross_output {
        ideal_output
            .checked_sub(gross_output)
            .map_err(StdError::from)
    } else {
        Ok(Uint128::zero())
    }
}

/// Validate that the swap's effective spread does not exceed the user's
/// tolerance. When `belief_price` is provided, spread is computed against
/// the expected return at that price; otherwise it is computed from the
/// constant-product spread relative to the gross output. Defaults to 1%
/// if `max_spread` is not specified.
fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_amount: Uint128,
    return_amount: Uint128,
    spread_amount: Uint128,
    commission_amount: Uint128,
    book_return_net: Uint128,
) -> Result<(), ContractError> {
    let default_spread = Decimal::percent(1);
    let max_allowed = max_spread.unwrap_or(default_spread);

    if let Some(bp) = belief_price {
        let expected_return = offer_amount * (Decimal::one() / bp);
        let actual_return = book_return_net
            .checked_add(return_amount)?
            .checked_add(commission_amount)?;
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
        let pool_gross = return_amount.checked_add(commission_amount)?;
        let total_gross_out = pool_gross.checked_add(book_return_net)?;
        let spread_cmp = spread_amount.min(pool_gross);
        if total_gross_out > Uint128::zero()
            && Decimal::from_ratio(spread_cmp, total_gross_out) > max_allowed
        {
            return Err(ContractError::MaxSpreadAssertion {
                max: max_allowed.to_string(),
                actual: Decimal::from_ratio(spread_cmp, total_gross_out).to_string(),
            });
        }
    }

    Ok(())
}

/// Execute a constant-product swap.
///
/// 1. Record a TWAP observation **before** reserves change (manipulation-resistant).
/// 2. Compute output via `new_output = ceil_div(k, new_input)` — pool-favorable rounding.
/// 3. Validate the k-invariant: `new_k >= k` with bounded rounding slack.
/// 4. Look up trader fee discount from the registry (if configured).
/// 5. Deduct commission (fee), send it to the treasury.
/// 6. Assert spread/slippage against user tolerance.
/// 7. Transfer the net return to the receiver.
/// 8. Fire post-swap hooks (burn, tax, etc.).
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
    trader: Option<String>,
    hybrid: Option<HybridSwapParams>,
) -> Result<Response, ContractError> {
    if input_amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let (pool_leg, book_leg, max_makers, book_hint) = match &hybrid {
        None => (input_amount, Uint128::zero(), 0u32, None),
        Some(h) => {
            if h.pool_input.checked_add(h.book_input)? != input_amount {
                return Err(ContractError::HybridSplitMismatch {});
            }
            if h.book_input > Uint128::zero() && h.max_maker_fills == 0 {
                return Err(ContractError::InvalidHybridParams {
                    reason: "max_maker_fills must be > 0 when book_input > 0".into(),
                });
            }
            (
                h.pool_input,
                h.book_input,
                h.max_maker_fills,
                h.book_start_hint,
            )
        }
    };

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    oracle_update(deps.storage, env.block.time.seconds(), reserve_a, reserve_b)?;

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

    let receiver = match to {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => sender.clone(),
    };

    let trader_addr = trader.unwrap_or_else(|| sender.to_string());
    let discount_registry = DISCOUNT_REGISTRY.load(deps.storage)?;
    let mut deregister_msgs: Vec<CosmosMsg> = vec![];

    let effective_fee_bps = match discount_registry {
        Some(ref registry) => {
            let discount_result: StdResult<fee_discount::DiscountResponse> =
                deps.querier.query_wasm_smart(
                    registry.to_string(),
                    &fee_discount::QueryMsg::GetDiscount {
                        trader: trader_addr.clone(),
                        sender: sender.to_string(),
                    },
                );
            match discount_result {
                Ok(discount) => {
                    if discount.needs_deregister {
                        deregister_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: registry.to_string(),
                            msg: to_json_binary(&fee_discount::ExecuteMsg::DeregisterWallet {
                                wallet: trader_addr.clone(),
                                epoch: discount.registration_epoch,
                            })?,
                            funds: vec![],
                        }));
                    }
                    let discounted = (fee_config.fee_bps as u32)
                        * (10000u32.saturating_sub(discount.discount_bps as u32))
                        / 10000u32;
                    discounted as u16
                }
                Err(_) => fee_config.fee_bps,
            }
        }
        None => fee_config.fee_bps,
    };

    let ask_token_addr = token_addr(&ask_asset_info);

    let mut book_messages: Vec<CosmosMsg> = vec![];
    let mut book_fill_events: Vec<Event> = vec![];
    let mut book_return_net = Uint128::zero();
    let mut offer_consumed_by_book = Uint128::zero();

    if book_leg > Uint128::zero() {
        if offer_token_addr == token_a_addr {
            let (t1_out, t0_used, _mk, msgs, fill_events) = orderbook::match_bids(
                deps.storage,
                env.block.time.seconds(),
                book_leg,
                max_makers,
                book_hint,
                env.contract.address.as_str(),
                token_a_addr,
                token_b_addr,
                &receiver,
                &fee_config.treasury,
                effective_fee_bps,
            )?;
            book_messages = msgs;
            book_fill_events = fill_events;
            book_return_net = t1_out;
            offer_consumed_by_book = t0_used;
        } else {
            let (t0_out, t1_used, _mk, msgs, fill_events) = orderbook::match_asks(
                deps.storage,
                env.block.time.seconds(),
                book_leg,
                max_makers,
                book_hint,
                env.contract.address.as_str(),
                token_a_addr,
                token_b_addr,
                &receiver,
                &fee_config.treasury,
                effective_fee_bps,
            )?;
            book_messages = msgs;
            book_fill_events = fill_events;
            book_return_net = t0_out;
            offer_consumed_by_book = t1_used;
        }
    }

    let pool_input_amount = pool_leg.checked_add(book_leg.checked_sub(offer_consumed_by_book)?)?;

    let mut return_amount = Uint128::zero();
    let mut commission_amount = Uint128::zero();
    let mut spread_amount = Uint128::zero();
    let mut pool_messages: Vec<CosmosMsg> = vec![];

    if pool_input_amount > Uint128::zero() {
        if input_reserve.is_zero() || output_reserve.is_zero() {
            return Err(ContractError::InsufficientLiquidity {});
        }

        let k = input_reserve.checked_mul(output_reserve)?;
        let new_input_reserve = input_reserve.checked_add(pool_input_amount)?;
        let new_output_reserve = ceil_div(k, new_input_reserve);
        let gross_output = output_reserve.checked_sub(new_output_reserve)?;

        let new_k = new_input_reserve.checked_mul(new_output_reserve)?;
        if new_k < k {
            return Err(ContractError::InvariantViolation {
                reason: format!("k decreased: {} -> {}", k, new_k),
            });
        }
        if new_k - k >= new_input_reserve {
            return Err(ContractError::InvariantViolation {
                reason: format!(
                    "k increase exceeds rounding bound: delta={}, bound={}",
                    new_k - k,
                    new_input_reserve
                ),
            });
        }

        let fee_numerator = gross_output.checked_mul(Uint128::new(effective_fee_bps as u128))?;
        commission_amount = fee_numerator.checked_div(Uint128::new(10000))?;
        return_amount = gross_output.checked_sub(commission_amount)?;

        let commission_remainder =
            fee_numerator - commission_amount.checked_mul(Uint128::new(10000))?;
        if commission_remainder >= Uint128::new(10000) {
            return Err(ContractError::InvariantViolation {
                reason: format!(
                    "commission rounding exceeds 1 token: remainder={}",
                    commission_remainder
                ),
            });
        }

        spread_amount = spot_linear_spread_over_gross(
            pool_input_amount,
            input_reserve,
            output_reserve,
            gross_output,
        )
        .map_err(ContractError::Std)?;

        let (new_reserve_a, new_reserve_b) = if offer_token_addr == token_a_addr {
            (
                reserve_a.checked_add(pool_input_amount)?,
                reserve_b.checked_sub(gross_output)?,
            )
        } else {
            (
                reserve_a.checked_sub(gross_output)?,
                reserve_b.checked_add(pool_input_amount)?,
            )
        };
        RESERVES.save(deps.storage, &(new_reserve_a, new_reserve_b))?;

        if !commission_amount.is_zero() {
            pool_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ask_token_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: fee_config.treasury.to_string(),
                    amount: commission_amount,
                })?,
                funds: vec![],
            }));
        }

        if !return_amount.is_zero() {
            pool_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ask_token_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: receiver.to_string(),
                    amount: return_amount,
                })?,
                funds: vec![],
            }));
        }
    }

    let total_return = book_return_net.checked_add(return_amount)?;

    assert_max_spread(
        belief_price,
        max_spread,
        input_amount,
        return_amount,
        spread_amount,
        commission_amount,
        book_return_net,
    )?;

    let hooks = HOOKS.load(deps.storage)?;
    let mut hook_messages: Vec<CosmosMsg> = vec![];
    for hook in hooks {
        hook_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.to_string(),
            msg: to_json_binary(&HookCallMsg::Hook(HookExecuteMsg::AfterSwap {
                pair: env.contract.address.clone(),
                sender: sender.clone(),
                offer_asset: Asset {
                    info: offer_asset_info.clone(),
                    amount: input_amount,
                },
                return_asset: Asset {
                    info: ask_asset_info.clone(),
                    amount: total_return,
                },
                commission_amount,
                spread_amount,
            }))?,
            funds: vec![],
        }));
    }

    let mut resp = Response::new()
        .add_messages(book_messages)
        .add_events(book_fill_events)
        .add_messages(pool_messages)
        .add_messages(hook_messages)
        .add_messages(deregister_msgs)
        .add_attribute("action", "swap")
        .add_attribute("sender", sender)
        .add_attribute("receiver", receiver)
        .add_attribute("offer_asset", offer_asset_info.to_string())
        .add_attribute("ask_asset", ask_asset_info.to_string())
        .add_attribute("offer_amount", input_amount)
        .add_attribute("return_amount", total_return)
        .add_attribute("pool_return_amount", return_amount)
        .add_attribute("book_return_amount", book_return_net)
        .add_attribute("spread_amount", spread_amount)
        .add_attribute("commission_amount", commission_amount)
        .add_attribute("effective_fee_bps", effective_fee_bps.to_string());

    if book_leg > Uint128::zero() {
        resp = resp.add_attribute("limit_book_offer_consumed", offer_consumed_by_book);
    }

    Ok(resp)
}

#[allow(clippy::too_many_arguments)]
fn execute_place_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Addr,
    amount: Uint128,
    side: LimitOrderSide,
    price: Decimal,
    hint_after_order_id: Option<u64>,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    if price.is_zero() {
        return Err(ContractError::InvalidHybridParams {
            reason: "limit price must be positive".into(),
        });
    }
    let now = env.block.time.seconds();
    if let Some(t) = expires_at {
        if t <= now {
            return Err(ContractError::InvalidHybridParams {
                reason: "expires_at must be in the future".into(),
            });
        }
    }
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    let discount_registry = DISCOUNT_REGISTRY.load(deps.storage)?;
    let (effective_fee_bps, deregister_msgs) = effective_fee_bps_with_discount_msgs(
        deps.as_ref(),
        fee_config.fee_bps,
        discount_registry,
        owner.clone(),
    )?;

    let maker_bps = orderbook::maker_fee_bps(effective_fee_bps);
    let maker_fee = amount
        .checked_mul(Uint128::new(maker_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    if maker_fee >= amount {
        return Err(ContractError::LimitOrderMakerFeeExceedsAmount {});
    }
    let remaining_for_book = amount.checked_sub(maker_fee)?;

    let token_a = token_addr(&pair_info.asset_infos[0]);
    let token_b = token_addr(&pair_info.asset_infos[1]);
    let cw20_addr = info.sender.as_str();

    let side_label = crate::orderbook::side_str(&side);
    let owner_str = owner.to_string();

    let mut resp = Response::new();
    for m in deregister_msgs {
        resp = resp.add_message(m);
    }
    if !maker_fee.is_zero() {
        resp = resp.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.treasury.to_string(),
                amount: maker_fee,
            })?,
            funds: vec![],
        }));
    }

    let id = match side {
        LimitOrderSide::Bid => {
            if cw20_addr != token_b {
                return Err(ContractError::InvalidToken {});
            }
            orderbook::insert_bid(
                deps.storage,
                price,
                remaining_for_book,
                owner,
                hint_after_order_id,
                max_adjust_steps,
                expires_at,
            )?
        }
        LimitOrderSide::Ask => {
            if cw20_addr != token_a {
                return Err(ContractError::InvalidToken {});
            }
            orderbook::insert_ask(
                deps.storage,
                price,
                remaining_for_book,
                owner,
                hint_after_order_id,
                max_adjust_steps,
                expires_at,
            )?
        }
    };

    resp = resp
        .add_attribute("action", "place_limit_order")
        .add_attribute("limit_order_placed", id.to_string())
        .add_attribute("order_id", id.to_string())
        .add_attribute("side", side_label)
        .add_attribute("price", price.to_string())
        .add_attribute("owner", owner_str.as_str())
        .add_attribute("maker_fee_amount", maker_fee)
        .add_attribute("effective_fee_bps", effective_fee_bps.to_string());
    if let Some(t) = expires_at {
        resp = resp.add_attribute("expires_at", t.to_string());
    }
    Ok(resp)
}

fn execute_update_limit_order_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    order_id: u64,
    price: Decimal,
    hint_after_order_id: Option<u64>,
    max_adjust_steps: u32,
) -> Result<Response, ContractError> {
    if price.is_zero() {
        return Err(ContractError::InvalidHybridParams {
            reason: "limit price must be positive".into(),
        });
    }
    let o = crate::state::ORDERS.load(deps.storage, order_id)?;
    if o.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(t) = o.expires_at {
        if env.block.time.seconds() >= t {
            return Err(ContractError::InvalidHybridParams {
                reason: "order expired".into(),
            });
        }
    }
    orderbook::relink_limit_order_price(
        deps.storage,
        order_id,
        price,
        hint_after_order_id,
        max_adjust_steps,
    )?;
    Ok(Response::new()
        .add_attribute("action", "update_limit_order_price")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("price", price.to_string()))
}

fn execute_claim_expired_limit_order(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    order_id: u64,
) -> Result<Response, ContractError> {
    let row = EXPIRED_LIMIT_CLAIMS
        .may_load(deps.storage, order_id)?
        .ok_or(ContractError::NoExpiredLimitClaim { order_id })?;
    if row.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let token_a = token_addr(&pair_info.asset_infos[0]);
    let token_b = token_addr(&pair_info.asset_infos[1]);

    let refund_msg = match row.side {
        LimitOrderSide::Bid => {
            let mut esc = PENDING_ESCROW_TOKEN1
                .may_load(deps.storage)?
                .unwrap_or(Uint128::zero());
            esc =
                esc.checked_sub(row.remaining)
                    .map_err(|_| ContractError::InvariantViolation {
                        reason: "pending escrow token1 underflow on claim expired".into(),
                    })?;
            PENDING_ESCROW_TOKEN1.save(deps.storage, &esc)?;
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_b.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: row.owner.to_string(),
                    amount: row.remaining,
                })?,
                funds: vec![],
            })
        }
        LimitOrderSide::Ask => {
            let mut esc = PENDING_ESCROW_TOKEN0
                .may_load(deps.storage)?
                .unwrap_or(Uint128::zero());
            esc =
                esc.checked_sub(row.remaining)
                    .map_err(|_| ContractError::InvariantViolation {
                        reason: "pending escrow token0 underflow on claim expired".into(),
                    })?;
            PENDING_ESCROW_TOKEN0.save(deps.storage, &esc)?;
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_a.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: row.owner.to_string(),
                    amount: row.remaining,
                })?,
                funds: vec![],
            })
        }
    };

    EXPIRED_LIMIT_CLAIMS.remove(deps.storage, order_id);

    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("action", "claim_expired_limit_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("owner", row.owner.as_str()))
}

fn execute_cancel_limit_order(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    order_id: u64,
) -> Result<Response, ContractError> {
    let o = crate::state::ORDERS.load(deps.storage, order_id)?;
    if o.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let token_a = token_addr(&pair_info.asset_infos[0]);
    let token_b = token_addr(&pair_info.asset_infos[1]);
    let removed = orderbook::unlink_order(deps.storage, order_id)?;

    let refund_msg = match removed.side {
        LimitOrderSide::Bid => {
            let mut esc = PENDING_ESCROW_TOKEN1
                .may_load(deps.storage)?
                .unwrap_or(Uint128::zero());
            esc = esc.checked_sub(removed.remaining).map_err(|_| {
                ContractError::InvariantViolation {
                    reason: "pending escrow token1 underflow on cancel".into(),
                }
            })?;
            PENDING_ESCROW_TOKEN1.save(deps.storage, &esc)?;
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_b.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: removed.owner.to_string(),
                    amount: removed.remaining,
                })?,
                funds: vec![],
            })
        }
        LimitOrderSide::Ask => {
            let mut esc = PENDING_ESCROW_TOKEN0
                .may_load(deps.storage)?
                .unwrap_or(Uint128::zero());
            esc = esc.checked_sub(removed.remaining).map_err(|_| {
                ContractError::InvariantViolation {
                    reason: "pending escrow token0 underflow on cancel".into(),
                }
            })?;
            PENDING_ESCROW_TOKEN0.save(deps.storage, &esc)?;
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_a.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: removed.owner.to_string(),
                    amount: removed.remaining,
                })?,
                funds: vec![],
            })
        }
    };

    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("action", "cancel_limit_order")
        .add_attribute("limit_order_cancelled", order_id.to_string())
        .add_attribute("owner", removed.owner.as_str()))
}

/// Deposit both tokens proportionally and mint LP tokens to the provider.
///
/// **First deposit:** LP = `sqrt(amount_a × amount_b)` with `MINIMUM_LIQUIDITY`
/// permanently burned to prevent share-inflation griefing.
/// Both CW20 `decimals` must be ≤ [`MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`] (see `#124`).
///
/// **Subsequent deposits:** LP = `min(a × supply / reserve_a, b × supply / reserve_b)`.
/// The smaller ratio is used, so excess tokens beyond the current pool ratio
/// effectively donate value to existing LPs (incentivizing balanced deposits).
///
/// `slippage_tolerance` protects against pool ratio changes between the
/// user's quote and execution.
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

    oracle_update(deps.storage, env.block.time.seconds(), reserve_a, reserve_b)?;

    let is_first_deposit = reserve_a.is_zero() && reserve_b.is_zero();

    if is_first_deposit {
        assert_pair_assets_decimals_allow_initial_mint(deps.as_ref(), &pair_info.asset_infos)?;
    }

    let lp_tokens_total = if is_first_deposit {
        let product = amount_a.checked_mul(amount_b)?;
        let lp = isqrt(product);
        // Sanity: isqrt rounding — lp^2 <= product < (lp+1)^2
        if lp.checked_mul(lp)? > product {
            return Err(ContractError::InvariantViolation {
                reason: format!("isqrt too large: {}^2 > {}", lp, product),
            });
        }
        if let Ok(next_sq) = (lp + Uint128::one()).checked_mul(lp + Uint128::one()) {
            if next_sq <= product {
                return Err(ContractError::InvariantViolation {
                    reason: format!("isqrt too small: {}^2 <= {}", lp + Uint128::one(), product),
                });
            }
        }
        lp
    } else {
        let numerator_a = amount_a.checked_mul(total_supply)?;
        let lp_a = numerator_a.checked_div(reserve_a)?;
        let numerator_b = amount_b.checked_mul(total_supply)?;
        let lp_b = numerator_b.checked_div(reserve_b)?;

        // Sanity: floor-division rounding loses < 1 LP token.
        // numerator - lp * reserve must be < reserve.
        let rem_a = numerator_a - lp_a.checked_mul(reserve_a)?;
        if rem_a >= reserve_a {
            return Err(ContractError::InvariantViolation {
                reason: format!("LP-A floor rounding exceeds 1 token: rem={}", rem_a),
            });
        }
        let rem_b = numerator_b - lp_b.checked_mul(reserve_b)?;
        if rem_b >= reserve_b {
            return Err(ContractError::InvariantViolation {
                reason: format!("LP-B floor rounding exceeds 1 token: rem={}", rem_b),
            });
        }

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
            let expected_lp_a = amount_a.checked_mul(total_supply)?.checked_div(reserve_a)?;
            let expected_lp_b = amount_b.checked_mul(total_supply)?.checked_div(reserve_b)?;
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
            msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount: lp_to_burn })?,
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

/// Burn LP tokens and return underlying assets pro-rata.
///
/// `amount_x = lp_amount × reserve_x / total_supply` (floor division).
/// The fractional remainder stays in the pool, slightly benefiting
/// remaining LPs.
///
/// `min_assets` (optional) protects against sandwich attacks: if an
/// attacker front-runs the withdrawal with a large swap to skew reserves,
/// the returned amounts will fall below the minimums and the tx reverts.
///
/// Auth: only callable via CW20 Send from the pair's LP token contract.
fn execute_withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    lp_amount: Uint128,
    min_assets: Option<[Uint128; 2]>,
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
    oracle_update(deps.storage, env.block.time.seconds(), reserve_a, reserve_b)?;

    if total_supply.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    let numerator_a = lp_amount.checked_mul(reserve_a)?;
    let amount_a = numerator_a.checked_div(total_supply)?;
    let numerator_b = lp_amount.checked_mul(reserve_b)?;
    let amount_b = numerator_b.checked_div(total_supply)?;

    // Sanity: floor-division rounding loses < 1 token per asset.
    // numerator - amount * total_supply must be < total_supply.
    let rem_a = numerator_a - amount_a.checked_mul(total_supply)?;
    if rem_a >= total_supply {
        return Err(ContractError::InvariantViolation {
            reason: format!("withdraw-A floor rounding exceeds 1 token: rem={}", rem_a),
        });
    }
    let rem_b = numerator_b - amount_b.checked_mul(total_supply)?;
    if rem_b >= total_supply {
        return Err(ContractError::InvariantViolation {
            reason: format!("withdraw-B floor rounding exceeds 1 token: rem={}", rem_b),
        });
    }

    if let Some([min_a, min_b]) = min_assets {
        if amount_a < min_a {
            return Err(ContractError::WithdrawSlippageExceeded {
                asset: pair_info.asset_infos[0].to_string(),
                actual: amount_a.to_string(),
                min: min_a.to_string(),
            });
        }
        if amount_b < min_b {
            return Err(ContractError::WithdrawSlippageExceeded {
                asset: pair_info.asset_infos[1].to_string(),
                actual: amount_b.to_string(),
                min: min_b.to_string(),
            });
        }
    }

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

/// Update the swap fee rate. Factory (governance) only.
/// `fee_bps` is in basis points (0–10000, where 10000 = 100%).
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

/// Replace the list of post-swap hook contracts. Factory (governance) only.
/// Hooks are called after every successful swap with the swap details.
/// A reverting hook will block the entire swap — only register trusted,
/// audited hook contracts.
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

/// Grow the TWAP oracle ring buffer. Permissionless (caller pays gas).
/// Larger cardinality supports longer TWAP windows. Bounded by
/// `MAX_OBSERVATION_CARDINALITY` (65 000 slots ≈ 109 hours at 6s blocks).
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

/// Set or clear the fee discount registry contract. Factory (governance) only.
/// When set, swaps query this registry for trader-specific fee discounts.
/// If the registry query fails, the pair silently falls back to the full fee.
fn execute_set_discount_registry(
    deps: DepsMut,
    info: MessageInfo,
    registry: Option<String>,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    let validated = match registry {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };

    DISCOUNT_REGISTRY.save(deps.storage, &validated)?;

    let registry_str = validated
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "none".to_string());

    Ok(Response::new()
        .add_attribute("action", "set_discount_registry")
        .add_attribute("registry", registry_str))
}

/// Emergency pause/unpause. Factory (governance) only.
/// When paused, all CW20 Receive messages (swaps, **limit placement**, LP
/// withdrawals via Send) and ProvideLiquidity are blocked; **`CancelLimitOrder`
/// is also blocked**, so resting limit escrow cannot be withdrawn until
/// unpause. Admin-only operations (fee updates, hooks, sweep) remain available.
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

/// Recover tokens that exceed tracked reserves (donations or accidental
/// transfers). Factory (governance) only. Sends the excess
/// (`actual_balance - internal_reserves`) to `recipient`. Does NOT modify
/// internal reserves — pool pricing is unaffected.
fn execute_sweep(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: String,
    recipient: String,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    let token_addr = deps.api.addr_validate(&token)?;
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let reserve_for_token = if pair_info.asset_infos[0].equal(&AssetInfo::Token {
        contract_addr: token_addr.to_string(),
    }) {
        reserve_a
    } else if pair_info.asset_infos[1].equal(&AssetInfo::Token {
        contract_addr: token_addr.to_string(),
    }) {
        reserve_b
    } else {
        Uint128::zero()
    };

    let actual_balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        token_addr.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let pending_escrow = if pair_info.asset_infos[0].equal(&AssetInfo::Token {
        contract_addr: token_addr.to_string(),
    }) {
        PENDING_ESCROW_TOKEN0
            .may_load(deps.storage)?
            .unwrap_or_default()
    } else if pair_info.asset_infos[1].equal(&AssetInfo::Token {
        contract_addr: token_addr.to_string(),
    }) {
        PENDING_ESCROW_TOKEN1
            .may_load(deps.storage)?
            .unwrap_or_default()
    } else {
        Uint128::zero()
    };

    let excess = actual_balance
        .balance
        .checked_sub(reserve_for_token)
        .unwrap_or(Uint128::zero())
        .checked_sub(pending_escrow)
        .unwrap_or(Uint128::zero());

    if excess.is_zero() {
        return Err(ContractError::NothingToSweep {
            token: token_addr.to_string(),
        });
    }

    let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient_addr.to_string(),
            amount: excess,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "sweep")
        .add_attribute("token", token_addr)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("amount", excess))
}

/// Update the LP token's CosmWasm admin address. Factory only.
/// Used by the factory to propagate governance changes to LP token contracts.
fn execute_set_lp_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    if info.sender != pair_info.factory {
        return Err(ContractError::Unauthorized {});
    }

    let validated_admin = deps.api.addr_validate(&new_admin)?;

    let update_admin_msg = CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
        contract_addr: pair_info.lp_token.to_string(),
        admin: validated_admin.to_string(),
    });

    Ok(Response::new()
        .add_message(update_admin_msg)
        .add_attribute("action", "set_lp_admin")
        .add_attribute("new_admin", validated_admin))
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
        QueryMsg::IsPaused {} => to_json_binary(&query_is_paused(deps)?),
        QueryMsg::LimitOrder { order_id } => to_json_binary(&query_limit_order(deps, order_id)?),
        QueryMsg::ExpiredLimitRefund { order_id } => {
            to_json_binary(&query_expired_limit_refund(deps, order_id)?)
        }
        QueryMsg::OrderBookHead { side } => to_json_binary(&query_order_book_head(deps, side)?),
        QueryMsg::HybridSimulation {
            offer_asset,
            hybrid,
        } => to_json_binary(&query_hybrid_simulation(deps, &env, offer_asset, hybrid)?),
        QueryMsg::HybridReverseSimulation { ask_asset, hybrid } => to_json_binary(
            &query_hybrid_reverse_simulation(deps, &env, ask_asset, hybrid)?,
        ),
    }
}

fn query_is_paused(deps: Deps) -> StdResult<dex_common::pair::PausedResponse> {
    let paused = PAUSED.may_load(deps.storage)?.unwrap_or(false);
    Ok(dex_common::pair::PausedResponse { paused })
}

fn query_limit_order(deps: Deps, order_id: u64) -> StdResult<LimitOrderResponse> {
    orderbook::load_order_response(deps.storage, order_id)
}

fn query_expired_limit_refund(
    deps: Deps,
    order_id: u64,
) -> StdResult<Option<ExpiredLimitRefundResponse>> {
    Ok(EXPIRED_LIMIT_CLAIMS
        .may_load(deps.storage, order_id)?
        .map(|r| ExpiredLimitRefundResponse {
            order_id,
            owner: r.owner,
            side: r.side,
            remaining: r.remaining,
            expires_at: r.expires_at,
        }))
}

fn query_order_book_head(deps: Deps, side: LimitOrderSide) -> StdResult<Option<u64>> {
    orderbook::query_head(deps.storage, side)
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

fn scale_hybrid_template(
    hybrid: &HybridSwapParams,
    total: Uint128,
) -> Result<HybridSwapParams, ContractError> {
    let den = hybrid.pool_input.checked_add(hybrid.book_input)?;
    if den.is_zero() {
        return Err(ContractError::InvalidHybridParams {
            reason: "hybrid template sum must be positive".into(),
        });
    }
    let book = total.checked_mul(hybrid.book_input)?.checked_div(den)?;
    let pool = total.checked_sub(book)?;
    Ok(HybridSwapParams {
        pool_input: pool,
        book_input: book,
        max_maker_fills: hybrid.max_maker_fills,
        book_start_hint: hybrid.book_start_hint,
    })
}

fn simulate_hybrid_swap(
    deps: Deps,
    env: &Env,
    input_amount: Uint128,
    hybrid: &HybridSwapParams,
    offer_asset_info: &AssetInfo,
) -> Result<HybridSimulationResponse, ContractError> {
    if input_amount.is_zero() {
        return Ok(HybridSimulationResponse {
            return_amount: Uint128::zero(),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
            book_return_amount: Uint128::zero(),
            pool_return_amount: Uint128::zero(),
        });
    }
    if hybrid.pool_input.checked_add(hybrid.book_input)? != input_amount {
        return Err(ContractError::HybridSplitMismatch {});
    }
    if hybrid.book_input > Uint128::zero() && hybrid.max_maker_fills == 0 {
        return Err(ContractError::InvalidHybridParams {
            reason: "max_maker_fills must be > 0 when book_input > 0".into(),
        });
    }
    let pool_leg = hybrid.pool_input;
    let book_leg = hybrid.book_input;
    let max_makers = hybrid.max_maker_fills;
    let book_hint = hybrid.book_start_hint;

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    let effective_fee_bps = fee_config.fee_bps;

    let (input_reserve, output_reserve, _ask_asset_info) =
        if offer_asset_info.equal(&pair_info.asset_infos[0]) {
            (reserve_a, reserve_b, pair_info.asset_infos[1].clone())
        } else if offer_asset_info.equal(&pair_info.asset_infos[1]) {
            (reserve_b, reserve_a, pair_info.asset_infos[0].clone())
        } else {
            return Err(ContractError::InvalidToken {});
        };

    let offer_token_addr = token_addr(offer_asset_info);
    let token_a_addr = token_addr(&pair_info.asset_infos[0]);

    let mut book_return_net = Uint128::zero();
    let mut offer_consumed_by_book = Uint128::zero();

    if book_leg > Uint128::zero() {
        if offer_token_addr == token_a_addr {
            let (t1_out, t0_used, _mk) = orderbook::simulate_match_bids(
                deps.storage,
                env.block.time.seconds(),
                book_leg,
                max_makers,
                book_hint,
                effective_fee_bps,
            )?;
            book_return_net = t1_out;
            offer_consumed_by_book = t0_used;
        } else {
            let (t0_out, t1_used, _mk) = orderbook::simulate_match_asks(
                deps.storage,
                env.block.time.seconds(),
                book_leg,
                max_makers,
                book_hint,
                effective_fee_bps,
            )?;
            book_return_net = t0_out;
            offer_consumed_by_book = t1_used;
        }
    }

    let pool_input_amount = pool_leg.checked_add(book_leg.checked_sub(offer_consumed_by_book)?)?;

    let mut return_amount = Uint128::zero();
    let mut commission_amount = Uint128::zero();
    let mut spread_amount = Uint128::zero();

    if pool_input_amount > Uint128::zero() {
        if input_reserve.is_zero() || output_reserve.is_zero() {
            return Err(ContractError::InsufficientLiquidity {});
        }
        let k = input_reserve.checked_mul(output_reserve)?;
        let new_input_reserve = input_reserve.checked_add(pool_input_amount)?;
        let new_output_reserve = ceil_div(k, new_input_reserve);
        let gross_output = output_reserve.checked_sub(new_output_reserve)?;

        let fee_numerator = gross_output.checked_mul(Uint128::new(effective_fee_bps as u128))?;
        commission_amount = fee_numerator.checked_div(Uint128::new(10000))?;
        return_amount = gross_output.checked_sub(commission_amount)?;

        spread_amount = spot_linear_spread_over_gross(
            pool_input_amount,
            input_reserve,
            output_reserve,
            gross_output,
        )
        .map_err(ContractError::Std)?;
    }

    let total_out = book_return_net.checked_add(return_amount)?;
    Ok(HybridSimulationResponse {
        return_amount: total_out,
        spread_amount,
        commission_amount,
        book_return_amount: book_return_net,
        pool_return_amount: return_amount,
    })
}

fn query_hybrid_simulation(
    deps: Deps,
    env: &Env,
    offer_asset: Asset,
    hybrid: HybridSwapParams,
) -> StdResult<HybridSimulationResponse> {
    simulate_hybrid_swap(deps, env, offer_asset.amount, &hybrid, &offer_asset.info)
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))
}

fn query_hybrid_reverse_simulation(
    deps: Deps,
    env: &Env,
    ask_asset: Asset,
    hybrid: HybridSwapParams,
) -> StdResult<HybridReverseSimulationResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let offer_info = if ask_asset.info.equal(&pair_info.asset_infos[0]) {
        pair_info.asset_infos[1].clone()
    } else if ask_asset.info.equal(&pair_info.asset_infos[1]) {
        pair_info.asset_infos[0].clone()
    } else {
        return Err(cosmwasm_std::StdError::generic_err(
            "Invalid ask asset: does not match pair assets",
        ));
    };

    let ask_target = ask_asset.amount;
    if ask_target.is_zero() {
        return Ok(HybridReverseSimulationResponse {
            offer_amount: Uint128::zero(),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
            book_return_amount: Uint128::zero(),
            pool_return_amount: Uint128::zero(),
        });
    }

    let mut hi = Uint128::new(1u128);
    for _ in 0..128 {
        let sim = simulate_hybrid_swap(
            deps,
            env,
            hi,
            &scale_hybrid_template(&hybrid, hi)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?,
            &offer_info,
        )
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
        if sim.return_amount >= ask_target {
            break;
        }
        hi = hi
            .checked_mul(Uint128::new(2u128))
            .map_err(|_| cosmwasm_std::StdError::generic_err("offer overflow"))?;
    }

    let check_hi = simulate_hybrid_swap(
        deps,
        env,
        hi,
        &scale_hybrid_template(&hybrid, hi)
            .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?,
        &offer_info,
    )
    .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
    if check_hi.return_amount < ask_target {
        return Err(cosmwasm_std::StdError::generic_err(
            "Insufficient liquidity for hybrid reverse simulation",
        ));
    }

    let mut lo = Uint128::new(1u128);
    let mut r_hi = hi;
    while lo < r_hi {
        let mid = lo.checked_add(r_hi)?.checked_div(Uint128::new(2u128))?;
        if mid.is_zero() {
            break;
        }
        let sim_mid = simulate_hybrid_swap(
            deps,
            env,
            mid,
            &scale_hybrid_template(&hybrid, mid)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?,
            &offer_info,
        )
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
        if sim_mid.return_amount >= ask_target {
            r_hi = mid;
        } else {
            lo = mid.checked_add(Uint128::new(1u128))?;
        }
    }

    let final_h = scale_hybrid_template(&hybrid, r_hi)
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
    let sim = simulate_hybrid_swap(deps, env, r_hi, &final_h, &offer_info)
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
    Ok(HybridReverseSimulationResponse {
        offer_amount: r_hi,
        spread_amount: sim.spread_amount,
        commission_amount: sim.commission_amount,
        book_return_amount: sim.book_return_amount,
        pool_return_amount: sim.pool_return_amount,
    })
}

/// Constant-product pool simulation only — **does not include** the on-chain limit book
/// or Pattern C hybrid fills. See `docs/limit-orders.md` and invariant L8 in
/// `docs/contracts-security-audit.md`.
fn query_simulation(deps: Deps, offer_asset: Asset) -> StdResult<SimulationResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let (input_reserve, output_reserve) = if offer_asset.info.equal(&pair_info.asset_infos[0]) {
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
    let new_output_reserve = ceil_div(k, new_input_reserve);
    let gross_output = output_reserve.checked_sub(new_output_reserve)?;

    let commission_amount = gross_output
        .checked_mul(Uint128::new(fee_config.fee_bps as u128))?
        .checked_div(Uint128::new(10000))?;
    let return_amount = gross_output.checked_sub(commission_amount)?;

    let spread_amount =
        spot_linear_spread_over_gross(offer_amount, input_reserve, output_reserve, gross_output)?;

    Ok(SimulationResponse {
        return_amount,
        spread_amount,
        commission_amount,
    })
}

/// Pool-only reverse simulation; **ignores** limit-book liquidity (same scope as `query_simulation`).
fn query_reverse_simulation(deps: Deps, ask_asset: Asset) -> StdResult<ReverseSimulationResponse> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let (reserve_a, reserve_b) = RESERVES.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;

    let (input_reserve, output_reserve) = if ask_asset.info.equal(&pair_info.asset_infos[1]) {
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

    let spread_amount =
        spot_linear_spread_over_gross(offer_amount, input_reserve, output_reserve, gross_needed)?;

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

    let mut price_a_cumulatives = Vec::with_capacity(seconds_ago.len());
    let mut price_b_cumulatives = Vec::with_capacity(seconds_ago.len());
    for &sa in &seconds_ago {
        let (cum_a, cum_b) = oracle_observe_single(
            deps.storage,
            block_time,
            sa,
            &state,
            &latest,
            reserve_a,
            reserve_b,
        )?;
        price_a_cumulatives.push(cum_a);
        price_b_cumulatives.push(cum_b);
    }

    Ok(ObserveResponse {
        price_a_cumulatives,
        price_b_cumulatives,
    })
}

fn query_oracle_info(deps: Deps) -> Result<OracleInfoResponse, ContractError> {
    let state = ORACLE_STATE.load(deps.storage)?;

    if state.cardinality_initialized == 0 {
        return Ok(OracleInfoResponse {
            observation_cardinality: state.cardinality,
            observation_index: state.index,
            observations_stored: 0,
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
        observations_stored: state.cardinality_initialized,
        oldest_observation_timestamp: oldest.timestamp,
        newest_observation_timestamp: latest.timestamp,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Match user-supplied assets to the pair's canonical ordering.
/// Returns `(amount_a, amount_b)` regardless of the order the caller
/// provided them in.
fn match_asset_amounts(
    pair_asset_infos: &[AssetInfo; 2],
    provided: &[Asset; 2],
) -> Result<(Uint128, Uint128), ContractError> {
    if provided[0].info.equal(&pair_asset_infos[0]) && provided[1].info.equal(&pair_asset_infos[1])
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

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)
        .map_err(ContractError::Std)?;

    if ORDER_NEXT_ID.may_load(deps.storage)?.is_none() {
        ORDER_NEXT_ID.save(deps.storage, &1u64)?;
    }
    if PENDING_ESCROW_TOKEN0.may_load(deps.storage)?.is_none() {
        PENDING_ESCROW_TOKEN0.save(deps.storage, &Uint128::zero())?;
    }
    if PENDING_ESCROW_TOKEN1.may_load(deps.storage)?.is_none() {
        PENDING_ESCROW_TOKEN1.save(deps.storage, &Uint128::zero())?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("version", CONTRACT_VERSION))
}

#[cfg(test)]
mod max_spread_tests {
    use cosmwasm_std::{Decimal, Uint128};

    use super::{assert_max_spread, spot_linear_spread_over_gross};
    use crate::error::ContractError;

    #[test]
    fn spot_linear_spread_matches_floor_ideal_minus_gross() {
        let s = spot_linear_spread_over_gross(
            Uint128::new(100),
            Uint128::new(1000),
            Uint128::new(500),
            Uint128::new(40),
        )
        .unwrap();
        let ideal = 100u128 * 500 / 1000;
        assert_eq!(ideal, 50);
        assert_eq!(s, Uint128::new(10));
    }

    /// When the spot-linear estimate exceeds gross by more than 100% of gross
    /// (integer rounding), raw `spread_amount` can be larger than
    /// `return + commission`. `max_spread = 100%` must still accept the swap.
    #[test]
    fn max_spread_one_allows_capped_ratio_when_raw_spread_exceeds_gross() {
        assert_max_spread(
            None,
            Some(Decimal::one()),
            Uint128::new(1),
            Uint128::new(90),
            Uint128::new(150),
            Uint128::new(10),
            Uint128::zero(),
        )
        .unwrap();
    }

    #[test]
    fn max_spread_half_rejects_at_full_slippage_metric() {
        let err = assert_max_spread(
            None,
            Some(Decimal::percent(50)),
            Uint128::new(1),
            Uint128::new(90),
            Uint128::new(150),
            Uint128::new(10),
            Uint128::zero(),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::MaxSpreadAssertion { .. }));
    }

    /// Hybrid no-belief: denominator includes `book_return_net`.
    #[test]
    fn max_spread_no_belief_denominator_includes_book_net() {
        let book = Uint128::new(50);
        let pool_gross = Uint128::new(100);
        let spread = Uint128::new(5);
        let denom = pool_gross.checked_add(book).unwrap();
        let exact_max = Decimal::from_ratio(spread.min(pool_gross), denom);
        assert_max_spread(
            None,
            Some(exact_max),
            Uint128::new(1),
            Uint128::new(90),
            spread,
            Uint128::new(10),
            book,
        )
        .unwrap();
        let tighter = Decimal::from_ratio(4u128, 150u128);
        let err = assert_max_spread(
            None,
            Some(tighter),
            Uint128::new(1),
            Uint128::new(90),
            spread,
            Uint128::new(10),
            book,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::MaxSpreadAssertion { .. }));
    }

    /// Belief path: `actual_return` includes pool commission.
    #[test]
    fn max_spread_belief_counts_pool_commission_in_actual_return() {
        let offer = Uint128::new(100);
        let belief_price = Decimal::from_ratio(Uint128::new(1), Uint128::new(2));
        let expected_return = offer * (Decimal::one() / belief_price);
        let book_net = Uint128::new(10);
        let pool_net = Uint128::new(170);
        let pool_commission = Uint128::new(20);
        let actual_with_commission = book_net
            .checked_add(pool_net)
            .unwrap()
            .checked_add(pool_commission)
            .unwrap();
        assert_eq!(actual_with_commission, expected_return);
        let max_tight = Decimal::permille(4);
        assert_max_spread(
            Some(belief_price),
            Some(max_tight),
            offer,
            pool_net,
            Uint128::zero(),
            pool_commission,
            book_net,
        )
        .unwrap();
        let err = assert_max_spread(
            Some(belief_price),
            Some(max_tight),
            offer,
            pool_net.checked_sub(Uint128::new(1)).unwrap(),
            Uint128::zero(),
            pool_commission,
            book_net,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::MaxSpreadAssertion { .. }));
    }
}
