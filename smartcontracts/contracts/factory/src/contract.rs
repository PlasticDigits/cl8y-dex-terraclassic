use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response, StdError,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    CodeIdsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PairCountResponse, PairResponse,
    PairsResponse, QueryMsg,
};
use crate::state::{
    Config, CONFIG, PAIR_COUNT, PAIR_INDEX, PAIRS, PENDING_PAIR, REPLY_INSTANTIATE_PAIR,
    WHITELISTED_CODE_IDS,
};
use dex_common::pagination::calc_limit;
use dex_common::pair::PairInstantiateMsg;
use dex_common::types::PairInfo;

const CONTRACT_NAME: &str = "cl8y-dex-factory";
const CONTRACT_VERSION: &str = "1.0.0";

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let governance = deps.api.addr_validate(&msg.governance)?;
    let treasury = deps.api.addr_validate(&msg.treasury)?;

    if msg.default_fee_bps > 10000 {
        return Err(ContractError::InvalidFee {});
    }

    CONFIG.save(
        deps.storage,
        &Config {
            governance,
            treasury,
            default_fee_bps: msg.default_fee_bps,
            pair_code_id: msg.pair_code_id,
            lp_token_code_id: msg.lp_token_code_id,
        },
    )?;

    PAIR_COUNT.save(deps.storage, &0u64)?;

    for code_id in msg.whitelisted_code_ids {
        WHITELISTED_CODE_IDS.save(deps.storage, code_id, &true)?;
    }

    Ok(Response::new().add_attribute("action", "instantiate"))
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
        ExecuteMsg::CreatePair { token_a, token_b } => {
            execute_create_pair(deps, env, info, token_a, token_b)
        }
        ExecuteMsg::AddWhitelistedCodeId { code_id } => {
            execute_add_whitelisted_code_id(deps, info, code_id)
        }
        ExecuteMsg::RemoveWhitelistedCodeId { code_id } => {
            execute_remove_whitelisted_code_id(deps, info, code_id)
        }
        ExecuteMsg::SetPairFee { pair, fee_bps } => {
            execute_set_pair_fee(deps, info, pair, fee_bps)
        }
        ExecuteMsg::SetPairHooks { pair, hooks } => {
            execute_set_pair_hooks(deps, info, pair, hooks)
        }
        ExecuteMsg::UpdateConfig {
            governance,
            treasury,
            default_fee_bps,
        } => execute_update_config(deps, info, governance, treasury, default_fee_bps),
    }
}

fn ensure_governance(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.governance {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn execute_create_pair(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    token_a: String,
    token_b: String,
) -> Result<Response, ContractError> {
    let token_a_addr = deps.api.addr_validate(&token_a)?;
    let token_b_addr = deps.api.addr_validate(&token_b)?;

    if token_a_addr == token_b_addr {
        return Err(ContractError::InvalidTokens {});
    }

    let (token_a_addr, token_b_addr) = if token_a_addr < token_b_addr {
        (token_a_addr, token_b_addr)
    } else {
        (token_b_addr, token_a_addr)
    };

    if PAIRS.has(deps.storage, (&token_a_addr, &token_b_addr)) {
        return Err(ContractError::PairAlreadyExists {});
    }

    let info_a = deps
        .querier
        .query_wasm_contract_info(token_a_addr.to_string())?;
    if !WHITELISTED_CODE_IDS.has(deps.storage, info_a.code_id) {
        return Err(ContractError::CodeIdNotWhitelisted {});
    }

    let info_b = deps
        .querier
        .query_wasm_contract_info(token_b_addr.to_string())?;
    if !WHITELISTED_CODE_IDS.has(deps.storage, info_b.code_id) {
        return Err(ContractError::CodeIdNotWhitelisted {});
    }

    let config = CONFIG.load(deps.storage)?;

    PENDING_PAIR.save(deps.storage, &(token_a_addr.clone(), token_b_addr.clone()))?;

    let instantiate_msg = PairInstantiateMsg {
        token_a: token_a_addr.clone(),
        token_b: token_b_addr.clone(),
        fee_bps: config.default_fee_bps,
        treasury: config.treasury,
        factory: env.contract.address,
        lp_token_code_id: config.lp_token_code_id,
    };

    let sub_msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: None,
            code_id: config.pair_code_id,
            msg: to_json_binary(&instantiate_msg)?,
            funds: vec![],
            label: format!("cl8y-dex-pair-{}-{}", token_a_addr, token_b_addr),
        },
        REPLY_INSTANTIATE_PAIR,
    );

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "create_pair")
        .add_attribute("token_a", token_a_addr)
        .add_attribute("token_b", token_b_addr))
}

