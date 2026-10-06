use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::blacklist_guard;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use crate::state::{SwapState, FACTORY, SWAP_STATE, WRAP_MAPPER};
use dex_common::max_spread::{self, CheckMaxSpreadError};
use dex_common::pair;
use dex_common::types::Asset;
use dex_common::wrap_mapper;

const CONTRACT_NAME: &str = "cl8y-dex-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SWAP_REPLY_ID: u64 = 1;
const SETTLEMENT_REPLY_ID: u64 = 2;
const MAX_HOPS: usize = 4;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let factory = deps.api.addr_validate(&msg.factory)?;
    FACTORY.save(deps.storage, &factory)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("factory", factory))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => execute_receive(deps, env, info, cw20_msg),
        ExecuteMsg::ExecuteSwapOperations { .. } => {
            Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Use CW20 Send to initiate swaps through the router",
            )))
        }
        ExecuteMsg::SetWrapMapper { wrap_mapper } => {
            execute_set_wrap_mapper(deps, info, wrap_mapper)
        }
    }
}

fn execute_set_wrap_mapper(
    deps: DepsMut,
    info: MessageInfo,
    wrap_mapper_addr: String,
) -> Result<Response, ContractError> {
    let factory = FACTORY.load(deps.storage)?;
    let factory_config: dex_common::factory::ConfigResponse = deps.querier.query_wasm_smart(
        factory.to_string(),
        &dex_common::factory::QueryMsg::Config {},
    )?;

    if info.sender != factory_config.governance {
        return Err(ContractError::Unauthorized {});
    }

    let validated = deps.api.addr_validate(&wrap_mapper_addr)?;
    WRAP_MAPPER.save(deps.storage, &validated)?;

    Ok(Response::new()
        .add_attribute("action", "set_wrap_mapper")
        .add_attribute("wrap_mapper", validated))
}

/// Revert if the current block time exceeds the user-supplied deadline.
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

/// Handle CW20 Receive — the entry point for all router swaps.
/// Parses the hook message and delegates to `execute_swap_operations`.
fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let input_token = info.sender.clone();
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    let amount = cw20_msg.amount;

    let hook_msg: Cw20HookMsg = from_json(&cw20_msg.msg)?;
    match hook_msg {
        Cw20HookMsg::ExecuteRouteSwapOperations {
            funding_version,
            operations,
            minimum_receive,
            to,
            deadline,
            unwrap_output,
            max_spread,
        } => {
            if funding_version != dex_common::route_tax::FUNDING_VERSION {
                return Err(cosmwasm_std::StdError::generic_err(
                    "unsupported route tax funding version",
                )
                .into());
            }
            assert_deadline(&env, deadline)?;
            execute_swap_operations(
                deps,
                env,
                sender,
                input_token,
                amount,
                operations,
                minimum_receive,
                to,
                unwrap_output,
                max_spread,
            )
        }

        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
            deadline,
            unwrap_output,
            max_spread,
        } => {
            for op in &operations {
                if let SwapOperation::TerraSwap {
                    offer_asset_info,
                    ask_asset_info,
                    ..
                } = op
                {
                    for asset in [offer_asset_info, ask_asset_info] {
                        if let Some(c) = crate::route_tax::capability(
                            deps.as_ref(),
                            &Addr::unchecked(asset.assert_is_token()?),
                        )? {
                            if c.buy_bps > 0 || c.sell_bps > 0 {
                                return Err(cosmwasm_std::StdError::generic_err("taxed routes require execute_route_swap_operations funding_version 1").into());
                            }
                        }
                    }
                }
            }
            assert_deadline(&env, deadline)?;
            execute_swap_operations(
                deps,
                env,
                sender,
                input_token,
                amount,
                operations,
                minimum_receive,
                to,
                unwrap_output,
                max_spread,
            )
        }
    }
}

/// Initiate a multi-hop swap chain.
///
/// 1. Validate operations are non-empty and no swap is already in progress.
/// 2. Resolve the first pair via factory query.
/// 3. Store `SwapState` (sender, recipient, remaining ops, minimum_receive).
/// 4. Send input tokens to the first pair via CW20 Send with SubMsg reply.
/// 5. Each hop is handled in `reply_swap_hop`.
fn query_router_cw20_balance(
    deps: Deps,
    router: &Addr,
    token: &Addr,
) -> Result<Uint128, ContractError> {
    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: router.to_string(),
        },
    )?;
    Ok(balance.balance)
}

fn hop_output_from_balance(
    balance_after: Uint128,
    pre_hop_balance: Uint128,
) -> Result<Uint128, ContractError> {
    let hop_output = balance_after.checked_sub(pre_hop_balance).map_err(|_| {
        ContractError::Std(cosmwasm_std::StdError::generic_err(
            "hop balance decreased unexpectedly",
        ))
    })?;
    if hop_output.is_zero() {
        return Err(ContractError::ZeroHopOutput {});
    }
    Ok(hop_output)
}

