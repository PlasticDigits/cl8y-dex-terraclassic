use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    Uint128,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw20::Expiration;
use cw20_base::allowances::{
    deduct_allowance, execute_decrease_allowance, execute_increase_allowance, query_allowance,
};
use cw20_base::enumerable::{query_all_accounts, query_owner_allowances, query_spender_allowances};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw20_base::state::{BALANCES, TOKEN_INFO};

use crate::error::ContractError;
use crate::identity;
use crate::invoice;
use crate::msg::{
    ConfigResponse, ExecuteMsg, ExemptionsResponse, FeaturesResponse, InstantiateMsg,
    IsExemptResponse, LaunchGuardsView, LauncherOriginResponse, MigrateOriginResponse, QueryMsg,
    SinkView, Sku, INVOICE_UST1, MAX_INITIAL_EXEMPT, MAX_TAX_BPS,
};
use crate::pair_registry;
use crate::state::{
    Config, Features, LaunchGuards, CONFIG, FEATURES, LAUNCH_GUARDS, LISTED_PAIRS, MANAGER_EXEMPT,
    MIGRATE_ORIGIN, PROTOCOL_EXEMPT, SINKS,
};
use crate::tax::{self, is_protocol_exempt};

const CONTRACT_NAME: &str = "crates.io:cl8y-community-tax-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    identity::validate_identity(&msg.name, &msg.symbol, msg.decimals)?;
    validate_instantiate_caps(msg.max_buy_bps, msg.max_sell_bps, msg.max_transfer_bps)?;
    validate_bps_at_init(msg.buy_bps, msg.max_buy_bps)?;
    validate_bps_at_init(msg.sell_bps, msg.max_sell_bps)?;
    if let Some(t) = msg.transfer_bps {
        validate_bps_at_init(t, msg.max_transfer_bps)?;
    }

    let features = Features::from_skus(&msg.features);
    if features.mint_control != msg.mint.is_some() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "MintControl SKU and mint init must match",
        )));
    }
    reject_sku_payload_without_feature(&features, &msg)?;
    reject_headroom_without_variable_rates(&features, &msg)?;

    let mint = msg.mint.as_ref().map(|m| cw20::MinterResponse {
        minter: m.minter.clone(),
        cap: m.cap,
    });

    let manager_for_cw20 = deps.api.addr_validate(&msg.manager)?;
    cw20_base::contract::instantiate(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: manager_for_cw20,
            funds: vec![],
        },
        Cw20InstantiateMsg {
            name: msg.name.clone(),
            symbol: msg.symbol.clone(),
            decimals: msg.decimals,
            initial_balances: msg.initial_balances.clone(),
            mint,
            marketing: msg.marketing.clone(),
        },
    )?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let manager = deps.api.addr_validate(&msg.manager)?;
    let treasury = deps.api.addr_validate(&msg.treasury)?;
    let factory = deps.api.addr_validate(&msg.factory)?;
    let ust1 = deps.api.addr_validate(&msg.ust1)?;
    let cmm_treasury = deps.api.addr_validate(&msg.cmm_treasury)?;
    let router = msg
        .router
        .as_ref()
        .map(|r| deps.api.addr_validate(r))
        .transpose()?;
    let autolp = msg
        .autolp
        .as_ref()
        .map(|a| deps.api.addr_validate(a))
        .transpose()?;
    let launcher = msg
        .launcher
        .as_ref()
        .map(|a| deps.api.addr_validate(a))
        .transpose()?;

    CONFIG.save(
        deps.storage,
        &Config {
            manager: manager.clone(),
            treasury: treasury.clone(),
            buy_bps: msg.buy_bps,
            sell_bps: msg.sell_bps,
            transfer_bps: msg.transfer_bps.unwrap_or(0),
            max_buy_bps: msg.max_buy_bps,
            max_sell_bps: msg.max_sell_bps,
            max_transfer_bps: msg.max_transfer_bps,
            factory: factory.clone(),
            router: router.clone(),
            ust1,
            cmm_treasury,
            autolp: autolp.clone(),
            launcher,
            mint_revoked: false,
        },
    )?;
    FEATURES.save(deps.storage, &features)?;

    PROTOCOL_EXEMPT.save(deps.storage, &env.contract.address, &true)?;
    PROTOCOL_EXEMPT.save(deps.storage, &factory, &true)?;
    if let Some(r) = &router {
        PROTOCOL_EXEMPT.save(deps.storage, r, &true)?;
    }
    if let Some(a) = &autolp {
        PROTOCOL_EXEMPT.save(deps.storage, a, &true)?;
    }

    if features.split_router {
        if let Some(sinks) = msg.sinks {
            let cfg_now = CONFIG.load(deps.storage)?;
            let stored = invoice::validate_sinks(deps.branch(), &cfg_now, &sinks)?;
            SINKS.save(deps.storage, &stored)?;
        }
    }
    if features.launch_guards {
        let g = msg
            .launch_guards
            .ok_or(ContractError::LaunchGuardsRequired {})?;
        LAUNCH_GUARDS.save(
            deps.storage,
            &LaunchGuards {
                max_wallet: g.max_wallet,
                cooldown_blocks: g.cooldown_blocks,
                trading_enabled: g.trading_enabled,
            },
        )?;
    }

    if features.exemption_directory {
        if let Some(addrs) = msg.initial_exempt {
            write_initial_exempt(deps.branch(), &env.contract.address, &addrs)?;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("manager", manager)
        .add_attribute("community_token", env.contract.address)
        .add_attribute("invoice_ust1", INVOICE_UST1.to_string()))
}

