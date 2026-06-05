use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    CodeIdsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PairCountResponse, PairResponse,
    PairsResponse, QueryMsg,
};
use crate::state::{
    Config, BLACKLISTED_PAIRS, BLACKLISTED_TOKENS, BLACKLISTED_WALLETS, CONFIG, PAIRS,
    PAIR_ADDR_REGISTERED, PAIR_COUNT, PAIR_CREATION_BLOCK, PAIR_INDEX, PAIR_KEY_INDEX,
    PENDING_PAIR, REPLY_INSTANTIATE_PAIR, WHITELISTED_CODE_IDS,
};
use dex_common::blacklist::{BlacklistCheck, BlacklistCheckResponse};
use dex_common::pagination::calc_limit;
use dex_common::pair::{
    clamp_max_batch_rungs, PairInstantiateMsg, MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP,
};
use dex_common::types::{pair_key, AssetInfo, PairInfo};

const CONTRACT_NAME: &str = "cl8y-dex-factory";
const CONTRACT_VERSION: &str = "1.5.0";

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

    let default_limit_batch_max_rungs = clamp_max_batch_rungs(msg.default_limit_batch_max_rungs);

    CONFIG.save(
        deps.storage,
        &Config {
            governance,
            treasury,
            default_fee_bps: msg.default_fee_bps,
            pair_code_id: msg.pair_code_id,
            lp_token_code_id: msg.lp_token_code_id,
            default_limit_batch_max_rungs,
            pair_creation_fee_uluna: msg.pair_creation_fee_uluna,
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
        ExecuteMsg::SetPairCreationFee { fee_uluna } => {
            execute_set_pair_creation_fee(deps, info, fee_uluna)
        }
        ExecuteMsg::SetPairHooks { pair, hooks } => execute_set_pair_hooks(deps, info, pair, hooks),
        ExecuteMsg::UpdateConfig {
            governance,
            treasury,
            default_fee_bps,
            default_limit_batch_max_rungs,
        } => execute_update_config(
            deps,
            info,
            governance,
            treasury,
            default_fee_bps,
            default_limit_batch_max_rungs,
        ),
        ExecuteMsg::SetPairLimitBatchMax { pair, max_rungs } => {
            execute_set_pair_limit_batch_max(deps, info, pair, max_rungs)
        }
        ExecuteMsg::SetPairLimitCleanConfig {
            pair,
            min_remaining_token0,
            min_remaining_token1,
        } => execute_set_pair_limit_clean_config(
            deps,
            info,
            pair,
            min_remaining_token0,
            min_remaining_token1,
        ),
        ExecuteMsg::SetDiscountRegistry { pair, registry } => {
            execute_set_discount_registry(deps, info, pair, registry)
        }
        ExecuteMsg::SetDiscountRegistryAll { registry } => {
            execute_set_discount_registry_all(deps, info, registry)
        }
        ExecuteMsg::SetDiscountRegistryBatch {
            registry,
            start_after,
            limit,
        } => execute_set_discount_registry_batch(deps, info, registry, start_after, limit),
        ExecuteMsg::SetLpAdminAll { admin } => execute_set_lp_admin_all(deps, info, admin),
        ExecuteMsg::SetLpAdminBatch {
            admin,
            start_after,
            limit,
        } => execute_set_lp_admin_batch(deps, info, admin, start_after, limit),
        ExecuteMsg::SetPairPaused { pair, paused } => {
            execute_set_pair_paused(deps, info, pair, paused)
        }
        ExecuteMsg::SweepPair {
            pair,
            token,
            recipient,
        } => execute_sweep_pair(deps, info, pair, token, recipient),
        ExecuteMsg::BlacklistWallet { address } => execute_blacklist_wallet(deps, info, address),
        ExecuteMsg::UnblacklistWallet { address } => {
            execute_unblacklist_wallet(deps, info, address)
        }
        ExecuteMsg::BlacklistToken { token } => execute_blacklist_token(deps, info, token),
        ExecuteMsg::UnblacklistToken { token } => execute_unblacklist_token(deps, info, token),
        ExecuteMsg::BlacklistPair { pair } => execute_blacklist_pair(deps, info, pair),
        ExecuteMsg::UnblacklistPair { pair } => execute_unblacklist_pair(deps, info, pair),
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
///
/// **Per-block gate:** at most one `CreatePair` may enter the pending/instantiate
/// path per `env.block.height`. This is an explicit rate limit and defense-in-depth
/// against mistaken assumptions about concurrent pending state (see `lib.rs`).
fn execute_create_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
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

    // GitLab #276: charge the governance-set pair-creation fee (uluna) and forward it to treasury.
    // Fund handling runs regardless of the fee value so nothing can get stuck in the factory: only
    // uluna may be attached (stray denoms rejected), the attached amount must cover the fee, the
    // fee goes to treasury, and ANY excess is refunded — including the whole amount when the fee is
    // disabled (fee == 0) and uluna was attached by mistake.
    let fee = config.pair_creation_fee_uluna;
    if info.funds.iter().any(|c| c.denom != "uluna") {
        return Err(ContractError::UnexpectedPairCreationFunds {});
    }
    let paid = info
        .funds
        .iter()
        .find(|c| c.denom == "uluna")
        .map(|c| c.amount)
        .unwrap_or_default();
    if paid < fee {
        return Err(ContractError::InsufficientPairCreationFee { required: fee });
    }
    let mut fee_msgs: Vec<CosmosMsg> = vec![];
    if !fee.is_zero() {
        fee_msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.treasury.to_string(),
            amount: vec![Coin::new(fee.u128(), "uluna")],
        }));
    }
    let refund = paid - fee;
    if !refund.is_zero() {
        fee_msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin::new(refund.u128(), "uluna")],
        }));
    }

    let height = env.block.height;
    if PAIR_CREATION_BLOCK
        .may_load(deps.storage)?
        .is_some_and(|h| h == height)
    {
        return Err(ContractError::OnePairCreationPerBlock {});
    }
    PAIR_CREATION_BLOCK.save(deps.storage, &height)?;

    PENDING_PAIR.save(deps.storage, &asset_infos)?;

    let instantiate_msg = PairInstantiateMsg {
        asset_infos: asset_infos.clone(),
        fee_bps: config.default_fee_bps,
        treasury: config.treasury,
        factory: env.contract.address,
        lp_token_code_id: config.lp_token_code_id,
        token_symbols: Some([sym_a.clone(), sym_b.clone()]),
        governance: config.governance.to_string(),
        max_batch_rungs: config.default_limit_batch_max_rungs,
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
        .add_messages(fee_msgs)
        .add_submessage(sub_msg)
        .add_attribute("action", "create_pair")
        .add_attribute("pair", format!("{}-{}", asset_infos[0], asset_infos[1]))
        .add_attribute("creation_fee_uluna", fee.to_string()))
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