/// Net native amount delivered to the recipient after wrap-mapper unwrap fee.
fn net_after_wrap_mapper_unwrap_fee(
    wrapped_amount: Uint128,
    fee_bps: u16,
) -> Result<Uint128, ContractError> {
    let fee = wrapped_amount.multiply_ratio(Uint128::from(fee_bps as u128), Uint128::new(10_000));
    wrapped_amount
        .checked_sub(fee)
        .map_err(|e| ContractError::Std(e.into()))
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_operations(
    deps: DepsMut,
    env: Env,
    sender: cosmwasm_std::Addr,
    input_token: cosmwasm_std::Addr,
    amount: Uint128,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<String>,
    unwrap_output: Option<bool>,
    max_spread: cosmwasm_std::Decimal,
) -> Result<Response, ContractError> {
    if operations.is_empty() {
        return Err(ContractError::EmptyOperations {});
    }

    if operations.len() > MAX_HOPS {
        return Err(ContractError::TooManyHops {
            max: MAX_HOPS,
            actual: operations.len(),
        });
    }

    let (first_hybrid_precheck, first_hop_min_return) = match &operations[0] {
        SwapOperation::TerraSwap {
            hybrid, min_return, ..
        } => (hybrid.clone(), *min_return),
        SwapOperation::NativeSwap { .. } => return Err(ContractError::NativeSwapNotSupported {}),
    };

    validate_hybrid_hop_execute_slippage_floor(&first_hybrid_precheck, first_hop_min_return)?;

    if SWAP_STATE.may_load(deps.storage)?.is_some() {
        return Err(ContractError::SwapInProgress {});
    }

    let do_unwrap = unwrap_output == Some(true);
    if do_unwrap {
        WRAP_MAPPER
            .may_load(deps.storage)?
            .ok_or(ContractError::WrapMapperNotSet {})?;
    }

    let factory = FACTORY.load(deps.storage)?;
    blacklist_guard::assert_router_swap_not_blacklisted(
        deps.as_ref(),
        &factory,
        &sender,
        &input_token,
        &operations,
    )?;

    let recipient = match to {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => sender.clone(),
    };

    let first_op = &operations[0];
    let remaining = operations[1..].to_vec();

    let (pair_addr, ask_asset_info) = resolve_operation(deps.as_ref(), &factory, first_op)?;

    let ask_addr_str = ask_asset_info
        .assert_is_token()
        .map_err(|_| ContractError::NativeTokenNotSupported {})?;
    let output_token = deps.api.addr_validate(ask_addr_str)?;
    let pre_hop_balance =
        query_router_cw20_balance(deps.as_ref(), &env.contract.address, &output_token)?;

    let funded = crate::route_tax::preview(
        deps.as_ref(),
        &input_token,
        &pair_addr,
        amount,
        dex_common::route_tax::RouteTaxKind::Sell,
        Some(sender.to_string()),
        Some(&env.contract.address),
    )?;
    let residue = amount.checked_sub(funded.economic.checked_add(funded.tax)?)?;
    let refunds = if residue.is_zero() {
        vec![]
    } else {
        vec![dex_common::route_tax::RouteTransfer {
            token: input_token.clone(),
            recipient: sender.clone(),
            amount: residue,
        }]
    };
    let tax_context = dex_common::route_tax::RouteTaxContextResponse {
        funding_version: dex_common::route_tax::FUNDING_VERSION,
        trader: sender.clone(),
        pair: pair_addr.clone(),
        offer_token: input_token.clone(),
        ask_token: output_token.clone(),
        gross: amount,
        economic: funded.economic,
        transfers: vec![],
        acquired_tokens: vec![],
    };
    SWAP_STATE.save(
        deps.storage,
        &SwapState {
            sender: sender.clone(),
            tax_context,
            refunds,
            recipient: recipient.clone(),
            remaining_operations: remaining,
            minimum_receive,
            output_token,
            pre_hop_balance,
            unwrap_output: do_unwrap,
            max_spread,
        },
    )?;

    let (first_hybrid, first_greedy, first_min_return) = match &first_op {
        SwapOperation::TerraSwap {
            hybrid,
            greedy,
            min_return,
            ..
        } => (hybrid.clone(), greedy.clone(), *min_return),
        SwapOperation::NativeSwap { .. } => (None, None, None),
    };
    let first_min_return = first_min_return
        .map(|m| {
            crate::route_tax::reverse_buy(
                deps.as_ref(),
                &SWAP_STATE.load(deps.storage)?.output_token,
                &pair_addr,
                m,
                Some(sender.to_string()),
                &env.contract.address,
            )
        })
        .transpose()?;
    let first_hybrid = crate::route_tax::funded_hybrid(first_hybrid, amount, funded.economic)?;
    validate_hybrid_declared_split_for_no_belief(funded.economic, &first_hybrid)?;
    validate_greedy_hop_mutex(&first_hybrid, &first_greedy)?;
    validate_greedy_hop_execute_slippage_floor(&first_greedy, first_min_return)?;

    let swap_msg = pair::Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(max_spread),
        min_return: first_min_return,
        to: None,
        deadline: None,
        trader: Some(sender.to_string()),
        hybrid: first_hybrid,
        greedy: first_greedy,
    };

    let send_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: input_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: pair_addr.to_string(),
                amount: funded.economic,
                msg: to_json_binary(&swap_msg)?,
            })?,
            funds: vec![],
        }),
        SWAP_REPLY_ID,
    );

    Ok(Response::new()
        .add_submessage(send_msg)
        .add_attribute("action", "execute_swap_operations")
        .add_attribute("sender", sender)
        .add_attribute("recipient", recipient))
}