pub(crate) fn validate_instantiate_caps(
    max_buy: u16,
    max_sell: u16,
    max_transfer: u16,
) -> Result<(), ContractError> {
    for (bps, cap_name) in [
        (max_buy, "max_buy"),
        (max_sell, "max_sell"),
        (max_transfer, "max_transfer"),
    ] {
        let _ = cap_name;
        if bps > MAX_TAX_BPS {
            return Err(ContractError::TaxBpsCap {
                bps,
                cap: MAX_TAX_BPS,
            });
        }
    }
    let combined = max_buy
        .saturating_add(max_sell)
        .saturating_add(max_transfer);
    if combined > MAX_TAX_BPS {
        return Err(ContractError::CombinedTaxCap {
            combined,
            cap: MAX_TAX_BPS,
        });
    }
    Ok(())
}

pub(crate) fn validate_bps_at_init(bps: u16, cap: u16) -> Result<(), ContractError> {
    if bps > cap {
        return Err(ContractError::TaxBpsCap { bps, cap });
    }
    Ok(())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, env, info, owner, amount),
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::Receive(cw20) => invoice::execute_receive(deps, env, info.sender, cw20),
        ExecuteMsg::RegisterListedPair { pair } => {
            pair_registry::register_listed_pair(deps, &env.contract.address, pair)
        }
        ExecuteMsg::BindAutolp { autolp } => execute_bind_autolp(deps, info, autolp),
    }
}

fn execute_bind_autolp(
    deps: DepsMut,
    info: MessageInfo,
    autolp: String,
) -> Result<Response, ContractError> {
    let features = FEATURES.load(deps.storage)?;
    if !features.auto_v2_lp {
        return Err(ContractError::SkuNotUnlocked {
            sku: Sku::AutoV2Lp.as_str().to_string(),
        });
    }
    let mut cfg = CONFIG.load(deps.storage)?;
    let launcher = cfg
        .launcher
        .as_ref()
        .ok_or(ContractError::Unauthorized {})?;
    if info.sender != *launcher {
        return Err(ContractError::Unauthorized {});
    }
    if cfg.autolp.is_some() {
        return Err(ContractError::AutolpAlreadyBound {});
    }
    let addr = deps.api.addr_validate(&autolp)?;
    PROTOCOL_EXEMPT.save(deps.storage, &addr, &true)?;
    cfg.autolp = Some(addr.clone());
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new()
        .add_attribute("action", "bind_autolp")
        .add_attribute("autolp", addr))
}

