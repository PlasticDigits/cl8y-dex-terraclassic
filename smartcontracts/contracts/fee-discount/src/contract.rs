use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, DiscountResponse, ExecuteMsg, InstantiateMsg, IsTrustedRouterResponse,
    QueryMsg, RegistrationResponse, TierEntry, TierResponse, TiersResponse,
};
use crate::state::{Config, Tier, CONFIG, REGISTRATIONS, TIERS, TRUSTED_ROUTERS};

const CONTRACT_NAME: &str = "crates.io:cl8y-dex-fee-discount";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        governance: deps.api.addr_validate(&msg.governance)?,
        cl8y_token: deps.api.addr_validate(&msg.cl8y_token)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("governance", config.governance)
        .add_attribute("cl8y_token", config.cl8y_token))
}

pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddTier {
            tier_id,
            min_cl8y_balance,
            discount_bps,
            governance_only,
        } => execute_add_tier(deps, info, tier_id, min_cl8y_balance, discount_bps, governance_only),
        ExecuteMsg::UpdateTier {
            tier_id,
            min_cl8y_balance,
            discount_bps,
            governance_only,
        } => execute_update_tier(deps, info, tier_id, min_cl8y_balance, discount_bps, governance_only),
        ExecuteMsg::RemoveTier { tier_id } => execute_remove_tier(deps, info, tier_id),
        ExecuteMsg::Register { tier_id } => execute_register(deps, info, tier_id),
        ExecuteMsg::RegisterWallet { wallet, tier_id } => {
            execute_register_wallet(deps, info, wallet, tier_id)
        }
        ExecuteMsg::Deregister {} => execute_deregister(deps, info),
        ExecuteMsg::DeregisterWallet { wallet } => {
            execute_deregister_wallet(deps, info, wallet)
        }
        ExecuteMsg::AddTrustedRouter { router } => execute_add_trusted_router(deps, info, router),
        ExecuteMsg::RemoveTrustedRouter { router } => {
            execute_remove_trusted_router(deps, info, router)
        }
        ExecuteMsg::UpdateConfig {
            governance,
            cl8y_token,
        } => execute_update_config(deps, info, governance, cl8y_token),
    }
}

fn ensure_governance(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.governance {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn execute_add_tier(
    deps: DepsMut,
    info: MessageInfo,
    tier_id: u8,
    min_cl8y_balance: Uint128,
    discount_bps: u16,
    governance_only: bool,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    if discount_bps > 10000 {
        return Err(ContractError::InvalidDiscountBps {
            value: discount_bps,
        });
    }

    if TIERS.has(deps.storage, tier_id) {
        return Err(ContractError::TierAlreadyExists { tier_id });
    }

    let tier = Tier {
        min_cl8y_balance,
        discount_bps,
        governance_only,
    };
    TIERS.save(deps.storage, tier_id, &tier)?;

    Ok(Response::new()
        .add_attribute("action", "add_tier")
        .add_attribute("tier_id", tier_id.to_string())
        .add_attribute("min_cl8y_balance", min_cl8y_balance)
        .add_attribute("discount_bps", discount_bps.to_string())
        .add_attribute("governance_only", governance_only.to_string()))
}

fn execute_update_tier(
    deps: DepsMut,
    info: MessageInfo,
    tier_id: u8,
    min_cl8y_balance: Option<Uint128>,
    discount_bps: Option<u16>,
    governance_only: Option<bool>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    let mut tier = TIERS
        .may_load(deps.storage, tier_id)?
        .ok_or(ContractError::TierNotFound { tier_id })?;

    if let Some(balance) = min_cl8y_balance {
        tier.min_cl8y_balance = balance;
    }
    if let Some(bps) = discount_bps {
        if bps > 10000 {
            return Err(ContractError::InvalidDiscountBps { value: bps });
        }
        tier.discount_bps = bps;
    }
    if let Some(gov) = governance_only {
        tier.governance_only = gov;
    }

    TIERS.save(deps.storage, tier_id, &tier)?;

    Ok(Response::new()
        .add_attribute("action", "update_tier")
        .add_attribute("tier_id", tier_id.to_string()))
}

fn execute_remove_tier(
    deps: DepsMut,
    info: MessageInfo,
    tier_id: u8,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    if !TIERS.has(deps.storage, tier_id) {
        return Err(ContractError::TierNotFound { tier_id });
    }

    TIERS.remove(deps.storage, tier_id);

    Ok(Response::new()
        .add_attribute("action", "remove_tier")
        .add_attribute("tier_id", tier_id.to_string()))
}

fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    tier_id: u8,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // EOA check: reject smart contracts
    if deps
        .querier
        .query_wasm_contract_info(info.sender.to_string())
        .is_ok()
    {
        return Err(ContractError::ContractNotAllowed {});
    }

    let tier = TIERS
        .may_load(deps.storage, tier_id)?
        .ok_or(ContractError::TierNotFound { tier_id })?;

    if tier.governance_only {
        return Err(ContractError::GovernanceOnlyTier { tier_id });
    }

    // If already registered, check current tier rules
    if let Some(current_tier_id) = REGISTRATIONS.may_load(deps.storage, info.sender.as_str())? {
        let current_tier = TIERS.may_load(deps.storage, current_tier_id)?;
        if let Some(ct) = current_tier {
            if ct.governance_only {
                return Err(ContractError::LockedToGovernanceTier {});
            }
        }
    }

    // Check CL8Y balance
    if !tier.min_cl8y_balance.is_zero() {
        let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
            config.cl8y_token.to_string(),
            &cw20::Cw20QueryMsg::Balance {
                address: info.sender.to_string(),
            },
        )?;

        if balance.balance < tier.min_cl8y_balance {
            return Err(ContractError::InsufficientBalance {
                required: tier.min_cl8y_balance.to_string(),
                actual: balance.balance.to_string(),
            });
        }
    }

    REGISTRATIONS.save(deps.storage, info.sender.as_str(), &tier_id)?;

    Ok(Response::new()
        .add_attribute("action", "register")
        .add_attribute("wallet", info.sender)
        .add_attribute("tier_id", tier_id.to_string()))
}