/// Look up the pair contract address for a single swap operation by
/// querying the factory. Returns the pair address and the ask asset info.
fn resolve_operation(
    deps: Deps,
    factory: &cosmwasm_std::Addr,
    op: &SwapOperation,
) -> Result<(cosmwasm_std::Addr, dex_common::types::AssetInfo), ContractError> {
    match op {
        SwapOperation::NativeSwap { .. } => Err(ContractError::NativeSwapNotSupported {}),
        SwapOperation::TerraSwap {
            offer_asset_info,
            ask_asset_info,
            ..
        } => {
            offer_asset_info
                .assert_is_token()
                .map_err(|_| ContractError::NativeTokenNotSupported {})?;
            ask_asset_info
                .assert_is_token()
                .map_err(|_| ContractError::NativeTokenNotSupported {})?;

            let pair_response: dex_common::factory::PairResponse = deps
                .querier
                .query_wasm_smart(
                    factory.to_string(),
                    &dex_common::factory::QueryMsg::Pair {
                        asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                    },
                )
                .map_err(|_| ContractError::PairNotFound {})?;

            Ok((pair_response.pair.contract_addr, ask_asset_info.clone()))
        }
    }
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        SWAP_REPLY_ID => reply_swap_hop(deps, env),
        SETTLEMENT_REPLY_ID => {
            SWAP_STATE.remove(deps.storage);
            Ok(Response::new().add_attribute("action", "route_settled"))
        }
        id => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            format!("unknown reply id: {id}"),
        ))),
    }
}

