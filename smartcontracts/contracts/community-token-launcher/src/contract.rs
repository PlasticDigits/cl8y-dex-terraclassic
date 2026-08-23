use cl8y_community_tax_autolp::msg::InstantiateMsg as AutolpInit;
use cl8y_community_tax_token::msg::{
    ExecuteMsg as TokenExecute, InstantiateMsg as TokenInit, InvoiceHookMsg, Sku, INVOICE_UST1,
};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CreateTokenMsg, ExecuteMsg, InstantiateMsg, InvoiceHookMsg as LauncherHook,
    QueryMsg,
};
use crate::state::{Config, PendingAutolp, CONFIG, PENDING_AUTOLP};

const CONTRACT_NAME: &str = "crates.io:cl8y-community-token-launcher";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_TOKEN: u64 = 1;
const REPLY_AUTOLP: u64 = 2;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            token_code_id: msg.token_code_id,
            autolp_code_id: msg.autolp_code_id,
            ust1: deps.api.addr_validate(&msg.ust1)?,
            cmm_treasury: deps.api.addr_validate(&msg.cmm_treasury)?,
            cmm_governance: deps.api.addr_validate(&msg.cmm_governance)?,
            factory: deps.api.addr_validate(&msg.factory)?,
            router: msg
                .router
                .as_ref()
                .map(|r| deps.api.addr_validate(r))
                .transpose()?,
        },
    )?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20) => execute_receive(deps, env, info, cw20),
        ExecuteMsg::CreateToken(args) => execute_create_free(deps, env, *args),
    }
}

/// **O601-3** — `features == []` cannot be paid via CW20 `Send(0)`.
fn execute_create_free(
    deps: DepsMut,
    env: Env,
    args: CreateTokenMsg,
) -> Result<Response, ContractError> {
    if !args.features.is_empty() {
        return Err(ContractError::FreeProfileOnly {});
    }
    let cfg = CONFIG.load(deps.storage)?;
    create_token(deps, env, cfg, Uint128::zero(), args)
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.ust1 {
        return Err(ContractError::InvoiceToken {});
    }
    let hook: LauncherHook = cosmwasm_std::from_json(&cw20.msg)?;
    match hook {
        LauncherHook::CreateToken(args) => create_token(deps, env, cfg, cw20.amount, *args),
        LauncherHook::EnableFeature { token, sku } => {
            enable_feature(deps, cfg, cw20.amount, token, sku)
        }
    }
}

fn create_token(
    deps: DepsMut,
    env: Env,
    cfg: Config,
    paid: Uint128,
    args: CreateTokenMsg,
) -> Result<Response, ContractError> {
    cl8y_community_tax_token::identity::validate_identity(&args.name, &args.symbol, args.decimals)?;
    reject_create_sku_payloads(&args)?;

    let paid_skus = args.features.len() as u128;
    let required = Uint128::new(INVOICE_UST1)
        .checked_mul(Uint128::new(paid_skus))
        .map_err(|e| ContractError::Std(e.into()))?;
    if paid != required {
        return Err(ContractError::InvoiceAmount {
            required: required.to_string(),
            got: paid.to_string(),
        });
    }

    let wants_autolp = args.features.iter().any(|s| matches!(s, Sku::AutoV2Lp));
    if wants_autolp && cfg.autolp_code_id.is_none() {
        return Err(ContractError::AutolpCodeNotSet {});
    }
    if wants_autolp {
        PENDING_AUTOLP.save(
            deps.storage,
            &PendingAutolp {
                token: env.contract.address.clone(), // overwritten in token reply
                manager: args.manager.clone(),
                threshold: args.autolp_threshold.unwrap_or(Uint128::new(1)),
                lp_recipient: args
                    .autolp_lp_recipient
                    .unwrap_or_else(|| args.manager.clone()),
            },
        )?;
    } else {
        PENDING_AUTOLP.remove(deps.storage);
    }

    let token_init = TokenInit {
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        decimals: args.decimals,
        initial_balances: args.initial_balances,
        marketing: None,
        manager: args.manager.clone(),
        treasury: args.treasury,
        buy_bps: args.buy_bps,
        sell_bps: args.sell_bps,
        max_buy_bps: args.max_buy_bps,
        max_sell_bps: args.max_sell_bps,
        max_transfer_bps: args.max_transfer_bps,
        factory: cfg.factory.to_string(),
        router: cfg.router.as_ref().map(|a| a.to_string()),
        ust1: cfg.ust1.to_string(),
        cmm_treasury: cfg.cmm_treasury.to_string(),
        features: args.features.clone(),
        mint: args.mint,
        transfer_bps: args.transfer_bps,
        sinks: args.sinks,
        autolp: None,
        launcher: Some(env.contract.address.to_string()),
        launch_guards: args.launch_guards,
        initial_exempt: args.initial_exempt,
    };

    let mut resp = Response::new()
        .add_message(forward_ust1(&cfg, paid)?)
        .add_attribute("action", "create_token")
        .add_attribute("manager", &args.manager)
        .add_attribute("code_id", cfg.token_code_id.to_string())
        .add_attribute("sku_count", paid_skus.to_string());

    for sku in &args.features {
        resp = resp.add_attribute("sku", sku.as_str());
    }

    let instantiate_token = WasmMsg::Instantiate {
        admin: Some(cfg.cmm_governance.to_string()),
        code_id: cfg.token_code_id,
        msg: to_json_binary(&token_init)?,
        funds: vec![],
        label: format!("community-tax-{}", args.symbol),
    };
    resp = resp
        .add_submessage(SubMsg::reply_on_success(instantiate_token, REPLY_TOKEN))
        .add_attribute("community_token", "pending");
    Ok(resp)
}

