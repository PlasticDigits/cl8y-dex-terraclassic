use super::*;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_json, Addr, Coin, Storage};

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

fn snapshot(storage: &dyn Storage) -> Vec<(Vec<u8>, Vec<u8>)> {
    storage.range(None, None, Order::Ascending).collect()
}

#[test]
fn migration_accepts_supported_versions_and_preserves_receipt_state() {
    for source_version in ["1.0.0", CONTRACT_VERSION] {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), Some(Uint128::new(100)));
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 70).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(MINTER, &[]),
            ExecuteMsg::BurnFrom {
                owner: DEBTOR_ADDR.to_string(),
                amount: Uint128::new(20),
            },
        )
        .unwrap();
        let token = TOKEN_INFO.load(&deps.storage).unwrap();
        let debtor = DEBTOR.load(&deps.storage).unwrap();
        let balances: Vec<_> = BALANCES
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();
        set_contract_version(&mut deps.storage, CONTRACT_NAME, source_version).unwrap();

        let response = migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
        assert_eq!(response.attributes[1].value, source_version);
        assert_eq!(TOKEN_INFO.load(&deps.storage).unwrap(), token);
        assert_eq!(DEBTOR.load(&deps.storage).unwrap(), debtor);
        assert_eq!(
            BALANCES
                .range(&deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()
                .unwrap(),
            balances
        );
        assert_eq!(token.total_supply, Uint128::new(50));
        assert_eq!(
            get_contract_version(&deps.storage).unwrap(),
            cw2::ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            }
        );
        // The upgraded receipt remains usable by the same sole minter.
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 1).unwrap();
        assert_eq!(
            TOKEN_INFO.load(&deps.storage).unwrap().total_supply,
            Uint128::new(51)
        );
    }
}

#[test]
fn migration_accepts_empty_receipt_without_creating_balance() {
    let mut deps = mock_dependencies();
    setup(deps.as_mut(), None);
    let before = snapshot(&deps.storage);
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    assert_eq!(snapshot(&deps.storage), before);
}

#[test]
fn migration_rejects_foreign_contract_and_unsupported_versions_without_writes() {
    let mut deps = mock_dependencies();
    setup(deps.as_mut(), None);
    set_contract_version(&mut deps.storage, "cw20-base", "1.0.0").unwrap();
    let before = snapshot(&deps.storage);
    assert_eq!(
        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap_err(),
        ContractError::InvalidMigrationContract {
            contract: "cw20-base".to_string(),
        }
    );
    assert_eq!(snapshot(&deps.storage), before);

    for version in [
        "0.9.0",
        "1.0.1",
        "1.2.0",
        "2.0.0",
        "1.1.0-rc.1",
        "1.0.0+custom",
        "not-semver",
    ] {
        set_contract_version(&mut deps.storage, CONTRACT_NAME, version).unwrap();
        let before = snapshot(&deps.storage);
        assert_eq!(
            migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap_err(),
            ContractError::InvalidMigrationVersion {
                version: version.to_string(),
            }
        );
        assert_eq!(snapshot(&deps.storage), before);
    }
}

#[test]
fn migration_rejects_broken_receipt_invariants_without_writes() {
    for invalid_state in [
        "supply",
        "foreign_balance",
        "foreign_zero",
        "decimals",
        "minter",
        "cap",
        "name",
        "symbol",
        "address",
        "missing_debtor",
        "missing_token",
    ] {
        let mut deps = mock_dependencies();
        setup(deps.as_mut(), None);
        mint(deps.as_mut(), MINTER, DEBTOR_ADDR, 5).unwrap();
        set_contract_version(&mut deps.storage, CONTRACT_NAME, "1.0.0").unwrap();
        let mut token = TOKEN_INFO.load(&deps.storage).unwrap();
        match invalid_state {
            "supply" => token.total_supply = Uint128::new(6),
            "foreign_balance" => BALANCES
                .save(&mut deps.storage, &Addr::unchecked(OTHER), &Uint128::new(1))
                .unwrap(),
            "foreign_zero" => BALANCES
                .save(&mut deps.storage, &Addr::unchecked(OTHER), &Uint128::zero())
                .unwrap(),
            "decimals" => token.decimals = 19,
            "minter" => token.mint.minter = Addr::unchecked(DEBTOR_ADDR),
            "cap" => token.mint.cap = Some(Uint128::new(4)),
            "name" => token.name = "x".to_string(),
            "symbol" => token.symbol = "wrong ticker".to_string(),
            "address" => token.mint.minter = Addr::unchecked("x"),
            "missing_debtor" => DEBTOR.remove(&mut deps.storage),
            "missing_token" => {}
            _ => unreachable!(),
        }
        TOKEN_INFO.save(&mut deps.storage, &token).unwrap();
        if invalid_state == "missing_token" {
            TOKEN_INFO.remove(&mut deps.storage);
        }
        let before = snapshot(&deps.storage);
        assert!(
            migrate(deps.as_mut(), mock_env(), MigrateMsg {}).is_err(),
            "accepted {invalid_state}"
        );
        assert_eq!(snapshot(&deps.storage), before, "mutated {invalid_state}");
    }
}