/// Handle the completion of each swap hop.
///
/// If operations remain: compute this hop's output delta, resolve the next pair,
/// snapshot the next output token's pre-hop balance, and chain another SubMsg swap.
///
/// If this was the final hop: assert `minimum_receive` against the amount the
/// recipient receives (post-unwrap net when `unwrap_output`), transfer hop
/// output to the recipient (or unwrap via wrap-mapper), and clear `SWAP_STATE`.
fn reply_swap_hop(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut state = SWAP_STATE.load(deps.storage)?;

    let balance_after =
        query_router_cw20_balance(deps.as_ref(), &env.contract.address, &state.output_token)?;
    let hop_output = hop_output_from_balance(balance_after, state.pre_hop_balance)?;
    if !state
        .tax_context
        .acquired_tokens
        .contains(&state.output_token)
    {
        state
            .tax_context
            .acquired_tokens
            .push(state.output_token.clone());
    }

    if state.remaining_operations.is_empty() {
        let wrap_mapper_addr = if state.unwrap_output {
            Some(
                WRAP_MAPPER
                    .may_load(deps.storage)?
                    .ok_or(ContractError::WrapMapperNotSet {})?,
            )
        } else {
            None
        };

        let delivered_amount = if let Some(ref mapper) = wrap_mapper_addr {
            let mapper_config: wrap_mapper::ConfigResponse = deps
                .querier
                .query_wasm_smart(mapper.to_string(), &wrap_mapper::QueryMsg::Config {})?;
            net_after_wrap_mapper_unwrap_fee(hop_output, mapper_config.unwrap_fee_bps()?)?
        } else {
            hop_output
        };

        if let Some(min) = state.minimum_receive {
            if delivered_amount < min {
                return Err(ContractError::MinimumReceiveAssertion {
                    minimum: min.to_string(),
                    actual: delivered_amount.to_string(),
                });
            }
        }

        state.tax_context.transfers = state.refunds.clone();
        state
            .tax_context
            .transfers
            .push(dex_common::route_tax::RouteTransfer {
                token: state.output_token.clone(),
                recipient: state.recipient.clone(),
                amount: hop_output,
            });
        SWAP_STATE.save(deps.storage, &state)?;
        let refund_messages = state
            .refunds
            .iter()
            .map(|r| crate::route_tax::transfer(deps.as_ref(), &r.token, &r.recipient, r.amount))
            .collect::<StdResult<Vec<_>>>()?;
        let output_msg = if let Some(mapper) = wrap_mapper_addr {
            let unwrap_hook = wrap_mapper::Cw20HookMsg::Unwrap {
                recipient: Some(state.recipient.to_string()),
            };

            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.output_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: mapper.to_string(),
                    amount: hop_output,
                    msg: to_json_binary(&unwrap_hook)?,
                })?,
                funds: vec![],
            })
        } else {
            crate::route_tax::transfer(
                deps.as_ref(),
                &state.output_token,
                &state.recipient,
                hop_output,
            )?
        };

        return Ok(Response::new()
            .add_messages(refund_messages)
            .add_submessage(SubMsg::reply_on_success(output_msg, SETTLEMENT_REPLY_ID))
            .add_attribute("action", "swap_complete")
            .add_attribute("output_amount", hop_output)
            .add_attribute("recipient", state.recipient));
    }

    let factory = FACTORY.load(deps.storage)?;
    let next_op = state.remaining_operations.remove(0);
    let (pair_addr, ask_asset_info) = resolve_operation(deps.as_ref(), &factory, &next_op)?;

    let current_token = state.output_token.clone();
    let ask_addr_str = ask_asset_info
        .assert_is_token()
        .map_err(|_| ContractError::NativeTokenNotSupported {})?;
    state.output_token = deps.api.addr_validate(ask_addr_str)?;
    state.pre_hop_balance =
        query_router_cw20_balance(deps.as_ref(), &env.contract.address, &state.output_token)?;

    let funded = crate::route_tax::preview(
        deps.as_ref(),
        &current_token,
        &pair_addr,
        hop_output,
        dex_common::route_tax::RouteTaxKind::Sell,
        Some(state.sender.to_string()),
        Some(&env.contract.address),
    )?;
    let residue = hop_output.checked_sub(funded.economic.checked_add(funded.tax)?)?;
    if !residue.is_zero() {
        state.refunds.push(dex_common::route_tax::RouteTransfer {
            token: current_token.clone(),
            recipient: state.sender.clone(),
            amount: residue,
        });
    }
    state.tax_context = dex_common::route_tax::RouteTaxContextResponse {
        funding_version: dex_common::route_tax::FUNDING_VERSION,
        trader: state.sender.clone(),
        pair: pair_addr.clone(),
        offer_token: current_token.clone(),
        ask_token: state.output_token.clone(),
        gross: hop_output,
        economic: funded.economic,
        transfers: vec![],
        acquired_tokens: state.tax_context.acquired_tokens.clone(),
    };
    SWAP_STATE.save(deps.storage, &state)?;

    let (hop_hybrid, hop_greedy, hop_min_return) = match &next_op {
        SwapOperation::TerraSwap {
            hybrid,
            greedy,
            min_return,
            ..
        } => (hybrid.clone(), greedy.clone(), *min_return),
        SwapOperation::NativeSwap { .. } => (None, None, None),
    };
    let hop_min_return = hop_min_return
        .map(|m| {
            crate::route_tax::reverse_buy(
                deps.as_ref(),
                &state.output_token,
                &pair_addr,
                m,
                Some(state.sender.to_string()),
                &env.contract.address,
            )
        })
        .transpose()?;
    let hop_hybrid = crate::route_tax::funded_hybrid(hop_hybrid, hop_output, funded.economic)?;
    validate_greedy_hop_mutex(&hop_hybrid, &hop_greedy)?;
    validate_hybrid_declared_split_for_no_belief(funded.economic, &hop_hybrid)?;
    validate_hybrid_hop_execute_slippage_floor(&hop_hybrid, hop_min_return)?;
    validate_greedy_hop_execute_slippage_floor(&hop_greedy, hop_min_return)?;

    let swap_msg = pair::Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(state.max_spread),
        min_return: hop_min_return,
        to: None,
        deadline: None,
        trader: Some(state.sender.to_string()),
        hybrid: hop_hybrid,
        greedy: hop_greedy,
    };

    let send_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: current_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: pair_addr.to_string(),
                amount: funded.economic,
                msg: to_json_binary(&swap_msg)?,
            })?,
            funds: vec![],
        }),
        SWAP_REPLY_ID,
    );

    Ok(Response::new()
        .add_submessage(send_msg)
        .add_attribute("action", "swap_hop")
        .add_attribute("next_pair", pair_addr)
        .add_attribute("amount", hop_output))
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::RouteTaxContext {} => to_json_binary(&SWAP_STATE.load(deps.storage)?.tax_context),
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
            trader,
            sender,
        } => to_json_binary(&query_simulate_swap_operations(
            deps,
            &env.contract.address,
            offer_amount,
            operations,
            trader,
            sender,
        )?),
        QueryMsg::ReverseSimulateSwapOperations {
            ask_amount,
            operations,
            trader,
            sender,
        } => to_json_binary(&query_reverse_simulate_swap_operations(
            deps,
            &env.contract.address,
            ask_amount,
            operations,
            trader,
            sender,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let factory = FACTORY.load(deps.storage)?;
    let wrap_mapper = WRAP_MAPPER.may_load(deps.storage)?;
    Ok(ConfigResponse {
        funding_version: dex_common::route_tax::FUNDING_VERSION,
        factory,
        wrap_mapper,
    })
}

/// Router swaps always use `belief_price: None`; mirror pair declared-split guard (#307).
fn validate_hybrid_declared_split_for_no_belief(
    hop_offer: Uint128,
    hybrid: &Option<pair::HybridSwapParams>,
) -> Result<(), ContractError> {
    if let Some(h) = hybrid {
        let leg_sum = h
            .pool_input
            .checked_add(h.book_input)
            .map_err(|e| ContractError::Std(cosmwasm_std::StdError::from(e)))?;
        if leg_sum != hop_offer {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "hybrid pool_input + book_input must equal hop offer amount",
            )));
        }
        max_spread::validate_declared_hybrid_pool_leg_for_no_belief(
            hop_offer,
            h.pool_input,
            h.book_input,
        )
        .map_err(hybrid_pool_leg_router_error)?;
    }
    Ok(())
}

fn validate_hybrid_hop_execute_slippage_floor(
    hybrid: &Option<pair::HybridSwapParams>,
    min_return: Option<Uint128>,
) -> Result<(), ContractError> {
    if let Some(h) = hybrid {
        max_spread::validate_hybrid_book_requires_slippage_floor(h.book_input, None, min_return)
            .map_err(hybrid_book_slippage_floor_router_error)?;
    }
    Ok(())
}