/// Verify that a pair contract address exists in the factory's registry (O(1) storage read).
fn assert_pair_in_registry(
    deps: &DepsMut,
    pair_addr: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    if PAIR_ADDR_REGISTERED.has(deps.storage, pair_addr.clone()) {
        return Ok(());
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

/// Set the uluna fee required to create a pair (forwarded to treasury). Governance only (GitLab #276).
fn execute_set_pair_creation_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee_uluna: Uint128,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.pair_creation_fee_uluna = fee_uluna;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_pair_creation_fee")
        .add_attribute("pair_creation_fee_uluna", fee_uluna.to_string()))
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

/// Set or clear the fee discount registry on all registered pairs in **one** transaction.
///
/// Governance only. Bounded to [`calc_limit`](dex_common::pagination::calc_limit)(`None`)
/// pairs (default **10**, hard cap **30** — same as batch). If `PAIR_COUNT` exceeds that
/// bound, returns [`ContractError::DiscountRegistryAllTooManyPairs`] so operators use
/// [`execute_set_discount_registry_batch`] (GitLab [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)).
fn execute_set_discount_registry_all(
    deps: DepsMut,
    info: MessageInfo,
    registry: Option<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let max_pairs = calc_limit(None);
    let count = PAIR_COUNT.load(deps.storage)?;
    if count > max_pairs as u64 {
        return Err(ContractError::DiscountRegistryAllTooManyPairs {
            pair_count: count,
            max: max_pairs as u32,
        });
    }

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
    let pairs_updated = messages.len();

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "set_discount_registry_all")
        .add_attribute("registry", registry_str)
        .add_attribute("pairs_updated", pairs_updated.to_string()))
}

