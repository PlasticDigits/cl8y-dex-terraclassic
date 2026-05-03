use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    CodeIdsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PairCountResponse, PairResponse,
    PairsResponse, QueryMsg,
};
use crate::state::{
    Config, CONFIG, PAIRS, PAIR_COUNT, PAIR_INDEX, PENDING_PAIR, REPLY_INSTANTIATE_PAIR,
    WHITELISTED_CODE_IDS,
};
use dex_common::pagination::calc_limit;
use dex_common::pair::{PairInstantiateMsg, MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP};
use dex_common::types::{pair_key, AssetInfo, PairInfo};

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
        ExecuteMsg::CreatePair { asset_infos } => execute_create_pair(deps, env, info, asset_infos),
        ExecuteMsg::AddWhitelistedCodeId { code_id } => {
            execute_add_whitelisted_code_id(deps, info, code_id)
        }
        ExecuteMsg::RemoveWhitelistedCodeId { code_id } => {
            execute_remove_whitelisted_code_id(deps, info, code_id)
        }
        ExecuteMsg::SetPairFee { pair, fee_bps } => execute_set_pair_fee(deps, info, pair, fee_bps),
        ExecuteMsg::SetPairHooks { pair, hooks } => execute_set_pair_hooks(deps, info, pair, hooks),
        ExecuteMsg::UpdateConfig {
            governance,
            treasury,
            default_fee_bps,
        } => execute_update_config(deps, info, governance, treasury, default_fee_bps),
        ExecuteMsg::SetDiscountRegistry { pair, registry } => {
            execute_set_discount_registry(deps, info, pair, registry)
        }
        ExecuteMsg::SetDiscountRegistryAll { registry } => {
            execute_set_discount_registry_all(deps, info, registry)
        }
        ExecuteMsg::SetPairPaused { pair, paused } => {
            execute_set_pair_paused(deps, info, pair, paused)
        }
        ExecuteMsg::SweepPair {
            pair,
            token,
            recipient,
        } => execute_sweep_pair(deps, info, pair, token, recipient),
    }
}

/// Verify that the caller is the governance address. All admin operations
/// in the factory are gated behind this check.
fn ensure_governance(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.governance {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

/// Instantiate a new Pair contract for the given CW20 token pair.
/// Both tokens must be CW20 (native rejected), both must have whitelisted
/// code IDs, and the pair must not already exist. The pair contract address
/// is captured in the `reply` handler after instantiation succeeds.
fn execute_create_pair(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let addr_a = asset_infos[0]
        .assert_is_token()
        .map_err(|_| ContractError::NativeTokenNotSupported {})?;
    let addr_b = asset_infos[1]
        .assert_is_token()
        .map_err(|_| ContractError::NativeTokenNotSupported {})?;

    let token_a_addr = deps.api.addr_validate(addr_a)?;
    let token_b_addr = deps.api.addr_validate(addr_b)?;

    if token_a_addr == token_b_addr {
        return Err(ContractError::InvalidTokens {});
    }

    let key = pair_key(&asset_infos);
    if PAIRS.has(deps.storage, &key) {
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

    let token_info_a: cw20::TokenInfoResponse = deps
        .querier
        .query_wasm_smart(token_a_addr.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})?;
    let token_info_b: cw20::TokenInfoResponse = deps
        .querier
        .query_wasm_smart(token_b_addr.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})?;

    let max_bd = MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP;
    if token_info_a.decimals > max_bd || token_info_b.decimals > max_bd {
        return Err(ContractError::PairAssetDecimalsTooHigh {
            decimals_a: token_info_a.decimals,
            decimals_b: token_info_b.decimals,
            max: max_bd,
        });
    }

    let truncate = |s: &str| -> String { s.chars().take(6).collect::<String>().to_uppercase() };
    let sym_a = truncate(&token_info_a.symbol);
    let sym_b = truncate(&token_info_b.symbol);

    let config = CONFIG.load(deps.storage)?;

    PENDING_PAIR.save(deps.storage, &asset_infos)?;

    let instantiate_msg = PairInstantiateMsg {
        asset_infos: asset_infos.clone(),
        fee_bps: config.default_fee_bps,
        treasury: config.treasury,
        factory: env.contract.address,
        lp_token_code_id: config.lp_token_code_id,
        token_symbols: Some([sym_a.clone(), sym_b.clone()]),
        governance: config.governance.to_string(),
    };

    let sub_msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: Some(config.governance.to_string()),
            code_id: config.pair_code_id,
            msg: to_json_binary(&instantiate_msg)?,
            funds: vec![],
            label: format!("{}-{} cl8ydex lp", sym_a, sym_b),
        },
        REPLY_INSTANTIATE_PAIR,
    );

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "create_pair")
        .add_attribute("pair", format!("{}-{}", asset_infos[0], asset_infos[1])))
}

