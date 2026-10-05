//! Issues bridged-token borrows for the CMM LP and a 1:1 debt receipt.
//!
//! `Issue` mints the bridged CW20 (USDT, later other assets) to the liquidity
//! recipient and mints the same amount of `XXX-Bridge-Debt` to the debtor, the
//! CMM. Repaying the bridged token through this contract burns both. User
//! bridge withdrawals stay on the bridge: they are backed by a remote lock and
//! do not mint a Bridge-Debt token.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, ContractInfoResponse, Deps, DepsMut, Env, MessageInfo,
    Order, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MarketResponse, MarketsResponse,
    MigrateMsg, QueryMsg,
};
use crate::state::{
    Config, IssuanceLimits, Market, MarketPins, UnderlyingPolicy, UpgradePins, CONFIG,
    CONTRACT_NAME, CONTRACT_VERSION, DEBT_TOKENS, MARKETS, UNDERLYING_CODES,
};
#[path = "security.rs"]
mod security;
use security::*;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    CONFIG.save(
        deps.storage,
        &Config {
            owner,
            debt_code_id: None,
            pending_owner: None,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    match msg {
        ExecuteMsg::ApproveUnderlyingCode {
            code_id,
            multi_minter,
        } => approve_underlying(deps, info, code_id, multi_minter),
        ExecuteMsg::ConfigureMarket {
            underlying,
            limits,
            paused,
        } => configure_market(deps, env, info, underlying, limits, paused),
        ExecuteMsg::ProposeMarketUpgrade {
            underlying,
            debt_code_id,
            underlying_code_id,
        } => propose_upgrade(
            deps,
            env,
            info,
            underlying,
            debt_code_id,
            underlying_code_id,
        ),
        ExecuteMsg::AcceptMarketUpgrade { underlying } => {
            accept_upgrade(deps, env, info, underlying)
        }
        ExecuteMsg::CancelMarketUpgrade { underlying } => cancel_upgrade(deps, info, underlying),
        ExecuteMsg::AcceptOwner {} => accept_owner(deps, info),
        ExecuteMsg::CancelOwnerTransfer {} => cancel_owner(deps, info),
        ExecuteMsg::SetDebtCodeId { code_id } => execute_set_debt_code_id(deps, info, code_id),
        ExecuteMsg::RegisterMarket {
            underlying,
            debt_token,
        } => execute_register_market(deps, env, info, underlying, debt_token),
        ExecuteMsg::Issue {
            underlying,
            recipient,
            amount,
        } => execute_issue(deps, env, info, underlying, recipient, amount),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Recover {
            underlying,
            recipient,
            amount,
        } => execute_recover(deps, env, info, underlying, recipient, amount),
        ExecuteMsg::UpdateOwner { owner } => execute_update_owner(deps, info, owner),
    }
}

fn execute_set_debt_code_id(
    deps: DepsMut,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    if code_id == 0 {
        return Err(ContractError::UnexpectedDebtCode {});
    }
    let mut config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    config.debt_code_id = Some(code_id);
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_debt_code_id")
        .add_attribute("code_id", code_id.to_string()))
}

fn execute_register_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    debt_token: String,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;

    let underlying = deps.api.addr_validate(&underlying)?;
    let debt_token = deps.api.addr_validate(&debt_token)?;
    if underlying == debt_token
        || underlying == env.contract.address
        || debt_token == env.contract.address
    {
        return Err(ContractError::InvalidMarket {});
    }
    if MARKETS.may_load(deps.storage, &underlying)?.is_some() {
        return Err(ContractError::MarketExists {});
    }
    if DEBT_TOKENS.has(deps.storage, &debt_token) {
        return Err(ContractError::DebtTokenInUse {});
    }

    let debt_code_id = config.debt_code_id.ok_or(ContractError::DebtCodeUnset {})?;
    let debt = inspect_debt_token(deps.as_ref(), &env, &debt_token, debt_code_id)?;
    let underlying_code_id = contract_code(deps.as_ref(), &underlying)?;
    let policy = UNDERLYING_CODES
        .may_load(deps.storage, underlying_code_id)?
        .ok_or(ContractError::UnapprovedImplementation {})?;
    let pins = inspect_underlying(
        deps.as_ref(),
        &env,
        &underlying,
        debt_code_id,
        underlying_code_id,
        policy.multi_minter,
    )?;
    let decimals = underlying_decimals(deps.as_ref(), &underlying)?;
    if debt.decimals != decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    if !debt.total_supply.is_zero() {
        return Err(ContractError::DebtSupplyNotZero {});
    }
    if debt.debtor == env.contract.address || debt.debtor == debt_token {
        return Err(ContractError::InvalidDebtor {});
    }

    let market = Market {
        underlying: underlying.clone(),
        debt_token: debt_token.clone(),
        debtor: debt.debtor,
        decimals,
        outstanding: Uint128::zero(),
        pins: Some(pins),
        limits: None,
        paused: false,
        epoch: 0,
        epoch_issued: Uint128::zero(),
        pending_upgrade: None,
    };
    MARKETS.save(deps.storage, &underlying, &market)?;
    DEBT_TOKENS.save(deps.storage, &debt_token, &cosmwasm_std::Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "register_bridge_debt_market")
        .add_attribute("underlying", market.underlying)
        .add_attribute("debt_token", market.debt_token)
        .add_attribute("debtor", market.debtor)
        .add_attribute("decimals", market.decimals.to_string()))
}