fn validate_greedy_hop_mutex(
    hybrid: &Option<pair::HybridSwapParams>,
    greedy: &Option<pair::GreedySwapParams>,
) -> Result<(), ContractError> {
    pair::resolve_swap_hybrid_mode(hybrid.clone(), greedy.clone())
        .map_err(|reason| ContractError::Std(cosmwasm_std::StdError::generic_err(reason)))?;
    Ok(())
}

fn validate_greedy_hop_execute_slippage_floor(
    greedy: &Option<pair::GreedySwapParams>,
    min_return: Option<Uint128>,
) -> Result<(), ContractError> {
    if greedy.is_some() {
        max_spread::validate_hybrid_book_requires_slippage_floor(Uint128::one(), None, min_return)
            .map_err(hybrid_book_slippage_floor_router_error)?;
    }
    Ok(())
}

fn hybrid_pool_leg_router_error(e: CheckMaxSpreadError) -> ContractError {
    match e {
        CheckMaxSpreadError::InsufficientPoolLegForBookHybrid {
            pool_input,
            min_pool_input,
            book_input,
        } => ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
            "Hybrid swap requires a material pool leg without belief_price: pool_input \
             {pool_input} is below minimum {min_pool_input} (book_input {book_input})"
        ))),
        other => ContractError::Std(cosmwasm_std::StdError::generic_err(format!("{other:?}"))),
    }
}

fn hybrid_book_slippage_floor_router_error(e: CheckMaxSpreadError) -> ContractError {
    match e {
        CheckMaxSpreadError::BookHybridRequiresSlippageFloor { book_input } => {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Hybrid swap with book_input {book_input} requires per-hop min_return \
                 without belief_price"
            )))
        }
        other => ContractError::Std(cosmwasm_std::StdError::generic_err(format!("{other:?}"))),
    }
}

/// Simulates multi-hop output via pair `HybridSimulation` (pool-only when `hybrid` is unset).
/// When `hybrid` is set, legs must sum to the per-hop offer amount.
fn query_simulate_swap_operations(
    deps: Deps,
    router: &Addr,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
    trader: Option<String>,
    sender: Option<String>,
) -> StdResult<SimulateSwapOperationsResponse> {
    simulate_swap_operations(
        deps,
        router,
        offer_amount,
        operations,
        trader,
        sender,
        false,
    )
}

/// Reverse responses use hybrid ratio templates rather than executable splits.
/// Scale those templates to the actual funded amount for fresh sufficiency only.
fn simulate_swap_operations(
    deps: Deps,
    router: &Addr,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
    trader: Option<String>,
    sender: Option<String>,
    reverse_templates: bool,
) -> StdResult<SimulateSwapOperationsResponse> {
    if operations.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err(
            "Operations must not be empty",
        ));
    }

    if operations.len() > MAX_HOPS {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "Too many hops: {} exceeds maximum of {}",
            operations.len(),
            MAX_HOPS
        )));
    }

    let factory = FACTORY.load(deps.storage)?;
    let mut current_amount = offer_amount;

    for op in &operations {
        match op {
            SwapOperation::NativeSwap { .. } => {
                return Err(cosmwasm_std::StdError::generic_err(
                    "Native swaps are not supported",
                ));
            }
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
                hybrid,
                greedy,
                ..
            } => {
                let pair_response: dex_common::factory::PairResponse =
                    deps.querier.query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )?;

                if hybrid.is_some() && greedy.is_some() {
                    return Err(cosmwasm_std::StdError::generic_err(
                        "cannot set both hybrid and greedy on a hop",
                    ));
                }

                let gross = current_amount;
                let funded = crate::route_tax::preview(
                    deps,
                    &Addr::unchecked(offer_asset_info.assert_is_token()?),
                    &pair_response.pair.contract_addr,
                    gross,
                    dex_common::route_tax::RouteTaxKind::Sell,
                    trader.clone(),
                    Some(router),
                )?;
                current_amount = funded.economic;
                let funded_hybrid = if reverse_templates {
                    hybrid
                        .as_ref()
                        .map(|h| {
                            let total = h.pool_input.checked_add(h.book_input)?;
                            if total.is_zero() {
                                return Err(cosmwasm_std::StdError::generic_err(
                                    "hybrid template sum must be positive",
                                ));
                            }
                            let mut funded = h.clone();
                            funded.book_input = current_amount
                                .checked_multiply_ratio(h.book_input, total)
                                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
                            funded.pool_input = current_amount.checked_sub(funded.book_input)?;
                            Ok(funded)
                        })
                        .transpose()?
                } else {
                    crate::route_tax::funded_hybrid(hybrid.clone(), gross, current_amount)?
                };
                let (sim_hybrid, sim_greedy) = match (&funded_hybrid, greedy) {
                    (_, Some(g)) => {
                        if g.max_maker_fills == 0 {
                            return Err(cosmwasm_std::StdError::generic_err(
                                "max_maker_fills must be > 0 for greedy",
                            ));
                        }
                        (None, Some(g.clone()))
                    }
                    (None, None) => (Some(pair::pool_only_hybrid_params(current_amount)), None),
                    (Some(h), None) => {
                        if h.pool_input.checked_add(h.book_input)? != current_amount {
                            return Err(cosmwasm_std::StdError::generic_err(
                                "hybrid pool_input + book_input must equal simulated offer amount for this hop",
                            ));
                        }
                        validate_hybrid_declared_split_for_no_belief(
                            current_amount,
                            &Some(h.clone()),
                        )
                        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
                        (Some(h.clone()), None)
                    }
                };
                let sim: pair::HybridSimulationResponse = deps.querier.query_wasm_smart(
                    pair_response.pair.contract_addr.to_string(),
                    &pair::QueryMsg::HybridSimulation {
                        offer_asset: Asset {
                            info: offer_asset_info.clone(),
                            amount: current_amount,
                        },
                        hybrid: sim_hybrid,
                        greedy: sim_greedy,
                        trader: trader.clone(),
                        sender: sender.clone(),
                        belief_price: None,
                    },
                )?;
                current_amount = crate::route_tax::preview(
                    deps,
                    &Addr::unchecked(ask_asset_info.assert_is_token()?),
                    &pair_response.pair.contract_addr,
                    sim.return_amount,
                    dex_common::route_tax::RouteTaxKind::Buy,
                    trader.clone(),
                    Some(router),
                )?
                .net;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: current_amount,
    })
}