/// Set or clear the fee discount registry on registered pairs **in bounded chunks** (`PAIR_INDEX` order).
///
/// Governance only. Scans contiguous indices upwards from `(start_after + 1)` (or `0` when
/// `start_after` is absent) and attaches at most `calc_limit(limit)` Wasm messages.
fn execute_set_discount_registry_batch(
    deps: DepsMut,
    info: MessageInfo,
    registry: Option<String>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let batch_limit = calc_limit(limit);
    let count = PAIR_COUNT.load(deps.storage)?;

    let start_idx = start_after.map_or(0u64, |s| s.saturating_add(1));
    let registry_str = registry.clone().unwrap_or_else(|| "none".to_string());

    if start_idx >= count {
        return Ok(Response::new()
            .add_attribute("action", "set_discount_registry_batch")
            .add_attribute("registry", registry_str)
            .add_attribute("pairs_updated", "0")
            .add_attribute("has_more", "false"));
    }

    let mut idx = start_idx;
    let mut messages = Vec::new();

    while idx < count && messages.len() < batch_limit {
        if let Ok(pair_info) = PAIR_INDEX.load(deps.storage, idx) {
            messages.push(WasmMsg::Execute {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetDiscountRegistry {
                    registry: registry.clone(),
                })?,
                funds: vec![],
            });
        }
        idx += 1;
    }

    let has_more = idx < count;
    let next_start_after = has_more.then_some(idx.saturating_sub(1));
    let last_scanned = idx.saturating_sub(1);
    let pairs_updated = messages.len();

    let mut resp = Response::new()
        .add_messages(messages)
        .add_attribute("action", "set_discount_registry_batch")
        .add_attribute("registry", registry_str)
        .add_attribute("pairs_updated", pairs_updated.to_string())
        .add_attribute("has_more", has_more.to_string())
        .add_attribute("scanned_through_index", last_scanned.to_string());

    if let Some(n) = next_start_after {
        resp = resp.add_attribute("next_start_after", n.to_string());
    }

    Ok(resp)
}

/// Update the LP-token CosmWasm admin on all registered pairs in ONE bounded tx (GitLab #277).
/// Governance only. Bounded to `calc_limit(None)` pairs; if `PAIR_COUNT` exceeds that, returns
/// [`ContractError::LpAdminAllTooManyPairs`] so operators use [`execute_set_lp_admin_batch`].
fn execute_set_lp_admin_all(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    deps.api.addr_validate(&admin)?;

    let max_pairs = calc_limit(None);
    let count = PAIR_COUNT.load(deps.storage)?;
    if count > max_pairs as u64 {
        return Err(ContractError::LpAdminAllTooManyPairs {
            pair_count: count,
            max: max_pairs as u32,
        });
    }

    let mut messages = Vec::new();
    for idx in 0..count {
        if let Ok(pair_info) = PAIR_INDEX.load(deps.storage, idx) {
            messages.push(WasmMsg::Execute {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetLpAdmin {
                    admin: admin.clone(),
                })?,
                funds: vec![],
            });
        }
    }

    let pairs_updated = messages.len();
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "set_lp_admin_all")
        .add_attribute("admin", admin)
        .add_attribute("pairs_updated", pairs_updated.to_string()))
}

/// Update the LP-token CosmWasm admin on registered pairs in bounded chunks (`PAIR_INDEX` order).
/// Governance only. Scans contiguous indices upwards from `(start_after + 1)` (or `0`) and attaches
/// at most `calc_limit(limit)` Wasm messages; rerun with `next_start_after` until `has_more=false`.
fn execute_set_lp_admin_batch(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    deps.api.addr_validate(&admin)?;

    let batch_limit = calc_limit(limit);
    let count = PAIR_COUNT.load(deps.storage)?;
    let start_idx = start_after.map_or(0u64, |s| s.saturating_add(1));

    if start_idx >= count {
        return Ok(Response::new()
            .add_attribute("action", "set_lp_admin_batch")
            .add_attribute("admin", admin)
            .add_attribute("pairs_updated", "0")
            .add_attribute("has_more", "false"));
    }

    let mut idx = start_idx;
    let mut messages = Vec::new();
    while idx < count && messages.len() < batch_limit {
        if let Ok(pair_info) = PAIR_INDEX.load(deps.storage, idx) {
            messages.push(WasmMsg::Execute {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_json_binary(&dex_common::pair::ExecuteMsg::SetLpAdmin {
                    admin: admin.clone(),
                })?,
                funds: vec![],
            });
        }
        idx += 1;
    }

    let has_more = idx < count;
    let next_start_after = has_more.then_some(idx.saturating_sub(1));
    let last_scanned = idx.saturating_sub(1);
    let pairs_updated = messages.len();

    let mut resp = Response::new()
        .add_messages(messages)
        .add_attribute("action", "set_lp_admin_batch")
        .add_attribute("admin", admin)
        .add_attribute("pairs_updated", pairs_updated.to_string())
        .add_attribute("has_more", has_more.to_string())
        .add_attribute("scanned_through_index", last_scanned.to_string());

    if let Some(n) = next_start_after {
        resp = resp.add_attribute("next_start_after", n.to_string());
    }

    Ok(resp)
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
fn execute_set_pair_limit_batch_max(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    max_rungs: u32,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let pair_addr = deps.api.addr_validate(&pair)?;
    let clamped = clamp_max_batch_rungs(max_rungs);
    let msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::UpdateLimitOrderConfig {
            max_batch_rungs: clamped,
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "set_pair_limit_batch_max")
        .add_attribute("pair", pair)
        .add_attribute("max_batch_rungs", clamped.to_string()))
}