fn execute_register_wallet(
    deps: DepsMut,
    info: MessageInfo,
    wallet: String,
    tier_id: u8,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;

    if !TIERS.has(deps.storage, tier_id) {
        return Err(ContractError::TierNotFound { tier_id });
    }

    let wallet_addr = deps.api.addr_validate(&wallet)?;
    REGISTRATIONS.save(deps.storage, wallet_addr.as_str(), &tier_id)?;

    Ok(Response::new()
        .add_attribute("action", "register_wallet")
        .add_attribute("wallet", wallet_addr)
        .add_attribute("tier_id", tier_id.to_string()))
}

fn execute_deregister(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let current_tier_id = REGISTRATIONS
        .may_load(deps.storage, info.sender.as_str())?
        .ok_or(ContractError::NotRegistered {})?;

    // Can't self-deregister from a governance tier
    if let Some(tier) = TIERS.may_load(deps.storage, current_tier_id)? {
        if tier.governance_only {
            return Err(ContractError::LockedToGovernanceTier {});
        }
    }

    REGISTRATIONS.remove(deps.storage, info.sender.as_str());

    Ok(Response::new()
        .add_attribute("action", "deregister")
        .add_attribute("wallet", info.sender))
}

fn execute_deregister_wallet(
    deps: DepsMut,
    info: MessageInfo,
    wallet: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let wallet_addr = deps.api.addr_validate(&wallet)?;

    let current_tier_id = REGISTRATIONS
        .may_load(deps.storage, wallet_addr.as_str())?
        .ok_or(ContractError::NotRegistered {})?;

    let is_governance = info.sender == config.governance;

    // For governance tiers, only governance can deregister
    if let Some(tier) = TIERS.may_load(deps.storage, current_tier_id)? {
        if tier.governance_only && !is_governance {
            return Err(ContractError::LockedToGovernanceTier {});
        }
    }

    if is_governance {
        // Governance can always deregister
        REGISTRATIONS.remove(deps.storage, wallet_addr.as_str());
    } else {
        // Non-governance callers: only deregister if balance is below threshold
        if let Some(tier) = TIERS.may_load(deps.storage, current_tier_id)? {
            if !tier.min_cl8y_balance.is_zero() {
                let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                    config.cl8y_token.to_string(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: wallet_addr.to_string(),
                    },
                )?;

                if balance.balance >= tier.min_cl8y_balance {
                    return Err(ContractError::Unauthorized {});
                }
            } else {
                return Err(ContractError::Unauthorized {});
            }
        }

        REGISTRATIONS.remove(deps.storage, wallet_addr.as_str());
    }

    Ok(Response::new()
        .add_attribute("action", "deregister_wallet")
        .add_attribute("wallet", wallet_addr))
}

fn execute_add_trusted_router(
    deps: DepsMut,
    info: MessageInfo,
    router: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let router_addr = deps.api.addr_validate(&router)?;
    TRUSTED_ROUTERS.save(deps.storage, router_addr.as_str(), &true)?;

    Ok(Response::new()
        .add_attribute("action", "add_trusted_router")
        .add_attribute("router", router_addr))
}

