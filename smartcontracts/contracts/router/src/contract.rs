use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use crate::state::{SwapState, FACTORY, SWAP_STATE, WRAP_MAPPER};
use dex_common::pair;
use dex_common::types::Asset;
use dex_common::wrap_mapper;

const CONTRACT_NAME: &str = "cl8y-dex-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SWAP_REPLY_ID: u64 = 1;
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
        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
            deadline,
            unwrap_output,
            max_spread,
        } => {
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
#[allow(clippy::too_many_arguments)]
fn execute_swap_operations(
    deps: DepsMut,
    _env: Env,
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

    SWAP_STATE.save(
        deps.storage,
        &SwapState {
            sender: sender.clone(),
            recipient: recipient.clone(),
            remaining_operations: remaining,
            minimum_receive,
            output_token,
            unwrap_output: do_unwrap,
            max_spread,
        },
    )?;

    let first_hybrid = match &first_op {
        SwapOperation::TerraSwap { hybrid, .. } => hybrid.clone(),
        SwapOperation::NativeSwap { .. } => None,
    };

    let swap_msg = pair::Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(max_spread),
        to: None,
        deadline: None,
        trader: Some(sender.to_string()),
        hybrid: first_hybrid,
    };

    let send_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: input_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: pair_addr.to_string(),
                amount,
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
            hybrid: _,
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
        id => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            format!("unknown reply id: {id}"),
        ))),
    }
}

/// Handle the completion of each swap hop.
///
/// If operations remain: query router's balance of the intermediate token,
/// resolve the next pair, and chain another SubMsg swap.
///
/// If this was the final hop: assert `minimum_receive`, transfer output
/// to the recipient (or unwrap via wrap-mapper), and clear `SWAP_STATE`.
fn reply_swap_hop(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut state = SWAP_STATE.load(deps.storage)?;

    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        state.output_token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let current_amount = balance.balance;

    if state.remaining_operations.is_empty() {
        SWAP_STATE.remove(deps.storage);

        if let Some(min) = state.minimum_receive {
            if current_amount < min {
                return Err(ContractError::MinimumReceiveAssertion {
                    minimum: min.to_string(),
                    actual: current_amount.to_string(),
                });
            }
        }

        let output_msg = if state.unwrap_output {
            let mapper = WRAP_MAPPER
                .may_load(deps.storage)?
                .ok_or(ContractError::WrapMapperNotSet {})?;

            let unwrap_hook = wrap_mapper::Cw20HookMsg::Unwrap {
                recipient: Some(state.recipient.to_string()),
            };

            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.output_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: mapper.to_string(),
                    amount: current_amount,
                    msg: to_json_binary(&unwrap_hook)?,
                })?,
                funds: vec![],
            })
        } else {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.output_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: state.recipient.to_string(),
                    amount: current_amount,
                })?,
                funds: vec![],
            })
        };

        return Ok(Response::new()
            .add_message(output_msg)
            .add_attribute("action", "swap_complete")
            .add_attribute("output_amount", current_amount)
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

    SWAP_STATE.save(deps.storage, &state)?;

    let hop_hybrid = match &next_op {
        SwapOperation::TerraSwap { hybrid, .. } => hybrid.clone(),
        SwapOperation::NativeSwap { .. } => None,
    };

    let swap_msg = pair::Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(state.max_spread),
        to: None,
        deadline: None,
        trader: Some(state.sender.to_string()),
        hybrid: hop_hybrid,
    };

    let send_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: current_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: pair_addr.to_string(),
                amount: current_amount,
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
        .add_attribute("amount", current_amount))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => to_json_binary(&query_simulate_swap_operations(
            deps,
            offer_amount,
            operations,
        )?),
        QueryMsg::ReverseSimulateSwapOperations {
            ask_amount,
            operations,
        } => to_json_binary(&query_reverse_simulate_swap_operations(
            deps, ask_amount, operations,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let factory = FACTORY.load(deps.storage)?;
    let wrap_mapper = WRAP_MAPPER.may_load(deps.storage)?;
    Ok(ConfigResponse {
        factory,
        wrap_mapper,
    })
}

/// Simulates multi-hop output using pool-only `Simulation` or pair `HybridSimulation`.
/// When `hybrid` is set, legs must sum to the per-hop offer amount.
fn query_simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
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
            } => {
                let pair_response: dex_common::factory::PairResponse =
                    deps.querier.query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )?;

                current_amount = match hybrid {
                    None => {
                        let sim: pair::SimulationResponse = deps.querier.query_wasm_smart(
                            pair_response.pair.contract_addr.to_string(),
                            &pair::QueryMsg::Simulation {
                                offer_asset: Asset {
                                    info: offer_asset_info.clone(),
                                    amount: current_amount,
                                },
                            },
                        )?;
                        sim.return_amount
                    }
                    Some(h) => {
                        if h.pool_input.checked_add(h.book_input)? != current_amount {
                            return Err(cosmwasm_std::StdError::generic_err(
                                "hybrid pool_input + book_input must equal simulated offer amount for this hop",
                            ));
                        }
                        let sim: pair::HybridSimulationResponse = deps.querier.query_wasm_smart(
                            pair_response.pair.contract_addr.to_string(),
                            &pair::QueryMsg::HybridSimulation {
                                offer_asset: Asset {
                                    info: offer_asset_info.clone(),
                                    amount: current_amount,
                                },
                                hybrid: h.clone(),
                            },
                        )?;
                        sim.return_amount
                    }
                };
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: current_amount,
    })
}

/// Reverse-simulates using `ReverseSimulation` or pair `HybridReverseSimulation`.
fn query_reverse_simulate_swap_operations(
    deps: Deps,
    ask_amount: Uint128,
    operations: Vec<SwapOperation>,
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

    for op in operations.iter().rev() {
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
            } => {
                let pair_response: dex_common::factory::PairResponse =
                    deps.querier.query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )?;

                current_amount = match hybrid {
                    None => {
                        let sim: pair::ReverseSimulationResponse = deps.querier.query_wasm_smart(
                            pair_response.pair.contract_addr.to_string(),
                            &pair::QueryMsg::ReverseSimulation {
                                ask_asset: Asset {
                                    info: ask_asset_info.clone(),
                                    amount: current_amount,
                                },
                            },
                        )?;
                        sim.offer_amount
                    }
                    Some(h) => {
                        let sim: pair::HybridReverseSimulationResponse =
                            deps.querier.query_wasm_smart(
                                pair_response.pair.contract_addr.to_string(),
                                &pair::QueryMsg::HybridReverseSimulation {
                                    ask_asset: Asset {
                                        info: ask_asset_info.clone(),
                                        amount: current_amount,
                                    },
                                    hybrid: h.clone(),
                                },
                            )?;
                        sim.offer_amount
                    }
                };
            }
        }
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
