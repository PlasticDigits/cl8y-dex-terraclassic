//! CW20 receipt for a bridged-mint borrow.
//!
//! One instance per borrowed asset: `USDT-Bridge-Debt` for bridged USDT, and
//! later `XXX-Bridge-Debt` for other bridged assets. The minter (the bridge-debt
//! issuer) mints 1:1 to the debtor and burns from that same balance on repay.
//! Holders cannot transfer or burn, so the debtor's balance stays equal to supply.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, MinterResponse, TokenInfoResponse};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
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
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Coin};

    const MINTER: &str = "terra1owner000000000000000000000000000000000";
    const DEBTOR_ADDR: &str = "terra1cmm00000000000000000000000000000000000";
    const OTHER: &str = "terra1user00000000000000000000000000000000";

    fn setup(deps: DepsMut, cap: Option<Uint128>) {
        instantiate(
            deps,
            mock_env(),
            mock_info("creator", &[]),
            InstantiateMsg {
                name: "USDT-Bridge-Debt".to_string(),
                symbol: "USDT-Bridge-Debt".to_string(),
                decimals: 18,
                minter: MINTER.to_string(),
                debtor: DEBTOR_ADDR.to_string(),
                cap,
            },
        )
        .unwrap();
    }

    fn mint(
        deps: DepsMut,
        sender: &str,
        recipient: &str,
        amount: u128,
    ) -> Result<Response, ContractError> {
        execute(
            deps,
            mock_env(),
            mock_info(sender, &[]),
            ExecuteMsg::Mint {
                recipient: recipient.to_string(),
                amount: Uint128::new(amount),
            },
        )
    }

    #[test]
    fn usdt_bridge_debt_symbol_is_accepted() {
        assert_eq!("USDT-Bridge-Debt".len(), 16);
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), None);

        let info: TokenInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap()).unwrap();
        assert_eq!(info.name, "USDT-Bridge-Debt");
        assert_eq!(info.symbol, "USDT-Bridge-Debt");
        assert_eq!(info.decimals, 18);
        assert_eq!(info.total_supply, Uint128::zero());

        let debtor: DebtorResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Debtor {}).unwrap()).unwrap();
        assert_eq!(debtor.debtor, DEBTOR_ADDR);

        let minters: MintersResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Minters {}).unwrap()).unwrap();
        assert!(minters.minters.is_empty());
    }

    #[test]
    fn rejects_symbol_that_does_not_fit_the_pattern() {
        let mut deps = mock_dependencies();
        let err = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            InstantiateMsg {
                name: "USDT-Bridge-Debt".to_string(),
                symbol: "USDT-Bridge-Debt-TOO-LONG-FOR-A-TICKER".to_string(),
                decimals: 18,
                minter: MINTER.to_string(),
                debtor: DEBTOR_ADDR.to_string(),
                cap: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::InvalidSymbol {});
    }

    #[test]
    fn mint_goes_only_to_the_debtor() {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), None);

        let amount = 10_000_000_000_000_000_000_000u128;
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, amount).unwrap();

        let balance: BalanceResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Balance {
                    address: DEBTOR_ADDR.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(balance.balance, Uint128::new(amount));

        let err = mint(deps.as_mut(), MINTER, OTHER, 1).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        let err = mint(deps.as_mut(), OTHER, DEBTOR_ADDR, 1).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn holder_cannot_transfer_or_burn() {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), None);
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 5).unwrap();

        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEBTOR_ADDR, &[]),
            ExecuteMsg::Transfer {
                recipient: OTHER.to_string(),
                amount: Uint128::new(1),
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::NotTransferable {});

        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEBTOR_ADDR, &[]),
            ExecuteMsg::Burn {
                amount: Uint128::new(1),
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::NotTransferable {});

        let balance: BalanceResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Balance {
                    address: DEBTOR_ADDR.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(balance.balance, Uint128::new(5));
    }

    #[test]
    fn minter_burns_only_the_debtor_balance() {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), None);
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 5).unwrap();

        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(MINTER, &[]),
            ExecuteMsg::BurnFrom {
                owner: OTHER.to_string(),
                amount: Uint128::new(1),
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(MINTER, &[]),
            ExecuteMsg::BurnFrom {
                owner: DEBTOR_ADDR.to_string(),
                amount: Uint128::new(5),
            },
        )
        .unwrap();

        let info: TokenInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap()).unwrap();
        assert_eq!(info.total_supply, Uint128::zero());
    }

    #[test]
    fn cap_and_zero_amount_are_enforced() {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), Some(Uint128::new(10)));
        let err = mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 11).unwrap_err();
        assert_eq!(err, ContractError::CannotExceedCap {});

        let err = mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 0).unwrap_err();
        assert_eq!(err, ContractError::ZeroAmount {});
    }

    #[test]
    fn minter_cannot_be_the_debtor() {
        let mut deps = mock_dependencies();
        let err = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            InstantiateMsg {
                name: "USDT-Bridge-Debt".to_string(),
                symbol: "USDT-Bridge-Debt".to_string(),
                decimals: 18,
                minter: DEBTOR_ADDR.to_string(),
                debtor: DEBTOR_ADDR.to_string(),
                cap: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::MinterIsDebtor {});
    }

    #[test]
    fn rejects_native_funds() {
        let mut deps = mock_dependencies();
        let err = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[Coin::new(1u128, "uluna")]),
            InstantiateMsg {
                name: "USDT-Bridge-Debt".to_string(),
                symbol: "USDT-Bridge-Debt".to_string(),
                decimals: 18,
                minter: MINTER.to_string(),
                debtor: DEBTOR_ADDR.to_string(),
                cap: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::UnexpectedFunds {});
    }
}
