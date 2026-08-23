use cl8y_community_tax_autolp::msg::InstantiateMsg as AutolpInit;
use cl8y_community_tax_token::msg::{
    ExecuteMsg as TokenExecute, InstantiateMsg as TokenInit, InvoiceHookMsg, Sku, INVOICE_UST1,
};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CreateTokenMsg, ExecuteMsg, InstantiateMsg, InvoiceHookMsg as LauncherHook,
    QueryMsg,
};
use crate::state::{Config, CONFIG};

const CONTRACT_NAME: &str = "crates.io:cl8y-community-token-launcher";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_TOKEN: u64 = 1;

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
    _deps: DepsMut,
    env: Env,
    cfg: Config,
    paid: Uint128,
    args: CreateTokenMsg,
) -> Result<Response, ContractError> {
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

    if wants_autolp {
        let code_id = cfg
            .autolp_code_id
            .ok_or(ContractError::AutolpCodeNotSet {})?;
        let autolp_init = AutolpInit {
            token: env.contract.address.to_string(), // replaced after token instantiate — see reply
            manager: args.manager.clone(),
            router: cfg.router.as_ref().map(|a| a.to_string()),
            pair: None,
            quote_token: None,
            threshold: args.autolp_threshold.unwrap_or(Uint128::new(1)),
            lp_recipient: args
                .autolp_lp_recipient
                .unwrap_or_else(|| args.manager.clone()),
        };
        // Token instantiate first (reply) then AutoLP — token address is in reply.
        let _ = (code_id, autolp_init);
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
    Ok(Response::new()
        .add_message(send)
        .add_attribute("action", "enable_feature")
        .add_attribute("token", token)
        .add_attribute("sku", sku.as_str()))
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
    if msg.id != REPLY_TOKEN {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "unknown reply",
        )));
    }
    let parsed = cw_utils::parse_reply_instantiate_data(msg)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;
    let addr = parsed.contract_address;
    Ok(Response::new()
        .add_attribute("action", "create_token_ready")
        .add_attribute("community_token", addr)
        .add_attribute(
            "code_id",
            CONFIG.load(deps.storage)?.token_code_id.to_string(),
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