fn execute_add_whitelisted_code_id(
    deps: DepsMut,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    WHITELISTED_CODE_IDS.save(deps.storage, code_id, &true)?;
    Ok(Response::new()
        .add_attribute("action", "add_whitelisted_code_id")
        .add_attribute("code_id", code_id.to_string()))
}

fn execute_remove_whitelisted_code_id(
    deps: DepsMut,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    WHITELISTED_CODE_IDS.remove(deps.storage, code_id);
    Ok(Response::new()
        .add_attribute("action", "remove_whitelisted_code_id")
        .add_attribute("code_id", code_id.to_string()))
}

fn execute_set_pair_fee(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    fee_bps: u16,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    if fee_bps > 10000 {
        return Err(ContractError::InvalidFee {});
    }

    let pair_addr = deps.api.addr_validate(&pair)?;
    let wasm_msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::UpdateFee { fee_bps })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "set_pair_fee")
        .add_attribute("pair", pair_addr)
        .add_attribute("fee_bps", fee_bps.to_string()))
}

fn execute_set_pair_hooks(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    hooks: Vec<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let pair_addr = deps.api.addr_validate(&pair)?;
    let wasm_msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::UpdateHooks {
            hooks: hooks.clone(),
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "set_pair_hooks")
        .add_attribute("pair", pair_addr))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    governance: Option<String>,
    treasury: Option<String>,
    default_fee_bps: Option<u16>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(gov) = governance {
        config.governance = deps.api.addr_validate(&gov)?;
    }
    if let Some(trs) = treasury {
        config.treasury = deps.api.addr_validate(&trs)?;
    }
    if let Some(fee) = default_fee_bps {
        if fee > 10000 {
            return Err(ContractError::InvalidFee {});
        }
        config.default_fee_bps = fee;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetPair { token_a, token_b } => {
            to_json_binary(&query_pair(deps, token_a, token_b)?)
        }
        QueryMsg::GetAllPairs { start_after, limit } => {
            to_json_binary(&query_all_pairs(deps, start_after, limit)?)
        }
        QueryMsg::GetWhitelistedCodeIds { start_after, limit } => {
            to_json_binary(&query_whitelisted_code_ids(deps, start_after, limit)?)
        }
        QueryMsg::GetPairCount {} => to_json_binary(&query_pair_count(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let c = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        governance: c.governance,
        treasury: c.treasury,
        default_fee_bps: c.default_fee_bps,
        pair_code_id: c.pair_code_id,
        lp_token_code_id: c.lp_token_code_id,
    })
}

fn query_pair(deps: Deps, token_a: String, token_b: String) -> StdResult<PairResponse> {
    let a = deps.api.addr_validate(&token_a)?;
    let b = deps.api.addr_validate(&token_b)?;
    let (a, b) = if a < b { (a, b) } else { (b, a) };

    let pair_info = PAIRS
        .load(deps.storage, (&a, &b))
        .map_err(|_| StdError::generic_err("pair not found"))?;

    Ok(PairResponse { pair: pair_info })
}