/// Reverse-simulates via pair `HybridReverseSimulation` (pool-only when `hybrid` is unset).
fn query_reverse_simulate_swap_operations(
    deps: Deps,
    router: &Addr,
    ask_amount: Uint128,
    operations: Vec<SwapOperation>,
    trader: Option<String>,
    sender: Option<String>,
) -> StdResult<SimulateSwapOperationsResponse> {
    if operations.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err(
            "Operations must not be empty",
        ));
    }

    if operations.len() > MAX_HOPS {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "Too many hops: {} exceeds maximum of {}",
            operations.len(),
            MAX_HOPS
        )));
    }

    let factory = FACTORY.load(deps.storage)?;
    let mut current_amount = ask_amount;
    let mut budget = crate::reverse_query::ReverseQueryBudget::default();
    // Reserve the fresh forward route's one full pair simulation per hop before
    // any search. Reserved cost is consumed even if a subsequent query fails.
    for hop in 0..operations.len() {
        budget.charge(hop, 1)?;
    }

    for (hop, op) in operations.iter().enumerate().rev() {
        match op {
            SwapOperation::NativeSwap { .. } => {
                return Err(cosmwasm_std::StdError::generic_err(
                    "Native swaps are not supported",
                ));
            }
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
                hybrid,
                greedy,
                ..
            } => {
                let pair_response: dex_common::factory::PairResponse =
                    deps.querier.query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )?;

                current_amount = crate::route_tax::reverse_buy(
                    deps,
                    &Addr::unchecked(ask_asset_info.assert_is_token()?),
                    &pair_response.pair.contract_addr,
                    current_amount,
                    trader.clone(),
                    router,
                )?;
                if hybrid.is_some() && greedy.is_some() {
                    return Err(cosmwasm_std::StdError::generic_err(
                        "cannot set both hybrid and greedy on a hop",
                    ));
                }
                let pool_reverse_cost = crate::reverse_query::pool_reverse_cost(
                    deps,
                    &pair_response.pair.contract_addr,
                );
                if let Some(greedy) = greedy {
                    let economic = crate::route_tax::reverse_greedy_offer(
                        deps,
                        crate::route_tax::GreedyReverseRequest {
                            pair_addr: &pair_response.pair.contract_addr,
                            offer_info: offer_asset_info,
                            ask_info: ask_asset_info,
                            target: current_amount,
                            seed_cost: pool_reverse_cost,
                            greedy,
                            trader: trader.clone(),
                            sender: sender.clone(),
                        },
                        &mut budget,
                        hop,
                    )?;
                    current_amount = crate::route_tax::reverse_sell(
                        deps,
                        &Addr::unchecked(offer_asset_info.assert_is_token()?),
                        &pair_response.pair.contract_addr,
                        economic,
                        trader.clone(),
                        router,
                    )?;
                    continue;
                }
                let hybrid_params = match hybrid {
                    None => pair::pool_only_hybrid_template(),
                    Some(h) => h.clone(),
                };
                // Pair reverse is opaque here: reserve its complete documented
                // maximum, not merely this outer smart-query call.
                budget.charge(
                    hop,
                    if hybrid_params.book_input.is_zero() {
                        pool_reverse_cost
                    } else {
                        crate::reverse_query::MAX_PAIR_HYBRID_REVERSE_SIMULATIONS
                    },
                )?;
                let sim: pair::HybridReverseSimulationResponse = deps.querier.query_wasm_smart(
                    pair_response.pair.contract_addr.to_string(),
                    &pair::QueryMsg::HybridReverseSimulation {
                        ask_asset: Asset {
                            info: ask_asset_info.clone(),
                            amount: current_amount,
                        },
                        hybrid: hybrid_params,
                        trader: trader.clone(),
                        sender: sender.clone(),
                        belief_price: None,
                    },
                )?;
                current_amount = crate::route_tax::reverse_sell(
                    deps,
                    &Addr::unchecked(offer_asset_info.assert_is_token()?),
                    &pair_response.pair.contract_addr,
                    sim.offer_amount,
                    trader.clone(),
                    router,
                )?;
            }
        }
    }

    let forward = simulate_swap_operations(
        deps,
        router,
        current_amount,
        operations,
        trader,
        sender,
        true,
    )?;
    if forward.amount < ask_amount {
        return Err(cosmwasm_std::StdError::generic_err(
            "REVERSE_FORWARD_INSUFFICIENT",
        ));
    }
    Ok(SimulateSwapOperationsResponse {
        amount: current_amount,
    })
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)
        .map_err(ContractError::Std)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("version", CONTRACT_VERSION))
}

