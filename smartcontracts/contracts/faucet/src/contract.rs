use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128,
    WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CooldownResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{Config, ALLOWED_TOKENS, CONFIG, LAST_CLAIM};

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-faucet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.drip_amount.is_zero() {
        return Err(ContractError::InvalidDripAmount {});
    }
    if msg.cooldown_seconds == 0 {
        return Err(ContractError::InvalidCooldown {});
    }
    if msg.allowed_tokens.is_empty() {
        return Err(ContractError::EmptyAllowlist {});
    }

    let admin = deps.api.addr_validate(&msg.admin)?;
    CONFIG.save(
        deps.storage,
        &Config {
            admin: admin.clone(),
            drip_amount: msg.drip_amount,
            cooldown_seconds: msg.cooldown_seconds,
            paused: false,
        },
    )?;

    for token in &msg.allowed_tokens {
        let addr = deps.api.addr_validate(token)?;
        ALLOWED_TOKENS.save(deps.storage, &addr, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", admin)
        .add_attribute("drip_amount", msg.drip_amount)
        .add_attribute("cooldown_seconds", msg.cooldown_seconds.to_string())
        .add_attribute("allowed_tokens", msg.allowed_tokens.len().to_string()))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Drip { token } => execute_drip(deps, env, info, token),
        ExecuteMsg::Pause {} => execute_pause(deps, info),
        ExecuteMsg::Unpause {} => execute_unpause(deps, info),
        ExecuteMsg::UpdateAllowedTokens { tokens } => {
            execute_update_allowed_tokens(deps, info, tokens)
        }
        ExecuteMsg::UpdateConfig {
            drip_amount,
            cooldown_seconds,
            admin,
        } => execute_update_config(deps, info, drip_amount, cooldown_seconds, admin),
    }
}

fn ensure_admin(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

/// Mint fixed drip to sender for an allowlisted token, enforcing global wallet cooldown.
fn execute_drip(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused {
        return Err(ContractError::Paused {});
    }

    let token_addr = deps.api.addr_validate(&token)?;
    if !ALLOWED_TOKENS
        .may_load(deps.storage, &token_addr)?
        .unwrap_or(false)
    {
        return Err(ContractError::TokenNotAllowed {});
    }

    if let Some(last) = LAST_CLAIM.may_load(deps.storage, &info.sender)? {
        let elapsed = env.block.time.seconds().saturating_sub(last.seconds());
        if elapsed < config.cooldown_seconds {
            return Err(ContractError::CooldownActive {
                seconds_remaining: config.cooldown_seconds - elapsed,
            });
        }
    }

    LAST_CLAIM.save(deps.storage, &info.sender, &env.block.time)?;

    let mint_msg = WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: info.sender.to_string(),
            amount: config.drip_amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(mint_msg)
        .add_attribute("action", "drip")
        .add_attribute("token", token_addr)
        .add_attribute("recipient", info.sender)
        .add_attribute("amount", config.drip_amount))
}

fn execute_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    ensure_admin(&deps, &info)?;
    CONFIG.update(deps.storage, |mut c| -> Result<_, ContractError> {
        c.paused = true;
        Ok(c)
    })?;
    Ok(Response::new().add_attribute("action", "pause"))
}

fn execute_unpause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    ensure_admin(&deps, &info)?;
    CONFIG.update(deps.storage, |mut c| -> Result<_, ContractError> {
        c.paused = false;
        Ok(c)
    })?;
    Ok(Response::new().add_attribute("action", "unpause"))
}

fn execute_update_allowed_tokens(
    deps: DepsMut,
    info: MessageInfo,
    tokens: Vec<String>,
) -> Result<Response, ContractError> {
    ensure_admin(&deps, &info)?;
    if tokens.is_empty() {
        return Err(ContractError::EmptyAllowlist {});
    }

    let keys: Vec<_> = ALLOWED_TOKENS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;
    for key in keys {
        ALLOWED_TOKENS.remove(deps.storage, &key);
    }

    for token in &tokens {
        let addr = deps.api.addr_validate(token)?;
        ALLOWED_TOKENS.save(deps.storage, &addr, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "update_allowed_tokens")
        .add_attribute("count", tokens.len().to_string()))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    drip_amount: Option<Uint128>,
    cooldown_seconds: Option<u64>,
    admin: Option<String>,
) -> Result<Response, ContractError> {
    ensure_admin(&deps, &info)?;

    let mut config = CONFIG.load(deps.storage)?;
    if let Some(amount) = drip_amount {
        if amount.is_zero() {
            return Err(ContractError::InvalidDripAmount {});
        }
        config.drip_amount = amount;
    }
    if let Some(secs) = cooldown_seconds {
        if secs == 0 {
            return Err(ContractError::InvalidCooldown {});
        }
        config.cooldown_seconds = secs;
    }
    if let Some(new_admin) = admin {
        config.admin = deps.api.addr_validate(&new_admin)?;
    }
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("drip_amount", config.drip_amount)
        .add_attribute("cooldown_seconds", config.cooldown_seconds.to_string())
        .add_attribute("admin", config.admin))
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Cooldown { address } => to_json_binary(&query_cooldown(deps, env, address)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let allowed_tokens = ALLOWED_TOKENS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ConfigResponse {
        admin: config.admin,
        drip_amount: config.drip_amount,
        cooldown_seconds: config.cooldown_seconds,
        paused: config.paused,
        allowed_tokens,
    })
}

fn query_cooldown(deps: Deps, env: Env, address: String) -> StdResult<CooldownResponse> {
    let config = CONFIG.load(deps.storage)?;
    let addr = deps.api.addr_validate(&address)?;
    let last_claim_at = LAST_CLAIM.may_load(deps.storage, &addr)?;

    let (can_claim, seconds_remaining) = match &last_claim_at {
        None => (true, 0u64),
        Some(last) => {
            let elapsed = env.block.time.seconds().saturating_sub(last.seconds());
            if elapsed >= config.cooldown_seconds {
                (true, 0u64)
            } else {
                (false, config.cooldown_seconds - elapsed)
            }
        }
    };

    Ok(CooldownResponse {
        can_claim: can_claim && !config.paused,
        seconds_remaining,
        last_claim_at,
        paused: config.paused,
    })
}

pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