/// Add a CW20 code ID to the whitelist. Governance only.
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

/// Remove a CW20 code ID from the whitelist. Governance only.
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

/// Verify that a pair contract address exists in the factory's registry.
/// Linear scan over `PAIR_INDEX` — acceptable for governance operations
/// which are infrequent, but would need indexing for high pair counts.
fn assert_pair_in_registry(
    deps: &DepsMut,
    pair_addr: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    let count = PAIR_COUNT.load(deps.storage)?;
    for idx in 0..count {
        if let Ok(info) = PAIR_INDEX.load(deps.storage, idx) {
            if info.contract_addr == *pair_addr {
                return Ok(());
            }
        }
    }
    Err(ContractError::PairNotInRegistry {
        pair: pair_addr.to_string(),
    })
}

/// Update the swap fee on a specific pair. Governance only.
/// Delegates to the pair's `UpdateFee` execute message.
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
    assert_pair_in_registry(&deps, &pair_addr)?;

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

/// Register post-swap hooks on a specific pair. Governance only.
/// Hooks are called after every swap — only register trusted contracts.
fn execute_set_pair_hooks(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    hooks: Vec<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let pair_addr = deps.api.addr_validate(&pair)?;
    assert_pair_in_registry(&deps, &pair_addr)?;

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

/// Set or clear the fee discount registry on a single pair. Governance only.
fn execute_set_discount_registry(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    registry: Option<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let pair_addr = deps.api.addr_validate(&pair)?;
    assert_pair_in_registry(&deps, &pair_addr)?;

    let wasm_msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetDiscountRegistry {
            registry: registry.clone(),
        })?,
        funds: vec![],
    };

    let registry_str = registry.unwrap_or_else(|| "none".to_string());

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "set_discount_registry")
        .add_attribute("pair", pair_addr)
        .add_attribute("registry", registry_str))
}

/// Set or clear the fee discount registry on ALL registered pairs.
/// Governance only. Iterates over `PAIR_INDEX` and sends a
/// `SetDiscountRegistry` message to each pair.
fn execute_set_discount_registry_all(
    deps: DepsMut,
    info: MessageInfo,
    registry: Option<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let count = PAIR_COUNT.load(deps.storage)?;
    let mut messages = Vec::new();

    for idx in 0..count {
        if let Ok(pair_info) = PAIR_INDEX.load(deps.storage, idx) {
            messages.push(WasmMsg::Execute {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetDiscountRegistry {
                    registry: registry.clone(),
                })?,
                funds: vec![],
            });
        }
    }

    let registry_str = registry.unwrap_or_else(|| "none".to_string());

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "set_discount_registry_all")
        .add_attribute("registry", registry_str)
        .add_attribute("pairs_updated", count.to_string()))
}

/// Emergency pause/unpause a specific pair. Governance only.
fn execute_set_pair_paused(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    paused: bool,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let pair_addr = deps.api.addr_validate(&pair)?;
    assert_pair_in_registry(&deps, &pair_addr)?;

    let wasm_msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetPaused { paused })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "set_pair_paused")
        .add_attribute("pair", pair_addr)
        .add_attribute("paused", paused.to_string()))
}

/// Recover excess tokens from a pair (donations, accidental transfers).
/// Governance only. Delegates to the pair's `Sweep` execute message.
fn execute_sweep_pair(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    token: String,
    recipient: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let pair_addr = deps.api.addr_validate(&pair)?;
    assert_pair_in_registry(&deps, &pair_addr)?;

    let wasm_msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::Sweep {
            token: token.clone(),
            recipient: recipient.clone(),
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "sweep_pair")
        .add_attribute("pair", pair_addr)
        .add_attribute("token", token)
        .add_attribute("recipient", recipient))
}

