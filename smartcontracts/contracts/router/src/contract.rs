use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use crate::state::FACTORY;
use dex_common::pair;
use dex_common::types::Asset;

const CONTRACT_NAME: &str = "cl8y-dex-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        ExecuteMsg::ExecuteSwapOperations {
            operations: _,
            minimum_receive: _,
            to: _,
            deadline: _,
        } => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Use CW20 Send to initiate swaps through the router",
        ))),
    }
}

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
            deadline: _,
        } => execute_swap_operations(
            deps,
            env,
            sender,
            input_token,
            amount,
            operations,
            minimum_receive,
            to,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_operations(
    deps: DepsMut,
    _env: Env,
    sender: cosmwasm_std::Addr,
    input_token: cosmwasm_std::Addr,
    amount: Uint128,
    operations: Vec<SwapOperation>,
    _minimum_receive: Option<Uint128>,
    to: Option<String>,
) -> Result<Response, ContractError> {
    if operations.is_empty() {
        return Err(ContractError::EmptyOperations {});
    }

    let factory = FACTORY.load(deps.storage)?;
    let recipient = match to {
        Some(addr) => addr,
        None => sender.to_string(),
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut current_token = input_token;
    let mut current_amount = amount;

    for (i, op) in operations.iter().enumerate() {
        match op {
            SwapOperation::NativeSwap { .. } => {
                return Err(ContractError::NativeSwapNotSupported {});
            }
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
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
                            asset_infos: [
                                offer_asset_info.clone(),
                                ask_asset_info.clone(),
                            ],
                        },
                    )
                    .map_err(|_| ContractError::PairNotFound {})?;

                let pair_addr = pair_response.pair.contract_addr;

                let is_last = i == operations.len() - 1;
                let swap_to = if is_last {
                    Some(recipient.clone())
                } else {
                    None
                };

                let swap_msg = pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: swap_to,
                    deadline: None,
                };

                let send_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: current_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: pair_addr.to_string(),
                        amount: current_amount,
                        msg: to_json_binary(&swap_msg)?,
                    })?,
                    funds: vec![],
                });

                messages.push(send_msg);

                if !is_last {
                    let sim: pair::SimulationResponse = deps.querier.query_wasm_smart(
                        pair_addr.to_string(),
                        &pair::QueryMsg::Simulation {
                            offer_asset: Asset {
                                info: offer_asset_info.clone(),
                                amount: current_amount,
                            },
                        },
                    )?;
                    current_amount = sim.return_amount;
                    let ask_addr = ask_asset_info.assert_is_token()
                        .map_err(|_| ContractError::NativeTokenNotSupported {})?;
                    current_token = deps.api.addr_validate(ask_addr)?;
                }
            }
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "execute_swap_operations")
        .add_attribute("sender", sender)
        .add_attribute("recipient", recipient))
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
            deps,
            ask_amount,
            operations,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let factory = FACTORY.load(deps.storage)?;
    Ok(ConfigResponse { factory })
}

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
            } => {
                let pair_response: dex_common::factory::PairResponse = deps
                    .querier
                    .query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [
                                offer_asset_info.clone(),
                                ask_asset_info.clone(),
                            ],
                        },
                    )?;

                let sim: pair::SimulationResponse = deps.querier.query_wasm_smart(
                    pair_response.pair.contract_addr.to_string(),
                    &pair::QueryMsg::Simulation {
                        offer_asset: Asset {
                            info: offer_asset_info.clone(),
                            amount: current_amount,
                        },
                    },
                )?;

                current_amount = sim.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: current_amount,
    })
}

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
            } => {
                let pair_response: dex_common::factory::PairResponse = deps
                    .querier
                    .query_wasm_smart(
                        factory.to_string(),
                        &dex_common::factory::QueryMsg::Pair {
                            asset_infos: [
                                offer_asset_info.clone(),
                                ask_asset_info.clone(),
                            ],
                        },
                    )?;

                let sim: pair::ReverseSimulationResponse = deps.querier.query_wasm_smart(
                    pair_response.pair.contract_addr.to_string(),
                    &pair::QueryMsg::ReverseSimulation {
                        ask_asset: Asset {
                            info: ask_asset_info.clone(),
                            amount: current_amount,
                        },
                    },
                )?;

                current_amount = sim.offer_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: current_amount,
    })
}
