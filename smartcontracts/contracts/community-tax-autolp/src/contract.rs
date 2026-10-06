use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use dex_common::pair::{pool_only_hybrid_params, Cw20HookMsg, ExecuteMsg as PairExecute};
use dex_common::types::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::lp_guard::{capture_policy, minimum_for, validate_policy};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::pair::{register_listed_pair_msg, require_factory_listed_tax_pair};
use crate::spread::{clamp_skim_max_spread, default_skim_max_spread};
use crate::state::{Config, PendingSkim, CONFIG, LP_POLICY, PENDING_SKIM, SKIMMING};

const CONTRACT_NAME: &str = "crates.io:cl8y-community-tax-autolp";
const CONTRACT_VERSION: &str = "1.1.0";
const REPLY_SWAP: u64 = 1;
const REPLY_PROVIDE: u64 = 2;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let token = deps.api.addr_validate(&msg.token)?;
    let manager = deps.api.addr_validate(&msg.manager)?;
    let lp_recipient = deps.api.addr_validate(&msg.lp_recipient)?;
    let factory = deps.api.addr_validate(&msg.factory)?;
    let skim_max_spread = match msg.skim_max_spread {
        Some(s) => clamp_skim_max_spread(s)?,
        None => default_skim_max_spread(),
    };
    let skim_min_return = msg.skim_min_return.filter(|m| !m.is_zero());

    let mut pair = None;
    let mut quote_token = msg
        .quote_token
        .as_ref()
        .map(|q| deps.api.addr_validate(q))
        .transpose()?;
    if let Some(p) = msg.pair.as_ref() {
        let (listed, quote) = require_factory_listed_tax_pair(deps.as_ref(), &factory, &token, p)?;
        pair = Some(listed);
        if quote.is_some() {
            quote_token = quote;
        }
    }

    let register = pair
        .as_ref()
        .map(|listed| register_listed_pair_msg(&token, listed))
        .transpose()?;

    if let Some(pair) = pair.as_ref() {
        if let Some(policy) = capture_policy(deps.as_ref(), pair, &token, quote_token.as_ref())? {
            LP_POLICY.save(deps.storage, &policy)?;
        }
    }

    CONFIG.save(
        deps.storage,
        &Config {
            token,
            manager,
            factory,
            router: msg
                .router
                .as_ref()
                .map(|r| deps.api.addr_validate(r))
                .transpose()?,
            pair,
            quote_token,
            threshold: msg.threshold,
            lp_recipient,
            skim_max_spread,
            skim_min_return,
        },
    )?;
    SKIMMING.save(deps.storage, &false)?;
    let mut resp = Response::new().add_attribute("action", "instantiate");
    if let Some(msg) = register {
        resp = resp.add_message(msg);
    }
    Ok(resp)
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SkimToLp {} => execute_skim(deps, env),
        ExecuteMsg::UpdateConfig {
            pair,
            router,
            quote_token,
            threshold,
            lp_recipient,
            skim_max_spread,
            skim_min_return,
        } => execute_update(
            deps,
            info,
            UpdateFields {
                pair,
                router,
                quote_token,
                threshold,
                lp_recipient,
                skim_max_spread,
                skim_min_return,
            },
        ),
    }
}

struct UpdateFields {
    pair: Option<String>,
    router: Option<String>,
    quote_token: Option<String>,
    threshold: Option<Uint128>,
    lp_recipient: Option<String>,
    skim_max_spread: Option<Decimal>,
    skim_min_return: Option<Uint128>,
}

