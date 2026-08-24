use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use dex_common::pair::{pool_only_hybrid_params, Cw20HookMsg, ExecuteMsg as PairExecute};
use dex_common::types::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::pair::require_factory_listed_tax_pair;
use crate::spread::{clamp_skim_max_spread, default_skim_max_spread};
use crate::state::{Config, CONFIG, SKIMMING};

const CONTRACT_NAME: &str = "crates.io:cl8y-community-tax-autolp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_SWAP: u64 = 1;

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
    Ok(Response::new().add_attribute("action", "instantiate"))
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
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.manager {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(p) = fields.pair {
        let (listed, quote) =
            require_factory_listed_tax_pair(deps.as_ref(), &cfg.factory, &cfg.token, &p)?;
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
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attribute("action", "update_config"))
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
    SKIMMING.save(deps.storage, &true)?;

    // Floor is config-only. Permissionless caller cannot pass None/None (M-2 / M610-3).
    let hook = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(cfg.skim_max_spread),
        min_return: cfg.skim_min_return,
        to: Some(env.contract.address.to_string()),
        deadline: None,
        trader: None,
        hybrid: Some(pool_only_hybrid_params(half)),
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
        .add_attribute("max_spread", cfg.skim_max_spread.to_string()))
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != REPLY_SWAP {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "unknown reply",
        )));
    }
    let cfg = CONFIG.load(deps.storage)?;
    // Residual **M610-8**: lock clears before provide messages. A listed pair's
    // provide hook could call SkimToLp again in this tx. Factory+token-side
    // listing (M-3) is what keeps that pair in the CL8Y set.
    SKIMMING.save(deps.storage, &false)?;

    let Some(quote) = cfg.quote_token.clone() else {
        return Ok(Response::new().add_attribute("action", "skim_reply_no_quote"));
    };
    let Some(pair) = cfg.pair.clone() else {
        return Ok(Response::new().add_attribute("action", "skim_reply_no_pair"));
    };

    let tax_left = token_balance(deps.as_ref(), &cfg.token, &env.contract.address)?;
    let quote_bal = token_balance(deps.as_ref(), &quote, &env.contract.address)?;
    if tax_left.is_zero() || quote_bal.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "skim_reply")
            .add_attribute("skipped", "missing_leg"));
    }

    let increase_tax = WasmMsg::Execute {
        contract_addr: cfg.token.to_string(),
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
                        contract_addr: cfg.token.to_string(),
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
            slippage_tolerance: None,
            receiver: Some(cfg.lp_recipient.to_string()),
            deadline: None,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(increase_tax),
            CosmosMsg::Wasm(increase_quote),
            CosmosMsg::Wasm(provide),
        ])
        .add_attribute("action", "skim_reply_provide")
        .add_attribute("token_in", tax_left)
        .add_attribute("quote_in", quote_bal))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
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
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if let Some(factory) = msg.factory {
        let mut cfg = CONFIG.load(deps.storage)?;
        cfg.factory = deps.api.addr_validate(&factory)?;
        CONFIG.save(deps.storage, &cfg)?;
    }
    Ok(Response::new().add_attribute("action", "migrate"))
}