/// Update factory configuration (governance address, treasury, default fee).
/// Governance only. Fee must be ≤ 10000 bps.
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    governance: Option<String>,
    treasury: Option<String>,
    default_fee_bps: Option<u16>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let mut config = CONFIG.load(deps.storage)?;
    let old_governance = config.governance.clone();

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

    let new_governance = config.governance.clone();
    CONFIG.save(deps.storage, &config)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    if new_governance != old_governance {
        let pair_count = PAIR_COUNT.load(deps.storage)?;
        for idx in 0..pair_count {
            if let Ok(pair_info) = PAIR_INDEX.load(deps.storage, idx) {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pair_info.contract_addr.to_string(),
                    msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetLpAdmin {
                        admin: new_governance.to_string(),
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "update_config"))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_json_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_json_binary(&query_pairs(deps, start_after, limit)?)
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

fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairResponse> {
    let key = pair_key(&asset_infos);
    let pair_info = PAIRS
        .load(deps.storage, &key)
        .map_err(|_| StdError::generic_err("pair not found"))?;

    Ok(PairResponse { pair: pair_info })
}

fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let limit = calc_limit(limit);

    let start_idx: Option<u64> = if let Some(after_assets) = start_after {
        let after_key = pair_key(&after_assets);
        let mut found_idx: Option<u64> = None;
        for result in PAIR_INDEX.range(deps.storage, None, None, Order::Ascending) {
            let (idx, info) = result?;
            let info_key = pair_key(&info.asset_infos);
            if info_key == after_key {
                found_idx = Some(idx);
                break;
            }
        }
        if found_idx.is_none() {
            return Err(StdError::generic_err(
                "start_after pair not found in registry",
            ));
        }
        found_idx
    } else {
        None
    };

    let min = start_idx.map(Bound::exclusive);

    let pairs: Vec<PairInfo> = PAIR_INDEX
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .map(|r| r.map(|(_, p)| p))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(PairsResponse { pairs })
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
    let code_ids: Vec<u64> = results.into_iter().take(limit).collect();
    let next = if has_more {
        code_ids.last().copied()
    } else {
        None
    };

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

/// Handle the reply from a successful Pair contract instantiation.
/// Queries the new pair for its `PairInfo`, stores it in `PAIRS` and
/// `PAIR_INDEX`, and increments `PAIR_COUNT`.
fn reply_instantiate_pair(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr = parse_reply_contract_address(msg)?;
    let pair_addr = deps.api.addr_validate(&contract_addr)?;

    let asset_infos = PENDING_PAIR.load(deps.storage)?;
    PENDING_PAIR.remove(deps.storage);

    let pair_info_resp: PairInfo = deps
        .querier
        .query_wasm_smart(pair_addr.to_string(), &dex_common::pair::QueryMsg::Pair {})?;

    let key = pair_key(&asset_infos);
    PAIRS.save(deps.storage, &key, &pair_info_resp)?;

    let count = PAIR_COUNT.load(deps.storage)?;
    PAIR_INDEX.save(deps.storage, count, &pair_info_resp)?;
    PAIR_COUNT.save(deps.storage, &(count + 1))?;

    Ok(Response::new()
        .add_attribute("action", "reply_instantiate_pair")
        .add_attribute("pair_contract", pair_addr)
        .add_attribute("pair_index", count.to_string()))
}

/// Extract the contract address from a SubMsg reply's protobuf-encoded data.
fn parse_reply_contract_address(msg: Reply) -> StdResult<String> {
    let response = msg.result.into_result().map_err(StdError::generic_err)?;
    let data = response
        .data
        .ok_or_else(|| StdError::generic_err("no data in instantiate reply"))?;
    parse_protobuf_contract_address(data.as_slice())
}

/// Decode a protobuf string field (field 1) containing the contract address.
/// Used instead of `cw_utils::parse_reply_instantiate_data` for compatibility
/// with Terra Classic's response format.
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