fn execute_update(
    deps: DepsMut,
    info: MessageInfo,
    fields: UpdateFields,
) -> Result<Response, ContractError> {
    if SKIMMING.load(deps.storage)? {
        return Err(ContractError::Reentrancy {});
    }
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.manager {
        return Err(ContractError::Unauthorized {});
    }
    let refresh_policy = fields.pair.is_some();
    let mut register = None;
    if let Some(p) = fields.pair {
        let (listed, quote) =
            require_factory_listed_tax_pair(deps.as_ref(), &cfg.factory, &cfg.token, &p)?;
        register = Some(register_listed_pair_msg(&cfg.token, &listed)?);
        cfg.pair = Some(listed);
        if quote.is_some() {
            cfg.quote_token = quote;
        }
    }
    if let Some(r) = fields.router {
        cfg.router = Some(deps.api.addr_validate(&r)?);
    }
    if let Some(q) = fields.quote_token {
        cfg.quote_token = Some(deps.api.addr_validate(&q)?);
    }
    if let Some(t) = fields.threshold {
        cfg.threshold = t;
    }
    if let Some(r) = fields.lp_recipient {
        cfg.lp_recipient = deps.api.addr_validate(&r)?;
    }
    if let Some(s) = fields.skim_max_spread {
        cfg.skim_max_spread = clamp_skim_max_spread(s)?;
    }
    if let Some(m) = fields.skim_min_return {
        cfg.skim_min_return = if m.is_zero() { None } else { Some(m) };
    }
    if refresh_policy {
        let policy = capture_policy(
            deps.as_ref(),
            cfg.pair.as_ref().ok_or(ContractError::RouterNotSet {})?,
            &cfg.token,
            cfg.quote_token.as_ref(),
        )?;
        if let Some(policy) = policy {
            LP_POLICY.save(deps.storage, &policy)?;
        } else {
            LP_POLICY.remove(deps.storage);
        }
    }
    CONFIG.save(deps.storage, &cfg)?;
    let mut resp = Response::new().add_attribute("action", "update_config");
    if let Some(msg) = register {
        resp = resp.add_message(msg);
    }
    Ok(resp)
}

fn token_balance(
    deps: Deps,
    token: &cosmwasm_std::Addr,
    owner: &cosmwasm_std::Addr,
) -> StdResult<Uint128> {
    let bal: BalanceResponse = deps.querier.query_wasm_smart(
        token,
        &Cw20QueryMsg::Balance {
            address: owner.to_string(),
        },
    )?;
    Ok(bal.balance)
}