fn reject_create_sku_payloads(args: &CreateTokenMsg) -> Result<(), ContractError> {
    let has = |sku: Sku| args.features.iter().any(|s| s == &sku);
    if args.transfer_bps.is_some() && !has(Sku::TransferTax) {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::SkuPayloadWithoutFeature {
                field: "transfer_bps".into(),
                sku: Sku::TransferTax.as_str().to_string(),
            },
        ));
    }
    if args.sinks.is_some() && !has(Sku::SplitRouter) {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::SkuPayloadWithoutFeature {
                field: "sinks".into(),
                sku: Sku::SplitRouter.as_str().to_string(),
            },
        ));
    }
    if args.launch_guards.is_some() && !has(Sku::LaunchGuards) {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::SkuPayloadWithoutFeature {
                field: "launch_guards".into(),
                sku: Sku::LaunchGuards.as_str().to_string(),
            },
        ));
    }
    if args.initial_exempt.as_ref().is_some_and(|v| !v.is_empty()) && !has(Sku::ExemptionDirectory)
    {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::SkuPayloadWithoutFeature {
                field: "initial_exempt".into(),
                sku: Sku::ExemptionDirectory.as_str().to_string(),
            },
        ));
    }
    if (args.autolp_threshold.is_some() || args.autolp_lp_recipient.is_some())
        && !has(Sku::AutoV2Lp)
    {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::SkuPayloadWithoutFeature {
                field: "autolp".into(),
                sku: Sku::AutoV2Lp.as_str().to_string(),
            },
        ));
    }
    if has(Sku::LaunchGuards) && args.launch_guards.is_none() {
        return Err(ContractError::Token(
            cl8y_community_tax_token::error::ContractError::LaunchGuardsRequired {},
        ));
    }
    Ok(())
}

