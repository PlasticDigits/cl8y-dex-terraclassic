//! Test-only LocalTerra wrapper: measure a smart query inside transaction gas.
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, to_json_vec, Binary, ContractResult, DepsMut, Empty, Env, MessageInfo,
    QueryRequest, Response, StdResult, SystemResult, WasmQuery,
};

#[cw_serde]
pub struct ExecuteMsg {
    pub contract: String,
    pub query: Binary,
}

#[entry_point]
pub fn instantiate(_: DepsMut, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let address = deps.api.addr_validate(&msg.contract)?;
    let request: QueryRequest<Empty> = WasmQuery::Smart {
        contract_addr: address.to_string(),
        msg: msg.query,
    }
    .into();
    // Capture deterministic query failures as successful measurement receipts.
    // This executes no downstream messages and changes no queried state.
    let response = match deps.querier.raw_query(&to_json_vec(&request)?) {
        SystemResult::Ok(ContractResult::Ok(data)) => Response::new()
            .add_attribute("query_status", "ok")
            .set_data(data),
        SystemResult::Ok(ContractResult::Err(error)) => Response::new()
            .add_attribute("query_status", "error")
            .add_attribute("query_error", error),
        SystemResult::Err(error) => Response::new()
            .add_attribute("query_status", "error")
            .add_attribute("query_error", error.to_string()),
    };
    Ok(response)
}
