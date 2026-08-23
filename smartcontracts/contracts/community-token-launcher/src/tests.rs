use cl8y_community_tax_token::msg::{InstantiateMsg as TokenInit, Sku, INVOICE_UST1};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{ConfigResponse, CreateTokenMsg, InstantiateMsg, InvoiceHookMsg, QueryMsg};

fn launcher_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply),
    )
}

fn token_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cl8y_community_tax_token::contract::execute,
        cl8y_community_tax_token::contract::instantiate,
        cl8y_community_tax_token::contract::query,
    ))
}

fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

#[test]
fn create_token_stamps_cmm_admin_and_forwards_sku_fee() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let cmm_gov = Addr::unchecked("cmm_gov");
    let cmm_treas = Addr::unchecked("cmm_treas");
    let factory = Addr::unchecked("factory");

    let ust1_code = app.store_code(cw20_base_contract());
    let token_code = app.store_code(token_contract());
    let launcher_code = app.store_code(launcher_contract());

    let ust1 = app
        .instantiate_contract(
            ust1_code,
            manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "UST1".into(),
                symbol: "USTT".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(500_000_000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "ust1",
            None,
        )
        .unwrap();

    let launcher = app
        .instantiate_contract(
            launcher_code,
            manager.clone(),
            &InstantiateMsg {
                token_code_id: token_code,
                autolp_code_id: None,
                ust1: ust1.to_string(),
                cmm_treasury: cmm_treas.to_string(),
                cmm_governance: cmm_gov.to_string(),
                factory: factory.to_string(),
                router: Some("router".into()),
            },
            &[],
            "launcher",
            Some(cmm_gov.to_string()),
        )
        .unwrap();

    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&launcher, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(cfg.cmm_governance, cmm_gov);
    assert_eq!(cfg.token_code_id, token_code);

    app.execute_contract(
        manager.clone(),
        ust1.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: launcher.to_string(),
            amount: Uint128::new(INVOICE_UST1),
            msg: cosmwasm_std::to_json_binary(&InvoiceHookMsg::CreateToken(Box::new(
                CreateTokenMsg {
                    name: "Comm".into(),
                    symbol: "COMM".into(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: manager.to_string(),
                        amount: Uint128::new(1_000_000),
                    }],
                    manager: manager.to_string(),
                    treasury: manager.to_string(),
                    buy_bps: 100,
                    sell_bps: 100,
                    max_buy_bps: 1000,
                    max_sell_bps: 1000,
                    max_transfer_bps: 500,
                    features: vec![Sku::TransferTax],
                    mint: None,
                    transfer_bps: Some(0),
                    sinks: None,
                    launch_guards: None,
                    autolp_threshold: None,
                    autolp_lp_recipient: None,
                },
            )))
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    let bal: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            &ust1,
            &cw20::Cw20QueryMsg::Balance {
                address: cmm_treas.to_string(),
            },
        )
        .unwrap();
    assert_eq!(bal.balance.u128(), INVOICE_UST1);
}

#[test]
fn create_token_wrong_invoice_rejected() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let ust1_code = app.store_code(cw20_base_contract());
    let token_code = app.store_code(token_contract());
    let launcher_code = app.store_code(launcher_contract());
    let ust1 = app
        .instantiate_contract(
            ust1_code,
            manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "UST1".into(),
                symbol: "USTT".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(500_000_000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "ust1",
            None,
        )
        .unwrap();
    let launcher = app
        .instantiate_contract(
            launcher_code,
            manager.clone(),
            &InstantiateMsg {
                token_code_id: token_code,
                autolp_code_id: None,
                ust1: ust1.to_string(),
                cmm_treasury: manager.to_string(),
                cmm_governance: manager.to_string(),
                factory: manager.to_string(),
                router: None,
            },
            &[],
            "launcher",
            None,
        )
        .unwrap();
    let err = app
        .execute_contract(
            manager.clone(),
            ust1,
            &cw20::Cw20ExecuteMsg::Send {
                contract: launcher.to_string(),
                amount: Uint128::new(1),
                msg: cosmwasm_std::to_json_binary(&InvoiceHookMsg::CreateToken(Box::new(
                    CreateTokenMsg {
                        name: "Comm".into(),
                        symbol: "COMM".into(),
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: manager.to_string(),
                            amount: Uint128::new(1),
                        }],
                        manager: manager.to_string(),
                        treasury: manager.to_string(),
                        buy_bps: 0,
                        sell_bps: 0,
                        max_buy_bps: 1000,
                        max_sell_bps: 1000,
                        max_transfer_bps: 0,
                        features: vec![Sku::TransferTax],
                        mint: None,
                        transfer_bps: None,
                        sinks: None,
                        launch_guards: None,
                        autolp_threshold: None,
                        autolp_lp_recipient: None,
                    },
                )))
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("exactly"));
}

#[allow(dead_code)]
fn _token_init(_: TokenInit) {}