fn execute_issue(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let underlying = deps.api.addr_validate(&underlying)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;

    consume_limit(&mut market, &env, amount)?;
    market.outstanding = market
        .outstanding
        .checked_add(amount)
        .map_err(|_| ContractError::Overflow {})?;
    MARKETS.save(deps.storage, &underlying, &market)?;

    let mint_underlying = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: recipient.to_string(),
            amount,
        })?,
        funds: vec![],
    };
    let mint_debt = WasmMsg::Execute {
        contract_addr: market.debt_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: market.debtor.to_string(),
            amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(mint_underlying)
        .add_message(mint_debt)
        .add_attribute("action", "issue_bridge_debt")
        .add_attribute("underlying", market.underlying)
        .add_attribute("debt_token", market.debt_token)
        .add_attribute("recipient", recipient)
        .add_attribute("debtor", market.debtor)
        .add_attribute("amount", amount))
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook: Cw20HookMsg = from_json(&msg.msg).map_err(|_| ContractError::InvalidHook {})?;
    let Cw20HookMsg::Repay {} = hook;
    if msg.amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let mut market = load_market(deps.as_ref(), &info.sender)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;
    if msg.amount > market.outstanding {
        return Err(ContractError::RepayExceedsDebt {});
    }

    market.outstanding -= msg.amount;
    if let Some(pending) = &mut market.pending_upgrade {
        pending.underlying_supply = pending
            .underlying_supply
            .checked_sub(msg.amount)
            .map_err(|_| ContractError::DebtSupplyDiverged {})?;
    }
    MARKETS.save(deps.storage, &info.sender, &market)?;

    let burn_underlying = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount: msg.amount })?,
        funds: vec![],
    };
    let burn_debt = WasmMsg::Execute {
        contract_addr: market.debt_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::BurnFrom {
            owner: market.debtor.to_string(),
            amount: msg.amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(burn_underlying)
        .add_message(burn_debt)
        .add_attribute("action", "repay_bridge_debt")
        .add_attribute("underlying", market.underlying)
        .add_attribute("payer", msg.sender)
        .add_attribute("debtor", market.debtor)
        .add_attribute("amount", msg.amount)
        .add_attribute("outstanding", market.outstanding))
}

fn execute_recover(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    let underlying = deps.api.addr_validate(&underlying)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    let market = load_market(deps.as_ref(), &underlying)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;

    let transfer = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(transfer)
        .add_attribute("action", "recover_bridged_token")
        .add_attribute("underlying", market.underlying)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount))
}

fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let mut config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    let owner = deps.api.addr_validate(&owner)?;
    config.pending_owner = Some(owner.clone());
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "propose_owner")
        .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Market { underlying } => to_json_binary(&query_market(deps, underlying)?),
        QueryMsg::Markets { start_after, limit } => {
            to_json_binary(&query_markets(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        pending_owner: config.pending_owner,
        debt_code_id: config.debt_code_id,
    })
}

fn query_market(deps: Deps, underlying: String) -> StdResult<MarketResponse> {
    let underlying = deps.api.addr_validate(&underlying)?;
    let market = MARKETS.load(deps.storage, &underlying)?;
    Ok(market_response(market))
}

fn query_markets(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MarketsResponse> {
    let limit = match limit {
        Some(n) if n > 0 => n.min(30),
        _ => 10,
    } as usize;
    let start = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let min = start.as_ref().map(Bound::exclusive);

    let markets = MARKETS
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, market) = item?;
            Ok(market_response(market))
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(MarketsResponse { markets })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_markets(deps, env, msg)
}