fn reject_sku_payload_without_feature(
    features: &Features,
    msg: &InstantiateMsg,
) -> Result<(), ContractError> {
    if msg.transfer_bps.is_some() && !features.transfer_tax {
        return Err(ContractError::SkuPayloadWithoutFeature {
            field: "transfer_bps".into(),
            sku: Sku::TransferTax.as_str().to_string(),
        });
    }
    if msg.sinks.is_some() && !features.split_router {
        return Err(ContractError::SkuPayloadWithoutFeature {
            field: "sinks".into(),
            sku: Sku::SplitRouter.as_str().to_string(),
        });
    }
    if msg.launch_guards.is_some() && !features.launch_guards {
        return Err(ContractError::SkuPayloadWithoutFeature {
            field: "launch_guards".into(),
            sku: Sku::LaunchGuards.as_str().to_string(),
        });
    }
    if msg.initial_exempt.as_ref().is_some_and(|v| !v.is_empty()) && !features.exemption_directory {
        return Err(ContractError::SkuPayloadWithoutFeature {
            field: "initial_exempt".into(),
            sku: Sku::ExemptionDirectory.as_str().to_string(),
        });
    }
    if msg.autolp.is_some() && !features.auto_v2_lp {
        return Err(ContractError::SkuPayloadWithoutFeature {
            field: "autolp".into(),
            sku: Sku::AutoV2Lp.as_str().to_string(),
        });
    }
    Ok(())
}

/// #605 M-1: without VariableRates, max_* must equal the current rate (no CLI headroom).
fn reject_headroom_without_variable_rates(
    features: &Features,
    msg: &InstantiateMsg,
) -> Result<(), ContractError> {
    if features.variable_rates {
        return Ok(());
    }
    let transfer = msg.transfer_bps.unwrap_or(0);
    for (field, max, current) in [
        ("max_buy_bps", msg.max_buy_bps, msg.buy_bps),
        ("max_sell_bps", msg.max_sell_bps, msg.sell_bps),
        ("max_transfer_bps", msg.max_transfer_bps, transfer),
    ] {
        if max != current {
            return Err(ContractError::SkuPayloadWithoutFeature {
                field: field.into(),
                sku: Sku::VariableRates.as_str().to_string(),
            });
        }
    }
    Ok(())
}

fn write_initial_exempt(
    deps: DepsMut,
    self_addr: &cosmwasm_std::Addr,
    addrs: &[String],
) -> Result<(), ContractError> {
    if addrs.len() > MAX_INITIAL_EXEMPT {
        return Err(ContractError::TooManyInitialExempt {
            max: MAX_INITIAL_EXEMPT,
        });
    }
    let cfg = CONFIG.load(deps.storage)?;
    for raw in addrs {
        let addr = deps.api.addr_validate(raw)?;
        if addr == *self_addr
            || addr == cfg.factory
            || cfg.router.as_ref() == Some(&addr)
            || cfg.autolp.as_ref() == Some(&addr)
            || PROTOCOL_EXEMPT
                .may_load(deps.storage, &addr)?
                .unwrap_or(false)
        {
            return Err(ContractError::ProtocolExemptNotAllowed {});
        }
        MANAGER_EXEMPT.save(deps.storage, &addr, &true)?;
    }
    Ok(())
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let to = deps.api.addr_validate(&recipient)?;
    let self_addr = env.contract.address.clone();
    let (_credit, _tax, _kind, resp) =
        tax::apply_transfer(deps, &env, &self_addr, &info.sender, &to, amount, None)?;
    Ok(resp)
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let to = deps.api.addr_validate(&contract)?;
    let self_addr = env.contract.address.clone();
    let sender = info.sender.clone();
    let (credit, _tax, _kind, resp) =
        tax::apply_transfer(deps, &env, &self_addr, &sender, &to, amount, Some(&msg))?;
    // Pair sell credits declared `amount`; Receive hook must match the credit.
    Ok(resp.add_message(tax::execute_send_hook(&to, &sender, credit, msg)?))
}

