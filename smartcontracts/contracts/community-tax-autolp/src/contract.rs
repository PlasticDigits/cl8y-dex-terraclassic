use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use dex_common::pair::{pool_only_hybrid_params, Cw20HookMsg, ExecuteMsg as PairExecute};
use dex_common::types::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
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
    CONFIG.save(
        deps.storage,
        &Config {
            token,
            manager,
            router: msg
                .router
                .as_ref()
                .map(|r| deps.api.addr_validate(r))
                .transpose()?,
            pair: msg
                .pair
                .as_ref()
                .map(|p| deps.api.addr_validate(p))
                .transpose()?,
            quote_token: msg
                .quote_token
                .as_ref()
                .map(|q| deps.api.addr_validate(q))
                .transpose()?,
            threshold: msg.threshold,
            lp_recipient,
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
        } => execute_update(
            deps,
            info,
            pair,
            router,
            quote_token,
            threshold,
            lp_recipient,
        ),
    }
}

fn execute_update(
    deps: DepsMut,
    info: MessageInfo,
    pair: Option<String>,
    router: Option<String>,
    quote_token: Option<String>,
    threshold: Option<Uint128>,
    lp_recipient: Option<String>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.manager {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(p) = pair {
        cfg.pair = Some(deps.api.addr_validate(&p)?);
    }
    if let Some(r) = router {
        cfg.router = Some(deps.api.addr_validate(&r)?);
    }
    if let Some(q) = quote_token {
        cfg.quote_token = Some(deps.api.addr_validate(&q)?);
    }
    if let Some(t) = threshold {
        cfg.threshold = t;
    }
    if let Some(r) = lp_recipient {
        cfg.lp_recipient = deps.api.addr_validate(&r)?;
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

    let hook = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: None,
        min_return: None,
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
        .add_attribute("half", half))
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != REPLY_SWAP {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "unknown reply",
        )));
    }
    let cfg = CONFIG.load(deps.storage)?;
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
                router: c.router,
                pair: c.pair,
                quote_token: c.quote_token,
                threshold: c.threshold,
                lp_recipient: c.lp_recipient,
                skimming,
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