fn execute_remove_trusted_router(
    deps: DepsMut,
    info: MessageInfo,
    router: String,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let router_addr = deps.api.addr_validate(&router)?;
    TRUSTED_ROUTERS.remove(deps.storage, router_addr.as_str());

    Ok(Response::new()
        .add_attribute("action", "remove_trusted_router")
        .add_attribute("router", router_addr))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    governance: Option<String>,
    cl8y_token: Option<String>,
) -> Result<Response, ContractError> {
    ensure_governance(&deps, &info)?;
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(gov) = governance {
        config.governance = deps.api.addr_validate(&gov)?;
    }
    if let Some(token) = cl8y_token {
        config.cl8y_token = deps.api.addr_validate(&token)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("governance", config.governance)
        .add_attribute("cl8y_token", config.cl8y_token))
}

// ---- Queries ----

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetDiscount { trader, sender } => {
            to_json_binary(&query_discount(deps, trader, sender)?)
        }
        QueryMsg::GetTier { tier_id } => to_json_binary(&query_tier(deps, tier_id)?),
        QueryMsg::GetTiers {} => to_json_binary(&query_tiers(deps)?),
        QueryMsg::GetRegistration { trader } => {
            to_json_binary(&query_registration(deps, trader)?)
        }
        QueryMsg::IsTrustedRouter { addr } => {
            to_json_binary(&query_is_trusted_router(deps, addr)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        governance: config.governance,
        cl8y_token: config.cl8y_token,
    })
}

fn query_discount(deps: Deps, trader: String, sender: String) -> StdResult<DiscountResponse> {
    let config = CONFIG.load(deps.storage)?;

    // Determine effective trader: if sender != trader, sender must be a trusted router
    let effective_trader = if trader != sender {
        let is_trusted = TRUSTED_ROUTERS
            .may_load(deps.storage, &sender)?
            .unwrap_or(false);
        if is_trusted {
            trader
        } else {
            sender
        }
    } else {
        trader
    };

    let tier_id = match REGISTRATIONS.may_load(deps.storage, &effective_trader)? {
        Some(id) => id,
        None => {
            return Ok(DiscountResponse {
                discount_bps: 0,
                needs_deregister: false,
            });
        }
    };

    let tier = match TIERS.may_load(deps.storage, tier_id)? {
        Some(t) => t,
        None => {
            return Ok(DiscountResponse {
                discount_bps: 0,
                needs_deregister: true,
            });
        }
    };

    // Governance tiers skip balance check and are never auto-deregistered
    if tier.governance_only {
        return Ok(DiscountResponse {
            discount_bps: tier.discount_bps,
            needs_deregister: false,
        });
    }

    // For tiers with 0 min balance, always eligible
    if tier.min_cl8y_balance.is_zero() {
        return Ok(DiscountResponse {
            discount_bps: tier.discount_bps,
            needs_deregister: false,
        });
    }

    // Check CL8Y balance
    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.cl8y_token.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: effective_trader,
        },
    )?;

    if balance.balance >= tier.min_cl8y_balance {
        Ok(DiscountResponse {
            discount_bps: tier.discount_bps,
            needs_deregister: false,
        })
    } else {
        Ok(DiscountResponse {
            discount_bps: 0,
            needs_deregister: true,
        })
    }
}

fn query_tier(deps: Deps, tier_id: u8) -> StdResult<TierResponse> {
    let tier = TIERS.load(deps.storage, tier_id)?;
    Ok(TierResponse { tier_id, tier })
}

fn query_tiers(deps: Deps) -> StdResult<TiersResponse> {
    let tiers: Vec<TierEntry> = TIERS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (tier_id, tier) = item?;
            Ok(TierEntry { tier_id, tier })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(TiersResponse { tiers })
}

fn query_registration(deps: Deps, trader: String) -> StdResult<RegistrationResponse> {
    match REGISTRATIONS.may_load(deps.storage, &trader)? {
        Some(tier_id) => {
            let tier = TIERS.may_load(deps.storage, tier_id)?;
            Ok(RegistrationResponse {
                registered: true,
                tier_id: Some(tier_id),
                tier,
            })
        }
        None => Ok(RegistrationResponse {
            registered: false,
            tier_id: None,
            tier: None,
        }),
    }
}

fn query_is_trusted_router(deps: Deps, addr: String) -> StdResult<IsTrustedRouterResponse> {
    let is_trusted = TRUSTED_ROUTERS
        .may_load(deps.storage, &addr)?
        .unwrap_or(false);
    Ok(IsTrustedRouterResponse { is_trusted })
}
