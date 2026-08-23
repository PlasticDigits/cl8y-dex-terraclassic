use cosmwasm_std::{to_json_binary, Addr, DepsMut, QueryRequest, Response, WasmQuery};
use dex_common::factory::{PairResponse, QueryMsg as FactoryQuery};
use dex_common::pair::QueryMsg as PairQuery;
use dex_common::types::{AssetInfo, PairInfo};

use crate::error::ContractError;
use crate::state::{CONFIG, LISTED_PAIRS, PROTOCOL_EXEMPT};

pub fn register_listed_pair(
    deps: DepsMut,
    self_addr: &Addr,
    pair_raw: String,
) -> Result<Response, ContractError> {
    let pair = deps.api.addr_validate(&pair_raw)?;
    if LISTED_PAIRS.may_load(deps.storage, &pair)?.unwrap_or(false) {
        return Ok(Response::new()
            .add_attribute("action", "register_listed_pair")
            .add_attribute("pair", &pair)
            .add_attribute("already", "true"));
    }

    let config = CONFIG.load(deps.storage)?;
    let pair_info: PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair.to_string(),
        msg: to_json_binary(&PairQuery::Pair {})?,
    }))?;

    if pair_info.contract_addr != pair {
        return Err(ContractError::PairNotListed {});
    }
    let holds_self = pair_info.asset_infos.iter().any(|a| match a {
        AssetInfo::Token { contract_addr } => contract_addr == self_addr.as_str(),
        AssetInfo::NativeToken { .. } => false,
    });
    if !holds_self {
        return Err(ContractError::PairNotListed {});
    }

    let factory_pair: PairResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.factory.to_string(),
        msg: to_json_binary(&FactoryQuery::Pair {
            asset_infos: pair_info.asset_infos.clone(),
        })?,
    }))?;
    if factory_pair.pair.contract_addr != pair {
        return Err(ContractError::PairNotListed {});
    }

    LISTED_PAIRS.save(deps.storage, &pair, &true)?;
    PROTOCOL_EXEMPT.save(deps.storage, &pair, &true)?;

    Ok(Response::new()
        .add_attribute("action", "register_listed_pair")
        .add_attribute("pair", pair)
        .add_attribute("already", "false"))
}