#[cfg(test)]
mod issue1389_tests {
    use super::*;
    use cosmwasm_std::{
        from_json, testing::mock_dependencies, ContractResult, SystemResult, WasmQuery,
    };
    use std::{cell::Cell, rc::Rc};

    use dex_common::types::AssetInfo;

    fn token(name: &str) -> AssetInfo {
        AssetInfo::Token {
            contract_addr: name.to_string(),
        }
    }
    fn operations(hops: usize, mode: u8, reverse: bool) -> Vec<SwapOperation> {
        (0..hops)
            .map(|hop| {
                let mode = if mode <= 2 {
                    mode
                } else {
                    ((u32::from(mode) - 3) / 3u32.pow(hop as u32) % 3) as u8
                };
                let (offer, ask) = if reverse {
                    (format!("token{}", hop + 1), format!("token{hop}"))
                } else {
                    (format!("token{hop}"), format!("token{}", hop + 1))
                };
                SwapOperation::TerraSwap {
                    offer_asset_info: token(&offer),
                    ask_asset_info: token(&ask),
                    hybrid: (mode == 2).then(|| pair::HybridSwapParams {
                        pool_input: Uint128::one(),
                        book_input: Uint128::one(),
                        max_maker_fills: 8,
                        book_start_hint: Some(999),
                    }),
                    greedy: (mode == 1).then(|| pair::greedy_swap_params(8, Some(999))),
                    min_return: Some(Uint128::one()),
                }
            })
            .collect()
    }
    fn response(amount: Uint128) -> pair::HybridSimulationResponse {
        pair::HybridSimulationResponse {
            return_amount: amount,
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
            pool_commission_amount: Uint128::zero(),
            book_commission_amount: Uint128::zero(),
            book_return_amount: Uint128::zero(),
            pool_return_amount: amount,
            limit_book_offer_consumed: Uint128::zero(),
            greedy_stop: None,
        }
    }
    fn run_version(
        version: &str,
        hops: usize,
        mode: u8,
        target: Uint128,
        backwards: bool,
        impossible: bool,
    ) -> (StdResult<SimulateSwapOperationsResponse>, u32, u32) {
        let mut deps = mock_dependencies();
        FACTORY
            .save(deps.as_mut().storage, &Addr::unchecked("factory"))
            .unwrap();
        let queries = Rc::new(Cell::new(0));
        let work = Rc::new(Cell::new(0));
        let (count, cost) = (queries.clone(), work.clone());
        let version = version.to_string();
        deps.querier.update_wasm(move |query| {
            if let WasmQuery::Raw { contract_addr, key } = query {
                assert_eq!(contract_addr, "pair");
                assert_eq!(key.as_slice(), b"contract_info");
                let bytes = match version.as_str() {
                    "missing" => cosmwasm_std::Binary::default(),
                    "malformed" => cosmwasm_std::Binary::from(b"not-json".as_slice()),
                    _ => to_json_binary(&cw2::ContractVersion {
                        contract: "cl8y-dex-pair".into(),
                        version: version.clone(),
                    })
                    .unwrap(),
                };
                return SystemResult::Ok(ContractResult::Ok(bytes));
            }
            let WasmQuery::Smart { contract_addr, msg } = query else {
                panic!("unexpected query")
            };
            let result = if contract_addr == "factory" {
                let dex_common::factory::QueryMsg::Pair { asset_infos } = from_json(msg).unwrap()
                else {
                    panic!("unexpected factory query")
                };
                to_json_binary(&dex_common::factory::PairResponse {
                    pair: dex_common::types::PairInfo {
                        asset_infos,
                        contract_addr: Addr::unchecked("pair"),
                        liquidity_token: Addr::unchecked("lp"),
                    },
                })
            } else if contract_addr == "pair" {
                count.set(count.get() + 1);
                match from_json::<pair::QueryMsg>(msg).unwrap() {
                    pair::QueryMsg::HybridReverseSimulation {
                        ask_asset,
                        hybrid,
                        trader,
                        sender,
                        ..
                    } => {
                        assert_eq!(trader.as_deref(), Some("trader"));
                        assert_eq!(sender.as_deref(), Some("sender"));
                        cost.set(
                            cost.get()
                                + if hybrid.book_input.is_zero()
                                    && matches!(version.as_str(), "1.19.0" | "1.20.0")
                                {
                                    1
                                } else {
                                    32
                                },
                        );
                        to_json_binary(&pair::HybridReverseSimulationResponse {
                            offer_amount: ask_asset.amount,
                            spread_amount: Uint128::zero(),
                            commission_amount: Uint128::zero(),
                            pool_commission_amount: Uint128::zero(),
                            book_commission_amount: Uint128::zero(),
                            book_return_amount: Uint128::zero(),
                            pool_return_amount: ask_asset.amount,
                        })
                    }
                    pair::QueryMsg::HybridSimulation {
                        offer_asset,
                        trader,
                        sender,
                        hybrid,
                        ..
                    } => {
                        assert_eq!(trader.as_deref(), Some("trader"));
                        assert_eq!(sender.as_deref(), Some("sender"));
                        if let Some(h) = hybrid {
                            assert_eq!(h.pool_input + h.book_input, offer_asset.amount);
                        }
                        cost.set(cost.get() + 1);
                        to_json_binary(&response(if impossible {
                            Uint128::zero()
                        } else {
                            offer_asset.amount
                        }))
                    }
                    _ => panic!("unexpected pair query"),
                }
            } else {
                match from_json::<dex_common::route_tax::TokenQueryMsg>(msg).unwrap() {
                    dex_common::route_tax::TokenQueryMsg::RouteTaxCapability {} => {
                        to_json_binary(&dex_common::route_tax::RouteTaxCapabilityResponse {
                            funding_version: 1,
                            router: Some(Addr::unchecked("router")),
                            buy_bps: 0,
                            sell_bps: 0,
                        })
                    }
                    dex_common::route_tax::TokenQueryMsg::RouteTaxPreview { amount, .. } => {
                        to_json_binary(&dex_common::route_tax::RouteTaxPreviewResponse {
                            funding_version: 1,
                            gross: amount,
                            economic: amount,
                            tax: Uint128::zero(),
                            net: amount,
                        })
                    }
                }
            };
            SystemResult::Ok(ContractResult::Ok(result.unwrap()))
        });
        let result = query_reverse_simulate_swap_operations(
            deps.as_ref(),
            &Addr::unchecked("router"),
            target,
            operations(hops, mode, backwards),
            Some("trader".into()),
            Some("sender".into()),
        );
        (result, queries.get(), work.get())
    }