fn execute_skim(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    if SKIMMING.load(deps.storage)? {
        return Err(ContractError::Reentrancy {});
    }
    let cfg = CONFIG.load(deps.storage)?;
    let bal = token_balance(deps.as_ref(), &cfg.token, &env.contract.address)?;
    if bal < cfg.threshold {
        return Ok(Response::new()
            .add_attribute("action", "skim_to_lp")
            .add_attribute("skipped", "below_threshold")
            .add_attribute("balance", bal));
    }
    let pair = cfg.pair.clone().ok_or(ContractError::RouterNotSet {})?;
    let half = bal.multiply_ratio(1u128, 2u128);
    if half.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "skim_to_lp")
            .add_attribute("skipped", "zero_half"));
    }
    let quote = cfg
        .quote_token
        .as_ref()
        .ok_or(ContractError::InvalidLpPool {})?;
    let policy = LP_POLICY
        .may_load(deps.storage)?
        .ok_or(ContractError::LpPolicyMissing {})?;
    validate_policy(
        deps.as_ref(),
        &policy,
        &cfg.token,
        &pair,
        quote,
        cfg.skim_max_spread,
    )?;
    let min_return = minimum_for(
        half,
        policy.quote_reserve,
        policy.tax_reserve,
        cfg.skim_max_spread,
    )?
    .max(cfg.skim_min_return.unwrap_or_default());
    let min_lp = minimum_for(
        bal.checked_sub(half)
            .map_err(cosmwasm_std::StdError::from)?,
        policy.total_share,
        policy.tax_reserve,
        cfg.skim_max_spread,
    )?;
    let lp_before = token_balance(deps.as_ref(), &policy.lp_token, &cfg.lp_recipient)?;
    PENDING_SKIM.save(
        deps.storage,
        &PendingSkim {
            pair: pair.clone(),
            token: cfg.token.clone(),
            quote: quote.clone(),
            lp_token: policy.lp_token,
            recipient: cfg.lp_recipient.clone(),
            lp_before,
            min_lp,
            tolerance: cfg.skim_max_spread,
        },
    )?;
    SKIMMING.save(deps.storage, &true)?;

    // Floor is config-only. Permissionless caller cannot pass None/None (M-2 / M610-3).
    let hook = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(cfg.skim_max_spread),
        min_return: Some(min_return),
        to: Some(env.contract.address.to_string()),
        deadline: None,
        trader: None,
        hybrid: Some(pool_only_hybrid_params(half)),
        greedy: None,
    })?;

    let send = WasmMsg::Execute {
        contract_addr: cfg.token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: pair.to_string(),
            amount: half,
            msg: hook,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(send, REPLY_SWAP))
        .add_attribute("action", "skim_to_lp")
        .add_attribute("half", half)
        .add_attribute("min_return", min_return)
        .add_attribute("min_lp", min_lp)
        .add_attribute("max_spread", cfg.skim_max_spread.to_string()))
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !matches!(msg.id, REPLY_SWAP | REPLY_PROVIDE) {
        return Err(cosmwasm_std::StdError::generic_err("unknown reply").into());
    }
    if !SKIMMING.load(deps.storage)? {
        return Err(cosmwasm_std::StdError::generic_err("no skim in progress").into());
    }
    let pending = PENDING_SKIM.load(deps.storage)?;
    if msg.id == REPLY_PROVIDE {
        let after = token_balance(deps.as_ref(), &pending.lp_token, &pending.recipient)?;
        let minted = after
            .checked_sub(pending.lp_before)
            .map_err(|_| ContractError::InsufficientLp {})?;
        if minted < pending.min_lp {
            return Err(ContractError::InsufficientLp {});
        }
        SKIMMING.save(deps.storage, &false)?;
        PENDING_SKIM.remove(deps.storage);
        return Ok(Response::new()
            .add_attribute("action", "skim_complete")
            .add_attribute("lp_minted", minted)
            .add_attribute("min_lp", pending.min_lp));
    }
    let pair = pending.pair;
    let quote = pending.quote;
    let tax_left = token_balance(deps.as_ref(), &pending.token, &env.contract.address)?;
    let quote_bal = token_balance(deps.as_ref(), &quote, &env.contract.address)?;
    if tax_left.is_zero() || quote_bal.is_zero() {
        return Err(ContractError::MissingLiquidityLeg {});
    }

    let increase_tax = WasmMsg::Execute {
        contract_addr: pending.token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair.to_string(),
            amount: tax_left,
            expires: None,
        })?,
        funds: vec![],
    };
    let increase_quote = WasmMsg::Execute {
        contract_addr: quote.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair.to_string(),
            amount: quote_bal,
            expires: None,
        })?,
        funds: vec![],
    };
    let provide = WasmMsg::Execute {
        contract_addr: pair.to_string(),
        msg: to_json_binary(&PairExecute::ProvideLiquidity {
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: pending.token.to_string(),
                    },
                    amount: tax_left,
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: quote.to_string(),
                    },
                    amount: quote_bal,
                },
            ],
            slippage_tolerance: Some(pending.tolerance),
            receiver: Some(pending.recipient.to_string()),
            deadline: None,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(increase_tax),
            CosmosMsg::Wasm(increase_quote),
        ])
        .add_submessage(SubMsg::reply_on_success(provide, REPLY_PROVIDE))
        .add_attribute("action", "skim_reply_provide")
        .add_attribute("token_in", tax_left)
        .add_attribute("quote_in", quote_bal))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetLpPolicy {} => to_json_binary(&LP_POLICY.may_load(deps.storage)?),
        QueryMsg::GetConfig {} => {
            let c = CONFIG.load(deps.storage)?;
            let skimming = SKIMMING.load(deps.storage)?;
            to_json_binary(&ConfigResponse {
                token: c.token,
                manager: c.manager,
                factory: c.factory,
                router: c.router,
                pair: c.pair,
                quote_token: c.quote_token,
                threshold: c.threshold,
                lp_recipient: c.lp_recipient,
                skim_max_spread: c.skim_max_spread,
                skim_min_return: c.skim_min_return,
                skimming,
            })
        }
    }
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    if cw2::get_contract_version(deps.storage)?.version == CONTRACT_VERSION {
        return Err(cosmwasm_std::StdError::generic_err(
            "must migrate from an older AutoLP version",
        )
        .into());
    }
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if let Some(factory) = msg.factory {
        let mut cfg = CONFIG.load(deps.storage)?;
        cfg.factory = deps.api.addr_validate(&factory)?;
        CONFIG.save(deps.storage, &cfg)?;
    }
    Ok(Response::new().add_attribute("action", "migrate"))
}