fn query_all_pairs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let limit = calc_limit(limit);

    let start: Option<u64> = start_after
        .map(|s| {
            s.parse::<u64>()
                .map_err(|e| StdError::generic_err(format!("invalid cursor: {e}")))
        })
        .transpose()?;

    let min = start.map(Bound::exclusive);

    let results: Vec<(u64, PairInfo)> = PAIR_INDEX
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = results.len() > limit;
    let next = if has_more {
        results.get(limit - 1).map(|(idx, _)| idx.to_string())
    } else {
        None
    };

    let pairs = results
        .into_iter()
        .take(limit)
        .map(|(_, p)| p)
        .collect();

    Ok(PairsResponse { pairs, next })
}

fn query_whitelisted_code_ids(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<CodeIdsResponse> {
    let limit = calc_limit(limit);
    let min = start_after.map(Bound::exclusive);

    let results: Vec<u64> = WHITELISTED_CODE_IDS
        .keys(deps.storage, min, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = results.len() > limit;
    let next = if has_more {
        results.get(limit - 1).copied()
    } else {
        None
    };

    let code_ids = results.into_iter().take(limit).collect();

    Ok(CodeIdsResponse { code_ids, next })
}

fn query_pair_count(deps: Deps) -> StdResult<PairCountResponse> {
    let count = PAIR_COUNT.load(deps.storage)?;
    Ok(PairCountResponse { count })
}

// ---------------------------------------------------------------------------
// Reply
// ---------------------------------------------------------------------------

pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_INSTANTIATE_PAIR => reply_instantiate_pair(deps, msg),
        id => Err(ContractError::Std(StdError::generic_err(format!(
            "unknown reply id: {id}"
        )))),
    }
}

fn reply_instantiate_pair(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr = parse_reply_contract_address(msg)?;
    let pair_addr = deps.api.addr_validate(&contract_addr)?;

    let (token_a, token_b) = PENDING_PAIR.load(deps.storage)?;
    PENDING_PAIR.remove(deps.storage);

    let pair_info_resp: dex_common::pair::PairInfoResponse = deps
        .querier
        .query_wasm_smart(
            pair_addr.to_string(),
            &dex_common::pair::QueryMsg::GetPairInfo {},
        )?;
    let pair_info = pair_info_resp.pair;

    PAIRS.save(deps.storage, (&token_a, &token_b), &pair_info)?;

    let count = PAIR_COUNT.load(deps.storage)?;
    PAIR_INDEX.save(deps.storage, count, &pair_info)?;
    PAIR_COUNT.save(deps.storage, &(count + 1))?;

    Ok(Response::new()
        .add_attribute("action", "reply_instantiate_pair")
        .add_attribute("pair_contract", pair_addr)
        .add_attribute("pair_index", count.to_string()))
}

/// Extracts the contract address from a `WasmMsg::Instantiate` reply by parsing
/// the protobuf-encoded `MsgInstantiateContractResponse` in the reply data.
fn parse_reply_contract_address(msg: Reply) -> StdResult<String> {
    let response = msg.result.into_result().map_err(StdError::generic_err)?;
    let data = response
        .data
        .ok_or_else(|| StdError::generic_err("no data in instantiate reply"))?;
    parse_protobuf_contract_address(data.as_slice())
}

/// Reads field 1 (contract_address, string) from a MsgInstantiateContractResponse
/// protobuf. Field tag 0x0a = field number 1, wire type 2 (length-delimited).
fn parse_protobuf_contract_address(bytes: &[u8]) -> StdResult<String> {
    if bytes.len() < 2 || bytes[0] != 0x0a {
        return Err(StdError::generic_err(
            "invalid protobuf: expected string field 1",
        ));
    }

    let mut len: usize = 0;
    let mut shift = 0;
    let mut pos = 1;
    loop {
        if pos >= bytes.len() {
            return Err(StdError::generic_err("truncated varint in protobuf"));
        }
        let byte = bytes[pos];
        len |= ((byte & 0x7f) as usize) << shift;
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    if bytes.len() < pos + len {
        return Err(StdError::generic_err("truncated protobuf string field"));
    }

    String::from_utf8(bytes[pos..pos + len].to_vec())
        .map_err(|e| StdError::generic_err(format!("invalid utf8 in protobuf: {e}")))
}
