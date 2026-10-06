//! CW20 receipt for a bridged-mint borrow.
//!
//! One instance per borrowed asset: `USDT-Bridge-Debt` for bridged USDT, and
//! later `XXX-Bridge-Debt` for other bridged assets. The minter (the bridge-debt
//! issuer) mints 1:1 to the debtor and burns from that same balance on repay.
//! Holders cannot transfer or burn, so the debtor's balance stays equal to supply.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{BalanceResponse, MinterResponse, TokenInfoResponse};
use semver::Version;

use crate::error::ContractError;
use crate::msg::{
    DebtorResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, MintersResponse, QueryMsg,
};
use crate::state::{
    MinterData, TokenInfo, BALANCES, CONTRACT_NAME, CONTRACT_VERSION, DEBTOR, SYMBOL_MAX_LEN,
    TOKEN_INFO,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_name(&msg.name)?;
    validate_symbol(&msg.symbol)?;
    if msg.decimals > 18 {
        return Err(ContractError::InvalidDecimals {});
    }

    let minter = deps.api.addr_validate(&msg.minter)?;
    let debtor = deps.api.addr_validate(&msg.debtor)?;
    if minter == debtor {
        return Err(ContractError::MinterIsDebtor {});
    }

    TOKEN_INFO.save(
        deps.storage,
        &TokenInfo {
            name: msg.name.clone(),
            symbol: msg.symbol.clone(),
            decimals: msg.decimals,
            total_supply: Uint128::zero(),
            mint: MinterData {
                minter: minter.clone(),
                cap: msg.cap,
            },
        },
    )?;
    DEBTOR.save(deps.storage, &debtor)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("name", msg.name)
        .add_attribute("symbol", msg.symbol)
        .add_attribute("minter", minter)
        .add_attribute("debtor", debtor))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    match msg {
        ExecuteMsg::Transfer { .. } | ExecuteMsg::Send { .. } | ExecuteMsg::Burn { .. } => {
            Err(ContractError::NotTransferable {})
        }
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, info, recipient, amount),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, info, owner, amount),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let mut token = TOKEN_INFO.load(deps.storage)?;
    if token.mint.minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let debtor = DEBTOR.load(deps.storage)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    if recipient != debtor {
        return Err(ContractError::Unauthorized {});
    }

    token.total_supply = token
        .total_supply
        .checked_add(amount)
        .map_err(|_| ContractError::Std(cosmwasm_std::StdError::generic_err("overflow")))?;
    if let Some(cap) = token.mint.cap {
        if token.total_supply > cap {
            return Err(ContractError::CannotExceedCap {});
        }
    }
    TOKEN_INFO.save(deps.storage, &token)?;

    BALANCES.update(deps.storage, &debtor, |balance| -> StdResult<_> {
        balance
            .unwrap_or_default()
            .checked_add(amount)
            .map_err(|err| cosmwasm_std::StdError::generic_err(err.to_string()))
    })?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", debtor)
        .add_attribute("amount", amount))
}

pub fn execute_burn_from(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let mut token = TOKEN_INFO.load(deps.storage)?;
    if token.mint.minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let debtor = DEBTOR.load(deps.storage)?;
    let owner = deps.api.addr_validate(&owner)?;
    if owner != debtor {
        return Err(ContractError::Unauthorized {});
    }

    let balance = BALANCES
        .may_load(deps.storage, &debtor)?
        .unwrap_or_default();
    if balance < amount {
        return Err(ContractError::InsufficientBalance {});
    }
    let left = balance - amount;
    if left.is_zero() {
        BALANCES.remove(deps.storage, &debtor);
    } else {
        BALANCES.save(deps.storage, &debtor, &left)?;
    }

    token.total_supply = token
        .total_supply
        .checked_sub(amount)
        .map_err(|_| ContractError::InsufficientBalance {})?;
    TOKEN_INFO.save(deps.storage, &token)?;

    Ok(Response::new()
        .add_attribute("action", "burn_from")
        .add_attribute("from", debtor)
        .add_attribute("amount", amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::Debtor {} => to_json_binary(&query_debtor(deps)?),
        QueryMsg::Minters {} => to_json_binary(&MintersResponse { minters: vec![] }),
    }
}

fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    Ok(BalanceResponse { balance })
}

fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    Ok(TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    })
}

fn query_minter(deps: Deps) -> StdResult<MinterResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    Ok(MinterResponse {
        minter: info.mint.minter.into_string(),
        cap: info.mint.cap,
    })
}

fn query_debtor(deps: Deps) -> StdResult<DebtorResponse> {
    let debtor = DEBTOR.load(deps.storage)?;
    Ok(DebtorResponse {
        debtor: debtor.into_string(),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let source = get_contract_version(deps.storage)?;
    if source.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidMigrationContract {
            contract: source.contract,
        });
    }
    let source_version =
        Version::parse(&source.version).map_err(|_| ContractError::InvalidMigrationVersion {
            version: source.version.clone(),
        })?;
    // Only these releases share the receipt storage layout. Same-version
    // migration is deliberately supported for a lossless code replacement.
    if source_version != Version::new(1, 0, 0) && source_version != Version::new(1, 1, 0) {
        return Err(ContractError::InvalidMigrationVersion {
            version: source.version,
        });
    }
    validate_migration_state(deps.as_ref())?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", source.version)
        .add_attribute("to_version", CONTRACT_VERSION))
}

/// Reject broken receipts instead of repairing accounting or changing identity.
/// All validation precedes the only write, the cw2 version update.
fn validate_migration_state(deps: Deps) -> Result<(), ContractError> {
    let token = TOKEN_INFO.load(deps.storage)?;
    let debtor = DEBTOR.load(deps.storage)?;
    validate_name(&token.name)?;
    validate_symbol(&token.symbol)?;
    deps.api.addr_validate(debtor.as_str())?;
    deps.api.addr_validate(token.mint.minter.as_str())?;
    if token.decimals > 18 {
        return Err(ContractError::InvalidDecimals {});
    }
    if token.mint.minter == debtor {
        return Err(ContractError::MinterIsDebtor {});
    }
    if token.mint.cap.is_some_and(|cap| cap < token.total_supply) {
        return Err(ContractError::CannotExceedCap {});
    }
    let balance = BALANCES
        .may_load(deps.storage, &debtor)?
        .unwrap_or_default();
    if balance != token.total_supply {
        return Err(ContractError::InvalidMigrationState {
            reason: "debtor balance must equal total supply".to_string(),
        });
    }
    // Receipts can have only the debtor's balance entry; even zero-valued
    // foreign entries indicate a state layout we do not support.
    for entry in BALANCES.range(deps.storage, None, None, Order::Ascending) {
        let (address, _) = entry?;
        if address != debtor {
            return Err(ContractError::InvalidMigrationState {
                reason: "receipt balances must belong only to the debtor".to_string(),
            });
        }
    }
    Ok(())
}

fn reject_funds(info: &MessageInfo) -> Result<(), ContractError> {
    if info.funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::UnexpectedFunds {})
    }
}

fn validate_name(name: &str) -> Result<(), ContractError> {
    if (3..=50).contains(&name.len()) {
        Ok(())
    } else {
        Err(ContractError::InvalidName {})
    }
}

fn validate_symbol(symbol: &str) -> Result<(), ContractError> {
    let bytes = symbol.as_bytes();
    if (3..=SYMBOL_MAX_LEN).contains(&bytes.len())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    {
        Ok(())
    } else {
        Err(ContractError::InvalidSymbol {})
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
