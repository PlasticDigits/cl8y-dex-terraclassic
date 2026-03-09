use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SimulateRouteResponse,
};
use crate::state::FACTORY;
use dex_common::pair::{self as pair_msg, SimulateSwapResponse};

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-router";
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
        ExecuteMsg::SwapTokens {
            route: _,
            min_output: _,
            to: _,
        } => Err(ContractError::InvalidRoute {
            reason: "Use CW20 Send to initiate swaps through the router".to_string(),
        }),
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
        Cw20HookMsg::SwapTokens {
            route,
            min_output,
            to,
        } => execute_swap(deps, env, sender, input_token, amount, route, min_output, to),
    }
}

// V1: Single-hop routing. Multi-hop will be added in a future version.
fn execute_swap(
    deps: DepsMut,
    _env: Env,
    sender: cosmwasm_std::Addr,
    input_token: cosmwasm_std::Addr,
    amount: Uint128,
    route: Vec<String>,
    min_output: Option<Uint128>,
    to: Option<String>,
) -> Result<Response, ContractError> {
    if route.is_empty() {
        return Err(ContractError::EmptyRoute {});
    }
    if route.len() > 1 {
        return Err(ContractError::MultiHopNotSupported {});
    }

    let pair_addr = deps.api.addr_validate(&route[0])?;
    let recipient = match to {
        Some(addr) => addr,
        None => sender.to_string(),
    };

    let swap_msg = pair_msg::Cw20HookMsg::Swap {
        min_output,
        to: Some(recipient.clone()),
    };

    let send_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: input_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: pair_addr.to_string(),
            amount,
            msg: to_json_binary(&swap_msg)?,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("action", "swap")
        .add_attribute("sender", sender)
        .add_attribute("pair", pair_addr)
        .add_attribute("input_token", input_token)
        .add_attribute("input_amount", amount)
        .add_attribute("recipient", recipient))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::SimulateRoute {
            route,
            offer_amount,
        } => to_json_binary(&query_simulate_route(deps, route, offer_amount)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let factory = FACTORY.load(deps.storage)?;
    Ok(ConfigResponse { factory })
}

fn query_simulate_route(
    deps: Deps,
    route: Vec<String>,
    offer_amount: Uint128,
) -> StdResult<SimulateRouteResponse> {
    if route.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err("Route must not be empty"));
    }

    let mut current_amount = offer_amount;
    let mut fee_amounts = Vec::new();

    for pair_addr_str in &route {
        let pair_addr = deps.api.addr_validate(pair_addr_str)?;

        let sim_response: SimulateSwapResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_addr.to_string(),
                msg: to_json_binary(&pair_msg::QueryMsg::SimulateSwap {
                    offer_token: pair_addr.to_string(),
                    offer_amount: current_amount,
                })?,
            }))?;

        fee_amounts.push(sim_response.fee_amount);
        current_amount = sim_response.return_amount;
    }

    Ok(SimulateRouteResponse {
        return_amount: current_amount,
        fee_amounts,
    })
}