struct DebtView {
    debtor: Addr,
    decimals: u8,
    total_supply: Uint128,
}

fn inspect_debt_token(
    deps: Deps,
    env: &Env,
    debt_token: &Addr,
    allowed: u64,
) -> Result<DebtView, ContractError> {
    let code: ContractInfoResponse =
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                contract_addr: debt_token.to_string(),
            }))?;
    if code.code_id != allowed {
        return Err(ContractError::UnexpectedDebtCode {});
    }

    let info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(debt_token, &Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    let minter: MinterResponse = deps
        .querier
        .query_wasm_smart(debt_token, &Cw20QueryMsg::Minter {})
        .map_err(|_| ContractError::DebtMinterMismatch {})?;
    let minter = deps.api.addr_validate(&minter.minter)?;
    if minter != env.contract.address {
        return Err(ContractError::DebtMinterMismatch {});
    }

    let debtor: DebtorProbe = deps
        .querier
        .query_wasm_smart(debt_token, &DebtProbe::Debtor {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    let debtor = deps.api.addr_validate(&debtor.debtor)?;

    let listed: MintersProbe = deps
        .querier
        .query_wasm_smart(debt_token, &DebtProbe::Minters {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    if !listed.minters.is_empty() {
        return Err(ContractError::AdditionalMinters {});
    }

    Ok(DebtView {
        debtor,
        decimals: info.decimals,
        total_supply: info.total_supply,
    })
}

fn assert_market_consistent(deps: Deps, env: &Env, market: &Market) -> Result<(), ContractError> {
    let pins = market
        .pins
        .as_ref()
        .ok_or(ContractError::UnapprovedImplementation {})?;
    let debt = inspect_debt_token(deps, env, &market.debt_token, pins.debt_code_id)?;
    let current = inspect_underlying(
        deps,
        env,
        &market.underlying,
        pins.debt_code_id,
        pins.underlying_code_id,
        pins.multi_minter,
    )?;
    if &current != pins {
        return Err(ContractError::UnderlyingMinterMismatch {});
    }
    let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        &market.debt_token,
        &Cw20QueryMsg::Balance {
            address: market.debtor.to_string(),
        },
    )?;
    if balance.balance != market.outstanding {
        return Err(ContractError::DebtSupplyDiverged {});
    }
    if debt.debtor != market.debtor {
        return Err(ContractError::DebtorMismatch {});
    }
    if debt.decimals != market.decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    if debt.total_supply != market.outstanding {
        return Err(ContractError::DebtSupplyDiverged {});
    }
    let decimals = underlying_decimals(deps, &market.underlying)?;
    if decimals != market.decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    Ok(())
}

fn underlying_decimals(deps: Deps, underlying: &Addr) -> Result<u8, ContractError> {
    let info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(underlying, &Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::InvalidUnderlying {})?;
    if info.decimals > 18 {
        return Err(ContractError::InvalidUnderlying {});
    }
    Ok(info.decimals)
}

fn load_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

fn load_market(deps: Deps, underlying: &Addr) -> Result<Market, ContractError> {
    MARKETS
        .load(deps.storage, underlying)
        .map_err(|_| ContractError::UnknownMarket {})
}

fn must_owner(config: &Config, info: &MessageInfo) -> Result<(), ContractError> {
    if info.sender == config.owner {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

fn reject_funds(info: &MessageInfo) -> Result<(), ContractError> {
    if info.funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::UnexpectedFunds {})
    }
}

fn market_response(market: Market) -> MarketResponse {
    MarketResponse {
        underlying: market.underlying,
        debt_token: market.debt_token,
        debtor: market.debtor,
        decimals: market.decimals,
        outstanding: market.outstanding,
        debt_code_id: market.pins.as_ref().map(|p| p.debt_code_id),
        underlying_code_id: market.pins.as_ref().map(|p| p.underlying_code_id),
        pins: market.pins,
        limits: market.limits,
        paused: market.paused,
        epoch: market.epoch,
        epoch_issued: market.epoch_issued,
        pending_upgrade: market.pending_upgrade,
    }
}

#[cosmwasm_schema::cw_serde]
enum DebtProbe {
    Debtor {},
    Minters {},
}

#[cosmwasm_schema::cw_serde]
struct DebtorProbe {
    debtor: String,
}

#[cosmwasm_schema::cw_serde]
struct MintersProbe {
    minters: Vec<String>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