    fn run(
        hops: usize,
        mode: u8,
        target: Uint128,
        backwards: bool,
        impossible: bool,
    ) -> (StdResult<SimulateSwapOperationsResponse>, u32, u32) {
        run_version("1.20.0", hops, mode, target, backwards, impossible)
    }

    #[test]
    fn issue1389_legacy_missing_malformed_and_future_pair_versions_reserve_maximum() {
        for version in ["1.18.0", "missing", "malformed", "1.21.0"] {
            let (result, calls, work) =
                run_version(version, 4, 0, Uint128::new(1_000_000), false, false);
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("REVERSE_QUERY_BUDGET_EXCEEDED"));
            assert_eq!((calls, work), (2, 64));
            let (result, calls, work) =
                run_version(version, 1, 1, Uint128::new(1_000_000), false, false);
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("REVERSE_QUERY_BUDGET_EXCEEDED"));
            assert_eq!((calls, work), (1, 32)); // seed permitted, forward denied
        }
        let (result, calls, work) =
            run_version("1.19.0", 4, 0, Uint128::new(1_000_000), false, false);
        assert!(result.is_ok());
        assert_eq!((calls, work), (8, 8));
    }

    #[test]
    fn issue1389_counts_every_pool_and_greedy_hop_and_fresh_validation() {
        for backwards in [false, true] {
            for target in [
                Uint128::zero(),
                Uint128::one(),
                Uint128::new(1_000_000),
                Uint128::new(100_000_000_000_000_000_000),
                Uint128::MAX,
            ] {
                let (result, calls, work) = run(4, 0, target, backwards, false);
                assert_eq!(result.unwrap().amount, target);
                assert_eq!((calls, work), (8, 8));
                let (result, calls, work) = run(4, 1, target, backwards, false);
                assert_eq!(result.unwrap().amount, target);
                let expected = if target.is_zero() {
                    4
                } else if target == Uint128::one() {
                    12
                } else {
                    16
                };
                assert_eq!((calls, work), (expected, expected));
            }
        }
    }

    #[test]
    fn issue1389_declared_hops_reserve_pair_internal_ceiling() {
        let (result, calls, work) = run(2, 2, Uint128::new(1_000_000), false, false);
        assert!(result.is_ok());
        assert_eq!((calls, work), (4, 66));
        let (result, calls, work) = run(4, 2, Uint128::new(1_000_000), false, false);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("REVERSE_QUERY_BUDGET_EXCEEDED"));
        assert_eq!((calls, work), (2, 64));
    }

    #[test]
    fn issue1389_all_four_hop_mode_combinations_obey_request_ceiling() {
        for mode in 3..84 {
            let (result, calls, work) = run(4, mode, Uint128::new(1_000_000), false, false);
            assert!(work <= crate::reverse_query::MAX_REVERSE_SIMULATIONS_PER_REQUEST);
            assert!(calls <= work);
            match result {
                Ok(quote) => assert_eq!(quote.amount, Uint128::new(1_000_000)),
                Err(error) => assert!(error.to_string().contains("REVERSE_QUERY_BUDGET_EXCEEDED")),
            }
        }
    }

    #[test]
    fn issue1389_bad_hops_and_failed_fresh_sufficiency_fail_closed() {
        let (result, calls, _) = run(5, 0, Uint128::one(), false, false);
        assert!(result.unwrap_err().to_string().contains("Too many hops"));
        assert_eq!(calls, 0);
        let (result, calls, _) = run(1, 0, Uint128::one(), false, true);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("REVERSE_FORWARD_INSUFFICIENT"));
        assert_eq!(calls, 2);
        let (result, calls, _) = run(1, 1, Uint128::MAX, false, true);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("REVERSE_TARGET_INFEASIBLE"));
        assert_eq!(calls, 2);
    }
}