fn enable_feature(
    deps: DepsMut,
    cfg: Config,
    paid: Uint128,
    token: String,
    sku: Sku,
) -> Result<Response, ContractError> {
    if matches!(sku, Sku::MintControl) {
        return Err(ContractError::MintControlInstantiateOnly {});
    }
    let required = Uint128::new(INVOICE_UST1);
    if paid != required {
        return Err(ContractError::InvoiceAmount {
            required: required.to_string(),
            got: paid.to_string(),
        });
    }
    let token = deps.api.addr_validate(&token)?;
    // Forward the same UST1 to the token so EnableFeature is atomic there.
    // Launcher already received `paid`; send it onward with the hook, then the
    // token forwards to CMM. To avoid a double-forward, launcher sends to token
    // and does **not** also send to CMM.
    let send = WasmMsg::Execute {
        contract_addr: cfg.ust1.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: token.to_string(),
            amount: paid,
            msg: to_json_binary(&InvoiceHookMsg::EnableFeature { sku: sku.clone() })?,
        })?,
        funds: vec![],
    };
    let mut resp = Response::new()
        .add_message(send)
        .add_attribute("action", "enable_feature")
        .add_attribute("token", token.to_string())
        .add_attribute("sku", sku.as_str());

    if matches!(sku, Sku::AutoV2Lp) {
        let code_id = cfg
            .autolp_code_id
            .ok_or(ContractError::AutolpCodeNotSet {})?;
        let token_cfg: cl8y_community_tax_token::msg::ConfigResponse =
            deps.querier.query_wasm_smart(
                &token,
                &cl8y_community_tax_token::msg::QueryMsg::GetConfig {},
            )?;
        PENDING_AUTOLP.save(
            deps.storage,
            &PendingAutolp {
                token: token.clone(),
                manager: token_cfg.manager.to_string(),
                threshold: Uint128::new(1),
                lp_recipient: token_cfg.manager.to_string(),
            },
        )?;
        resp = resp.add_submessage(instantiate_autolp_submsg(
            &cfg,
            code_id,
            &token,
            &PendingAutolp {
                token: token.clone(),
                manager: token_cfg.manager.to_string(),
                threshold: Uint128::new(1),
                lp_recipient: token_cfg.manager.to_string(),
            },
        )?);
    }

    Ok(resp)
}

fn forward_ust1(cfg: &Config, amount: Uint128) -> StdResult<WasmMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: cfg.ust1.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: cfg.cmm_treasury.to_string(),
            amount,
        })?,
        funds: vec![],
    })
}

pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_TOKEN => reply_token(deps, msg),
        REPLY_AUTOLP => reply_autolp(deps, msg),
        _ => Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "unknown reply",
        ))),
    }
}

fn reply_token(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let parsed = cw_utils::parse_reply_instantiate_data(msg)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;
    let addr = parsed.contract_address;
    let cfg = CONFIG.load(deps.storage)?;
    let mut resp = Response::new()
        .add_attribute("action", "create_token_ready")
        .add_attribute("community_token", &addr)
        .add_attribute("code_id", cfg.token_code_id.to_string());

    if let Some(mut pending) = PENDING_AUTOLP.may_load(deps.storage)? {
        let code_id = cfg
            .autolp_code_id
            .ok_or(ContractError::AutolpCodeNotSet {})?;
        pending.token = Addr::unchecked(&addr);
        PENDING_AUTOLP.save(deps.storage, &pending)?;
        resp = resp.add_submessage(instantiate_autolp_submsg(
            &cfg,
            code_id,
            &pending.token,
            &pending,
        )?);
    }
    Ok(resp)
}

fn reply_autolp(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let parsed = cw_utils::parse_reply_instantiate_data(msg)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;
    let autolp = parsed.contract_address;
    let pending = PENDING_AUTOLP.load(deps.storage)?;
    PENDING_AUTOLP.remove(deps.storage);
    let bind = WasmMsg::Execute {
        contract_addr: pending.token.to_string(),
        msg: to_json_binary(&TokenExecute::BindAutolp {
            autolp: autolp.clone(),
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(bind)
        .add_attribute("action", "autolp_bound")
        .add_attribute("community_token", pending.token)
        .add_attribute("autolp", autolp))
}

fn instantiate_autolp_submsg(
    cfg: &Config,
    code_id: u64,
    token: &cosmwasm_std::Addr,
    pending: &PendingAutolp,
) -> Result<SubMsg, ContractError> {
    let init = AutolpInit {
        token: token.to_string(),
        manager: pending.manager.clone(),
        router: cfg.router.as_ref().map(|a| a.to_string()),
        pair: None,
        quote_token: None,
        threshold: pending.threshold,
        lp_recipient: pending.lp_recipient.clone(),
    };
    Ok(SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: Some(cfg.cmm_governance.to_string()),
            code_id,
            msg: to_json_binary(&init)?,
            funds: vec![],
            label: format!("community-autolp-{}", token),
        },
        REPLY_AUTOLP,
    ))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => {
            let c = CONFIG.load(deps.storage)?;
            to_json_binary(&ConfigResponse {
                token_code_id: c.token_code_id,
                autolp_code_id: c.autolp_code_id,
                ust1: c.ust1,
                cmm_treasury: c.cmm_treasury,
                cmm_governance: c.cmm_governance,
                factory: c.factory,
                router: c.router,
            })
        }
    }
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}

#[allow(dead_code)]
fn _token_exec(_: TokenExecute) {}