fn execute_transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let to = deps.api.addr_validate(&recipient)?;
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;
    let self_addr = env.contract.address.clone();
    // TransferFrom is never a swap Send — provide/limit-adjacent pulls stay 1:1 (T592-8).
    let (_credit, _tax, _kind, resp) =
        tax::apply_transfer(deps, &env, &self_addr, &owner_addr, &to, amount, None)?;
    Ok(resp.add_attribute("by", info.sender))
}

fn execute_send_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let to = deps.api.addr_validate(&contract)?;
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;
    let self_addr = env.contract.address.clone();
    let (credit, _tax, _kind, resp) =
        tax::apply_transfer(deps, &env, &self_addr, &owner_addr, &to, amount, Some(&msg))?;
    Ok(resp
        .add_attribute("by", info.sender)
        .add_message(tax::execute_send_hook(&to, &owner_addr, credit, msg)?))
}

fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    tax::burn_tokens(deps.storage, &info.sender, amount)?;
    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount))
}

fn execute_burn_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;
    tax::burn_tokens(deps.storage, &owner_addr, amount)?;
    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", owner_addr)
        .add_attribute("by", info.sender)
        .add_attribute("amount", amount))
}

fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let features = FEATURES.load(deps.storage)?;
    if !features.mint_control {
        return Err(ContractError::MintDisabled {});
    }
    let config = CONFIG.load(deps.storage)?;
    if config.mint_revoked {
        return Err(ContractError::MintRevoked {});
    }
    let rcpt = deps.api.addr_validate(&recipient)?;
    let token = TOKEN_INFO.load(deps.storage)?;
    let mint = token.mint.as_ref().ok_or(ContractError::MintDisabled {})?;
    if mint.minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(cap) = mint.cap {
        if token
            .total_supply
            .checked_add(amount)
            .map_err(ContractError::from)?
            > cap
        {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "mint cap exceeded",
            )));
        }
    }
    BALANCES.update(deps.storage, &rcpt, |bal| -> Result<_, ContractError> {
        Ok(bal.unwrap_or_default().checked_add(amount)?)
    })?;
    TOKEN_INFO.update(deps.storage, |mut t| -> Result<_, ContractError> {
        t.total_supply = t.total_supply.checked_add(amount)?;
        Ok(t)
    })?;
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", rcpt)
        .add_attribute("amount", amount))
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => {
            to_json_binary(&cw20_base::contract::query_balance(deps, address)?)
        }
        QueryMsg::TokenInfo {} => to_json_binary(&cw20_base::contract::query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&cw20_base::contract::query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_json_binary(&query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_json_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => {
            to_json_binary(&cw20_base::contract::query_marketing_info(deps)?)
        }
        QueryMsg::DownloadLogo {} => {
            to_json_binary(&cw20_base::contract::query_download_logo(deps)?)
        }
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetFeatures {} => to_json_binary(&query_features(deps)?),
        QueryMsg::GetExemptions { start_after, limit } => {
            to_json_binary(&query_exemptions(deps, start_after, limit)?)
        }
        QueryMsg::IsProtocolExempt { address } => to_json_binary(&query_is_exempt(deps, address)?),
        QueryMsg::TaxPreview {
            from,
            to,
            amount,
            send_msg,
        } => {
            let from = deps.api.addr_validate(&from)?;
            let to = deps.api.addr_validate(&to)?;
            let preview = tax::preview(
                deps,
                &env.contract.address,
                &from,
                &to,
                amount,
                send_msg.as_ref(),
            )?;
            to_json_binary(&preview)
        }
        QueryMsg::GetLauncherOrigin {} => {
            let cfg = CONFIG.load(deps.storage)?;
            to_json_binary(&LauncherOriginResponse {
                launcher: cfg.launcher,
            })
        }
        QueryMsg::GetMigrateOrigin {} => {
            let o = MIGRATE_ORIGIN.may_load(deps.storage)?;
            to_json_binary(&MigrateOriginResponse {
                source_cw2: o.as_ref().map(|x| x.source_cw2.clone()),
                source_version: o.as_ref().map(|x| x.source_version.clone()),
                source_code_id: o.as_ref().and_then(|x| x.source_code_id),
                migrated_at_height: o.as_ref().map(|x| x.migrated_at_height),
            })
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let c = CONFIG.load(deps.storage)?;
    let sinks = SINKS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        .map(|s| SinkView {
            kind: s.kind,
            addr: s.addr,
            bps: s.bps,
        })
        .collect();
    let launch_guards = LAUNCH_GUARDS
        .may_load(deps.storage)?
        .map(|g| LaunchGuardsView {
            max_wallet: g.max_wallet,
            cooldown_blocks: g.cooldown_blocks,
            trading_enabled: g.trading_enabled,
        });
    Ok(ConfigResponse {
        manager: c.manager,
        treasury: c.treasury,
        buy_bps: c.buy_bps,
        sell_bps: c.sell_bps,
        transfer_bps: c.transfer_bps,
        max_buy_bps: c.max_buy_bps,
        max_sell_bps: c.max_sell_bps,
        max_transfer_bps: c.max_transfer_bps,
        factory: c.factory,
        router: c.router,
        ust1: c.ust1,
        cmm_treasury: c.cmm_treasury,
        autolp: c.autolp,
        sinks,
        launch_guards,
        mint_revoked: c.mint_revoked,
    })
}

fn query_features(deps: Deps) -> StdResult<FeaturesResponse> {
    let f = FEATURES.load(deps.storage)?;
    Ok(FeaturesResponse {
        mint_control: f.mint_control,
        transfer_tax: f.transfer_tax,
        split_router: f.split_router,
        auto_v2_lp: f.auto_v2_lp,
        exemption_directory: f.exemption_directory,
        variable_rates: f.variable_rates,
        launch_guards: f.launch_guards,
    })
}

fn query_exemptions(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ExemptionsResponse> {
    let limit = limit.unwrap_or(30).min(100) as usize;
    let start = start_after
        .as_ref()
        .map(|s| deps.api.addr_validate(s))
        .transpose()?;
    let min = start.as_ref().map(cw_storage_plus::Bound::exclusive);
    let protocol: Vec<Addr> = PROTOCOL_EXEMPT
        .range(deps.storage, min.clone(), None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok().map(|(a, _)| a))
        .collect();
    let manager: Vec<Addr> = MANAGER_EXEMPT
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok().map(|(a, _)| a))
        .collect();
    let _ = LISTED_PAIRS;
    Ok(ExemptionsResponse { protocol, manager })
}

fn query_is_exempt(deps: Deps, address: String) -> StdResult<IsExemptResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let cfg = CONFIG.load(deps.storage)?;
    // self is protocol-exempt; we don't have env here — check map + known ids
    let protocol = is_protocol_exempt(deps.storage, &cfg.factory, &addr)
        || PROTOCOL_EXEMPT
            .may_load(deps.storage, &addr)?
            .unwrap_or(false);
    let manager = MANAGER_EXEMPT
        .may_load(deps.storage, &addr)?
        .unwrap_or(false);
    Ok(IsExemptResponse {
        address: addr,
        protocol,
        manager,
    })
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    let existing = cw2::get_contract_version(deps.storage).ok();
    if existing
        .as_ref()
        .is_some_and(|v| v.contract == CONTRACT_NAME)
    {
        if msg.adopt.is_some() {
            return Err(ContractError::AdoptNotForSameCrate {});
        }
        ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        return Ok(Response::new().add_attribute("action", "migrate"));
    }
    let adopt = msg.adopt.ok_or_else(|| ContractError::AdoptRequired {
        cw2: existing
            .as_ref()
            .map(|v| v.contract.clone())
            .unwrap_or_default(),
    })?;
    crate::adopt::execute_adopt(deps, env, adopt)
}

// Silence unused import in older rustc if Expiration is only in match arms via cw20_base.
#[allow(dead_code)]
fn _expire(_: Expiration) {}