fn execute_set_pair_limit_clean_config(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
    min_remaining_token0: cosmwasm_std::Uint128,
    min_remaining_token1: cosmwasm_std::Uint128,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let pair_addr = deps.api.addr_validate(&pair)?;
    let msg = WasmMsg::Execute {
        contract_addr: pair_addr.to_string(),
        msg: to_json_binary(&dex_common::pair::ExecuteMsg::UpdateLimitCleanConfig {
            min_remaining_token0,
            min_remaining_token1,
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "set_pair_limit_clean_config")
        .add_attribute("pair", pair)
        .add_attribute("min_remaining_token0", min_remaining_token0)
        .add_attribute("min_remaining_token1", min_remaining_token1))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    governance: Option<String>,
    treasury: Option<String>,
    default_fee_bps: Option<u16>,
    default_limit_batch_max_rungs: Option<u32>,
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
    if let Some(max_rungs) = default_limit_batch_max_rungs {
        config.default_limit_batch_max_rungs = clamp_max_batch_rungs(max_rungs);
    }

    CONFIG.save(deps.storage, &config)?;

    // GitLab #277: governance rotation no longer fans a SetLpAdmin to every pair in one tx
    // (unbounded). Rotate LP-token admins explicitly afterward via SetLpAdminAll / SetLpAdminBatch.
    Ok(Response::new().add_attribute("action", "update_config"))
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
        QueryMsg::BlacklistCheck(check) => to_json_binary(&query_blacklist_check(deps, check)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let c = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        governance: c.governance,
        treasury: c.treasury,
        default_fee_bps: c.default_fee_bps,
        default_limit_batch_max_rungs: c.default_limit_batch_max_rungs,
        pair_code_id: c.pair_code_id,
        lp_token_code_id: c.lp_token_code_id,
        pair_creation_fee_uluna: c.pair_creation_fee_uluna,
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
        let found_idx = PAIR_KEY_INDEX
            .may_load(deps.storage, &after_key)?
            .ok_or_else(|| StdError::generic_err("start_after pair not found in registry"))?;
        Some(found_idx)
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

fn query_blacklist_check(deps: Deps, check: BlacklistCheck) -> StdResult<BlacklistCheckResponse> {
    let wallet_blacklisted = if let Some(wallet) = check.wallet {
        let addr = deps.api.addr_validate(&wallet)?;
        BLACKLISTED_WALLETS.has(deps.storage, &addr)
    } else {
        false
    };

    let mut blacklisted_tokens = Vec::new();
    for token in check.tokens {
        let addr = deps.api.addr_validate(&token)?;
        if BLACKLISTED_TOKENS.has(deps.storage, &addr) {
            blacklisted_tokens.push(addr);
        }
    }

    let mut blacklisted_pairs = Vec::new();
    let mut pair_addrs: Vec<String> = check.pairs;
    if let Some(pair) = check.pair {
        if !pair_addrs.contains(&pair) {
            pair_addrs.push(pair);
        }
    }
    for pair in pair_addrs {
        let addr = deps.api.addr_validate(&pair)?;
        if BLACKLISTED_PAIRS.has(deps.storage, &addr) {
            blacklisted_pairs.push(addr);
        }
    }
    let pair_blacklisted = !blacklisted_pairs.is_empty();

    let blocked = wallet_blacklisted || pair_blacklisted || !blacklisted_tokens.is_empty();

    Ok(BlacklistCheckResponse {
        blocked,
        wallet_blacklisted,
        blacklisted_tokens,
        pair_blacklisted,
        blacklisted_pairs,
    })
}

fn execute_blacklist_wallet(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let wallet = deps.api.addr_validate(&address)?;
    if BLACKLISTED_WALLETS.has(deps.storage, &wallet) {
        return Err(ContractError::WalletAlreadyBlacklisted {
            address: wallet.to_string(),
        });
    }
    BLACKLISTED_WALLETS.save(deps.storage, &wallet, &true)?;
    Ok(Response::new()
        .add_attribute("action", "blacklist_wallet")
        .add_attribute("wallet", wallet))
}

fn execute_unblacklist_wallet(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let wallet = deps.api.addr_validate(&address)?;
    if !BLACKLISTED_WALLETS.has(deps.storage, &wallet) {
        return Err(ContractError::WalletNotBlacklisted {
            address: wallet.to_string(),
        });
    }
    BLACKLISTED_WALLETS.remove(deps.storage, &wallet);
    Ok(Response::new()
        .add_attribute("action", "unblacklist_wallet")
        .add_attribute("wallet", wallet))
}

fn execute_blacklist_token(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let token_addr = deps.api.addr_validate(&token)?;
    if BLACKLISTED_TOKENS.has(deps.storage, &token_addr) {
        return Err(ContractError::TokenAlreadyBlacklisted {
            token: token_addr.to_string(),
        });
    }
    BLACKLISTED_TOKENS.save(deps.storage, &token_addr, &true)?;
    Ok(Response::new()
        .add_attribute("action", "blacklist_token")
        .add_attribute("token", token_addr))
}

fn execute_unblacklist_token(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let token_addr = deps.api.addr_validate(&token)?;
    if !BLACKLISTED_TOKENS.has(deps.storage, &token_addr) {
        return Err(ContractError::TokenNotBlacklisted {
            token: token_addr.to_string(),
        });
    }
    BLACKLISTED_TOKENS.remove(deps.storage, &token_addr);
    Ok(Response::new()
        .add_attribute("action", "unblacklist_token")
        .add_attribute("token", token_addr))
}

fn execute_blacklist_pair(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let pair_addr = deps.api.addr_validate(&pair)?;
    if !PAIR_ADDR_REGISTERED.has(deps.storage, pair_addr.clone()) {
        return Err(ContractError::PairNotInRegistry {
            pair: pair_addr.to_string(),
        });
    }
    if BLACKLISTED_PAIRS.has(deps.storage, &pair_addr) {
        return Err(ContractError::PairAlreadyBlacklisted {
            pair: pair_addr.to_string(),
        });
    }
    BLACKLISTED_PAIRS.save(deps.storage, &pair_addr, &true)?;
    Ok(Response::new()
        .add_attribute("action", "blacklist_pair")
        .add_attribute("pair", pair_addr))
}

fn execute_unblacklist_pair(
    deps: DepsMut,
    info: MessageInfo,
    pair: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let pair_addr = deps.api.addr_validate(&pair)?;
    if !BLACKLISTED_PAIRS.has(deps.storage, &pair_addr) {
        return Err(ContractError::PairNotBlacklisted {
            pair: pair_addr.to_string(),
        });
    }
    BLACKLISTED_PAIRS.remove(deps.storage, &pair_addr);
    Ok(Response::new()
        .add_attribute("action", "unblacklist_pair")
        .add_attribute("pair", pair_addr))
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
/// Queries the new pair for its `PairInfo`, stores it in `PAIRS`,
/// `PAIR_INDEX`, `PAIR_KEY_INDEX`, and `PAIR_ADDR_REGISTERED`, and increments `PAIR_COUNT`.
fn reply_instantiate_pair(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr = parse_reply_contract_address(msg)?;
    let pair_addr = deps.api.addr_validate(&contract_addr)?;

    let asset_infos = PENDING_PAIR.load(deps.storage)?;
    PENDING_PAIR.remove(deps.storage);

    let pair_info_resp: PairInfo = deps
        .querier
        .query_wasm_smart(pair_addr.to_string(), &dex_common::pair::QueryMsg::Pair {})?;

    if pair_info_resp.contract_addr != pair_addr {
        return Err(ContractError::Std(StdError::generic_err(
            "pair query contract_addr does not match instantiate reply address",
        )));
    }

    let key = pair_key(&asset_infos);
    PAIRS.save(deps.storage, &key, &pair_info_resp)?;

    let count = PAIR_COUNT.load(deps.storage)?;
    PAIR_INDEX.save(deps.storage, count, &pair_info_resp)?;
    PAIR_KEY_INDEX.save(deps.storage, &key, &count)?;
    PAIR_ADDR_REGISTERED.save(deps.storage, pair_info_resp.contract_addr.clone(), &true)?;
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

    let count = PAIR_COUNT.load(deps.storage)?;
    for idx in 0..count {
        let info = PAIR_INDEX.load(deps.storage, idx)?;
        let key = pair_key(&info.asset_infos);
        PAIR_KEY_INDEX.save(deps.storage, &key, &idx)?;
        PAIR_ADDR_REGISTERED.save(deps.storage, info.contract_addr, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("version", CONTRACT_VERSION))
}

#[cfg(test)]
mod pair_addr_registry_tests {
    use super::*;
    use crate::msg::MigrateMsg;
    use crate::state::{PAIR_ADDR_REGISTERED, PAIR_COUNT, PAIR_INDEX, PAIR_KEY_INDEX};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::Addr;
    use cw2::{get_contract_version, set_contract_version};
    use dex_common::types::{pair_key, AssetInfo, PairInfo};

    fn sample_pair_info(contract: &str, lp: &str) -> PairInfo {
        PairInfo {
            asset_infos: [
                AssetInfo::Token {
                    contract_addr: format!("token_a_{contract}"),
                },
                AssetInfo::Token {
                    contract_addr: format!("token_b_{contract}"),
                },
            ],
            contract_addr: Addr::unchecked(contract),
            liquidity_token: Addr::unchecked(lp),
        }
    }

    #[test]
    fn migrate_from_1_0_0_backfills_pair_addr_registered_and_pair_key_index() {
        let mut deps = mock_dependencies();
        set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "1.0.0").unwrap();

        let pair0 = sample_pair_info("pair0", "lp0");
        let pair1 = sample_pair_info("pair1", "lp1");

        PAIR_COUNT.save(deps.as_mut().storage, &2u64).unwrap();
        PAIR_INDEX.save(deps.as_mut().storage, 0, &pair0).unwrap();
        PAIR_INDEX.save(deps.as_mut().storage, 1, &pair1).unwrap();

        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();

        assert!(PAIR_ADDR_REGISTERED.has(deps.as_ref().storage, Addr::unchecked("pair0")));
        assert!(PAIR_ADDR_REGISTERED.has(deps.as_ref().storage, Addr::unchecked("pair1")));

        let key0 = pair_key(&pair0.asset_infos);
        let key1 = pair_key(&pair1.asset_infos);
        assert_eq!(
            PAIR_KEY_INDEX.load(deps.as_ref().storage, &key0).unwrap(),
            0
        );
        assert_eq!(
            PAIR_KEY_INDEX.load(deps.as_ref().storage, &key1).unwrap(),
            1
        );

        let ver = get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(ver.contract, CONTRACT_NAME);
        assert_eq!(ver.version.as_str(), CONTRACT_VERSION);
    }

    #[test]
    fn query_pairs_start_after_uses_pair_key_index() {
        let mut deps = mock_dependencies();
        set_contract_version(deps.as_mut().storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();

        let pair0 = sample_pair_info("pair0", "lp0");
        let pair1 = sample_pair_info("pair1", "lp1");
        let pair2 = sample_pair_info("pair2", "lp2");

        PAIR_COUNT.save(deps.as_mut().storage, &3u64).unwrap();
        PAIR_INDEX.save(deps.as_mut().storage, 0, &pair0).unwrap();
        PAIR_INDEX.save(deps.as_mut().storage, 1, &pair1).unwrap();
        PAIR_INDEX.save(deps.as_mut().storage, 2, &pair2).unwrap();

        PAIR_KEY_INDEX
            .save(deps.as_mut().storage, &pair_key(&pair0.asset_infos), &0u64)
            .unwrap();
        PAIR_KEY_INDEX
            .save(deps.as_mut().storage, &pair_key(&pair1.asset_infos), &1u64)
            .unwrap();
        PAIR_KEY_INDEX
            .save(deps.as_mut().storage, &pair_key(&pair2.asset_infos), &2u64)
            .unwrap();

        let page: PairsResponse =
            query_pairs(deps.as_ref(), Some(pair1.asset_infos.clone()), Some(2)).unwrap();
        assert_eq!(page.pairs.len(), 1);
        assert_eq!(page.pairs[0].contract_addr, pair2.contract_addr);

        let err = query_pairs(
            deps.as_ref(),
            Some([
                AssetInfo::Token {
                    contract_addr: "missing_a".to_string(),
                },
                AssetInfo::Token {
                    contract_addr: "missing_b".to_string(),
                },
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("start_after pair not found"));
    }
}
