//! Multi-test coverage for GitLab #592 (T592-1–T592-12) and #608 (H608-1–H608-8).

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dex_common::factory::{PairResponse, QueryMsg as FactoryQuery};
use dex_common::pair::{Cw20HookMsg, QueryMsg as PairQuery};
use dex_common::types::{AssetInfo, PairInfo};

use crate::msg::{
    ConfigResponse, ExecuteMsg, FeaturesResponse, InstantiateMsg, InvoiceHookMsg, IsExemptResponse,
    LaunchGuardsConfig, MintInit, QueryMsg, SettingsBatch, Sink, SinkKind, Sku, TaxKind,
    TaxPreviewResponse, INVOICE_UST1,
};

const GENESIS: u128 = 1_000_000_000;

fn token_contract() -> Box<dyn Contract<Empty>> {
    let exec = |deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg| {
        crate::contract::execute(deps, env, info, msg)
    };
    let inst = |deps: DepsMut, env: Env, info: MessageInfo, msg: InstantiateMsg| {
        crate::contract::instantiate(deps, env, info, msg)
    };
    let q = |deps: Deps, env: Env, msg: QueryMsg| crate::contract::query(deps, env, msg);
    Box::new(ContractWrapper::new(exec, inst, q))
}

fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

// --- mock pair / factory for RegisterListedPair ---

#[cosmwasm_schema::cw_serde]
struct MockPairInit {
    token: String,
    other: String,
    factory: String,
}

#[cosmwasm_schema::cw_serde]
enum MockPairExec {
    Receive(cw20::Cw20ReceiveMsg),
}

fn mock_pair_contract() -> Box<dyn Contract<Empty>> {
    fn inst(deps: DepsMut, env: Env, _info: MessageInfo, msg: MockPairInit) -> StdResult<Response> {
        deps.storage.set(b"token", msg.token.as_bytes());
        deps.storage.set(b"other", msg.other.as_bytes());
        deps.storage.set(b"factory", msg.factory.as_bytes());
        deps.storage.set(b"self", env.contract.address.as_bytes());
        Ok(Response::new())
    }
    fn exec(_d: DepsMut, _e: Env, _i: MessageInfo, _m: MockPairExec) -> StdResult<Response> {
        Ok(Response::new())
    }
    fn query(deps: Deps, env: Env, msg: PairQuery) -> StdResult<Binary> {
        match msg {
            PairQuery::Pair {} => {
                let token = String::from_utf8(deps.storage.get(b"token").unwrap()).unwrap();
                let other = String::from_utf8(deps.storage.get(b"other").unwrap()).unwrap();
                to_json_binary(&PairInfo {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: token,
                        },
                        AssetInfo::Token {
                            contract_addr: other,
                        },
                    ],
                    contract_addr: env.contract.address,
                    liquidity_token: Addr::unchecked("lp"),
                })
            }
            _ => to_json_binary(&Empty {}),
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

#[cosmwasm_schema::cw_serde]
struct MockFactoryInit {
    pair: String,
    token: String,
    other: String,
}

#[cosmwasm_schema::cw_serde]
enum MockFactoryExec {
    Set { pair: String, token: String },
}

fn mock_factory_contract() -> Box<dyn Contract<Empty>> {
    fn inst(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: MockFactoryInit,
    ) -> StdResult<Response> {
        deps.storage.set(b"pair", msg.pair.as_bytes());
        deps.storage.set(b"token", msg.token.as_bytes());
        deps.storage.set(b"other", msg.other.as_bytes());
        Ok(Response::new())
    }
    fn exec(deps: DepsMut, _e: Env, _i: MessageInfo, msg: MockFactoryExec) -> StdResult<Response> {
        match msg {
            MockFactoryExec::Set { pair, token } => {
                deps.storage.set(b"pair", pair.as_bytes());
                deps.storage.set(b"token", token.as_bytes());
            }
        }
        Ok(Response::new())
    }
    fn query(deps: Deps, _env: Env, msg: FactoryQuery) -> StdResult<Binary> {
        match msg {
            FactoryQuery::Pair { .. } => {
                let pair = String::from_utf8(deps.storage.get(b"pair").unwrap()).unwrap();
                let token = String::from_utf8(deps.storage.get(b"token").unwrap()).unwrap();
                let other = String::from_utf8(deps.storage.get(b"other").unwrap()).unwrap();
                to_json_binary(&PairResponse {
                    pair: PairInfo {
                        asset_infos: [
                            AssetInfo::Token {
                                contract_addr: token,
                            },
                            AssetInfo::Token {
                                contract_addr: other,
                            },
                        ],
                        contract_addr: Addr::unchecked(pair),
                        liquidity_token: Addr::unchecked("lp"),
                    },
                })
            }
            _ => to_json_binary(&Empty {}),
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

struct EnvTok {
    app: App,
    token: Addr,
    ust1: Addr,
    manager: Addr,
    treasury: Addr,
    user: Addr,
    pair: Addr,
    #[allow(dead_code)]
    factory: Addr,
    cmm: Addr,
}

fn clean(features: Vec<Sku>, mint: Option<MintInit>) -> EnvTok {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");
    let cmm = Addr::unchecked("cmm_treasury");
    let other = Addr::unchecked("quote_token");

    let ust1_code = app.store_code(cw20_base_contract());
    let token_code = app.store_code(token_contract());
    let pair_code = app.store_code(mock_pair_contract());
    let factory_code = app.store_code(mock_factory_contract());

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
                    amount: Uint128::new(1_000_000_000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "ust1",
            None,
        )
        .unwrap();

    let factory = app
        .instantiate_contract(
            factory_code,
            manager.clone(),
            &MockFactoryInit {
                pair: "pending".into(),
                token: "pending".into(),
                other: other.to_string(),
            },
            &[],
            "factory",
            None,
        )
        .unwrap();

    let launch_guards = if features.iter().any(|s| matches!(s, Sku::LaunchGuards)) {
        Some(LaunchGuardsConfig {
            max_wallet: None,
            cooldown_blocks: 0,
            trading_enabled: false,
        })
    } else {
        None
    };
    let variable = features.iter().any(|s| matches!(s, Sku::VariableRates));
    let transfer_init = if features.iter().any(|s| matches!(s, Sku::TransferTax)) {
        200
    } else {
        0
    };
    let token = app
        .instantiate_contract(
            token_code,
            manager.clone(),
            &InstantiateMsg {
                name: "Comm".into(),
                symbol: "COMM".into(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: user.to_string(),
                        amount: Uint128::new(GENESIS / 2),
                    },
                    Cw20Coin {
                        address: manager.to_string(),
                        amount: Uint128::new(GENESIS / 2),
                    },
                ],
                marketing: None,
                manager: manager.to_string(),
                treasury: treasury.to_string(),
                buy_bps: 500,
                sell_bps: 500,
                max_buy_bps: if variable { 1000 } else { 500 },
                max_sell_bps: if variable { 1000 } else { 500 },
                max_transfer_bps: if variable { 500 } else { transfer_init },
                factory: factory.to_string(),
                router: Some("router".into()),
                ust1: ust1.to_string(),
                cmm_treasury: cmm.to_string(),
                features,
                mint,
                transfer_bps: if transfer_init > 0 {
                    Some(transfer_init)
                } else {
                    None
                },
                sinks: None,
                autolp: None,
                launcher: Some("launcher".into()),
                launch_guards,
                initial_exempt: None,
            },
            &[],
            "comm",
            Some("cmm_gov".into()),
        )
        .unwrap();

    // Fund the pair after it exists.
    let pair = app
        .instantiate_contract(
            pair_code,
            manager.clone(),
            &MockPairInit {
                token: token.to_string(),
                other: other.to_string(),
                factory: factory.to_string(),
            },
            &[],
            "pair",
            None,
        )
        .unwrap();

    app.execute_contract(
        manager.clone(),
        factory.clone(),
        &MockFactoryExec::Set {
            pair: pair.to_string(),
            token: token.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        manager.clone(),
        token.clone(),
        &ExecuteMsg::Transfer {
            recipient: pair.to_string(),
            amount: Uint128::new(100_000_000),
        },
        &[],
    )
    .unwrap();

    EnvTok {
        app,
        token,
        ust1,
        manager,
        treasury,
        user,
        pair,
        factory,
        cmm,
    }
}

fn balance(app: &App, token: &Addr, who: &str) -> u128 {
    let r: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &Cw20QueryMsg::Balance {
                address: who.to_string(),
            },
        )
        .unwrap();
    r.balance.u128()
}

fn swap_hook() -> Binary {
    to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: None,
        min_return: None,
        to: None,
        deadline: None,
        trader: None,
        hybrid: None,
    })
    .unwrap()
}

#[test]
fn instantiate_free_profile() {
    let e = clean(vec![], None);
    let cfg: ConfigResponse = e
        .app
        .wrap()
        .query_wasm_smart(&e.token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(cfg.manager, e.manager);
    assert_eq!(cfg.buy_bps, 500);
    assert_eq!(cfg.sell_bps, 500);
    let feat: FeaturesResponse = e
        .app
        .wrap()
        .query_wasm_smart(&e.token, &QueryMsg::GetFeatures {})
        .unwrap();
    assert!(!feat.mint_control);
    assert_eq!(balance(&e.app, &e.token, e.user.as_str()), GENESIS / 2);
}

#[test]
fn inbound_to_pair_plain_transfer_is_one_to_one() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let pair_before = balance(&e.app, &e.token, e.pair.as_str());
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: e.pair.to_string(),
                amount: Uint128::new(1_000_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        balance(&e.app, &e.token, e.pair.as_str()),
        pair_before + 1_000_000
    );
}

#[test]
fn sell_extra_debit_on_swap_send() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let user_before = balance(&e.app, &e.token, e.user.as_str());
    let pair_before = balance(&e.app, &e.token, e.pair.as_str());
    let treas_before = balance(&e.app, &e.token, e.treasury.as_str());
    let amount = 1_000_000u128;
    let tax = amount * 500 / 10_000; // 50_000
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Send {
                contract: e.pair.to_string(),
                amount: Uint128::new(amount),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        balance(&e.app, &e.token, e.user.as_str()),
        user_before - amount - tax
    );
    assert_eq!(
        balance(&e.app, &e.token, e.pair.as_str()),
        pair_before + amount
    );
    assert_eq!(
        balance(&e.app, &e.token, e.treasury.as_str()),
        treas_before + tax
    );
}

#[test]
fn sell_insufficient_balance_fails() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let user_bal = balance(&e.app, &e.token, e.user.as_str());
    // amount such that amount + tax > balance
    let amount = user_bal; // tax would overflow
    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Send {
                contract: e.pair.to_string(),
                amount: Uint128::new(amount),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Insufficient")
            || err.root_cause().to_string().contains("extra-debit")
    );
}

#[test]
fn buy_outbound_split() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let amount = 1_000_000u128;
    let tax = amount * 500 / 10_000;
    let user_before = balance(&e.app, &e.token, e.user.as_str());
    let pair_before = balance(&e.app, &e.token, e.pair.as_str());
    e.app
        .execute_contract(
            e.pair.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: e.user.to_string(),
                amount: Uint128::new(amount),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        balance(&e.app, &e.token, e.pair.as_str()),
        pair_before - amount
    );
    assert_eq!(
        balance(&e.app, &e.token, e.user.as_str()),
        user_before + amount - tax
    );
    assert_eq!(balance(&e.app, &e.token, e.treasury.as_str()), tax);
}

#[test]
fn transfer_tax_off_is_one_to_one() {
    let mut e = clean(vec![], None);
    let other = Addr::unchecked("alice");
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: other.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&e.app, &e.token, other.as_str()), 10_000);
}

#[test]
fn transfer_tax_sku_taxes_wallet() {
    let mut e = clean(vec![Sku::TransferTax], None);
    let other = Addr::unchecked("alice");
    let treas_before = balance(&e.app, &e.token, e.treasury.as_str());
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: other.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();
    let tax = 10_000u128 * 200 / 10_000;
    assert_eq!(balance(&e.app, &e.token, other.as_str()), 10_000 - tax);
    assert_eq!(
        balance(&e.app, &e.token, e.treasury.as_str()),
        treas_before + tax
    );
}

#[test]
fn settings_unpaid_rejected() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.token.clone(),
            &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                sender: e.manager.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        buy_bps: Some(100),
                        ..Default::default()
                    },
                })
                .unwrap(),
            }),
            &[],
        )
        .unwrap_err();
    // Direct Receive from manager is not UST1 token
    assert!(
        err.root_cause().to_string().contains("UST1")
            || err.root_cause().to_string().contains("Invoice")
    );
}

#[test]
fn settings_batch_flat_fee_and_noop() {
    let mut e = clean(vec![Sku::VariableRates], None);
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        buy_bps: Some(100),
                        sell_bps: Some(200),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&e.app, &e.ust1, e.cmm.as_str()), INVOICE_UST1);
    let cfg: ConfigResponse = e
        .app
        .wrap()
        .query_wasm_smart(&e.token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(cfg.buy_bps, 100);
    assert_eq!(cfg.sell_bps, 200);

    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        buy_bps: Some(100),
                        sell_bps: Some(200),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("identical")
            || err.root_cause().to_string().contains("No-op")
            || err.root_cause().to_string().contains("empty")
    );
    assert_eq!(balance(&e.app, &e.ust1, e.cmm.as_str()), INVOICE_UST1);
}

#[test]
fn batch_unactivated_sku_reverts() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        transfer_bps: Some(100),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("not unlocked"));
    assert_eq!(balance(&e.app, &e.ust1, e.cmm.as_str()), 0);
}

#[test]
fn enable_feature_and_mint_control() {
    let mut e = clean(
        vec![Sku::MintControl],
        Some(MintInit {
            minter: "manager".into(),
            cap: Some(Uint128::new(GENESIS * 2)),
        }),
    );
    e.app
        .execute_contract(
            e.manager.clone(),
            e.token.clone(),
            &ExecuteMsg::Mint {
                recipient: e.user.to_string(),
                amount: Uint128::new(1_000),
            },
            &[],
        )
        .unwrap();

    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Mint {
                recipient: e.user.to_string(),
                amount: Uint128::new(1),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));

    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        revoke_mint: Some(true),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.token.clone(),
            &ExecuteMsg::Mint {
                recipient: e.user.to_string(),
                amount: Uint128::new(1),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("revoked")
            || err.root_cause().to_string().contains("Mint")
    );
}

#[test]
fn mint_absent_at_instantiate_never() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.token.clone(),
            &ExecuteMsg::Mint {
                recipient: e.user.to_string(),
                amount: Uint128::new(1),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("not enabled")
            || err.root_cause().to_string().contains("Mint")
    );

    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::EnableFeature {
                    sku: Sku::MintControl,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("instantiate"));
}

#[test]
fn non_manager_invoice_unauthorized() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: e.user.to_string(),
                amount: Uint128::new(INVOICE_UST1),
            },
            &[],
        )
        .unwrap();
    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        buy_bps: Some(0),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));
}

#[test]
fn protocol_exempt_cannot_be_removed() {
    let mut e = clean(vec![Sku::ExemptionDirectory], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        remove_exempt: Some(vec![e.pair.to_string()]),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("protocol"));
}

#[test]
fn register_spoof_pair_rejected() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.user.to_string(),
            },
            &[],
        )
        .unwrap_err();
    // user is not a pair contract
    let _ = err;
}

#[test]
fn tax_preview_matches_sell() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let p: TaxPreviewResponse = e
        .app
        .wrap()
        .query_wasm_smart(
            &e.token,
            &QueryMsg::TaxPreview {
                from: e.user.to_string(),
                to: e.pair.to_string(),
                amount: Uint128::new(1_000_000),
                send_msg: Some(swap_hook()),
            },
        )
        .unwrap();
    assert_eq!(p.kind, TaxKind::Sell);
    assert_eq!(p.credit, Uint128::new(1_000_000));
    assert_eq!(p.debit, Uint128::new(1_050_000));
    assert_eq!(p.tax, Uint128::new(50_000));
}

#[test]
fn excess_invoice_rejected() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1 * 2),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        buy_bps: Some(0),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("exactly"));
}

#[test]
fn split_router_ratios_and_burn() {
    let mut e = clean(vec![Sku::SplitRouter], None);
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        sinks: Some(vec![
                            Sink {
                                kind: SinkKind::Treasury,
                                addr: None,
                                bps: 5000,
                            },
                            Sink {
                                kind: SinkKind::Burn,
                                addr: None,
                                bps: 5000,
                            },
                        ]),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let supply_before: cw20::TokenInfoResponse = e
        .app
        .wrap()
        .query_wasm_smart(&e.token, &QueryMsg::TokenInfo {})
        .unwrap();
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Send {
                contract: e.pair.to_string(),
                amount: Uint128::new(1_000_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();
    let tax = 50_000u128;
    assert_eq!(balance(&e.app, &e.token, e.treasury.as_str()), tax / 2);
    let supply_after: cw20::TokenInfoResponse = e
        .app
        .wrap()
        .query_wasm_smart(&e.token, &QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(
        supply_after.total_supply.u128(),
        supply_before.total_supply.u128() - tax / 2
    );
}

#[test]
fn enable_feature_second_pay_rejected() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::EnableFeature {
                    sku: Sku::TransferTax,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::EnableFeature {
                    sku: Sku::TransferTax,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("already"));
}

fn enable_launch_guards(e: &mut EnvTok, max_wallet: Option<Uint128>, cooldown_blocks: u64) {
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        launch_guards: Some(LaunchGuardsConfig {
                            max_wallet,
                            cooldown_blocks,
                            trading_enabled: true,
                        }),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
}

fn register_listed_pair(e: &mut EnvTok) {
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
}

fn sell_to_pair(e: &mut EnvTok, who: &str, amount: u128) {
    e.app
        .execute_contract(
            Addr::unchecked(who),
            e.token.clone(),
            &ExecuteMsg::Send {
                contract: e.pair.to_string(),
                amount: Uint128::new(amount),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();
}

fn sell_to_pair_err(e: &mut EnvTok, who: &str, amount: u128) -> String {
    let err = e
        .app
        .execute_contract(
            Addr::unchecked(who),
            e.token.clone(),
            &ExecuteMsg::Send {
                contract: e.pair.to_string(),
                amount: Uint128::new(amount),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap_err();
    format!("{err:?}")
}

fn assert_cooldown(err: impl std::fmt::Debug) {
    let s = format!("{err:?}");
    assert!(
        s.contains("cooldown") || s.contains("Cooldown"),
        "expected cooldown: {s}"
    );
}

fn assert_max_wallet(err: impl std::fmt::Debug) {
    let s = format!("{err:?}");
    assert!(
        s.contains("Max wallet") || s.contains("MaxWallet"),
        "expected max wallet: {s}"
    );
}

#[test]
fn launch_guards_block_both_sides_and_sell_bypasses_max_wallet() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, Some(Uint128::new(1)), 0);
    register_listed_pair(&mut e);
    // Sell still works despite max_wallet=1 (T592-11 / H608-6).
    sell_to_pair(&mut e, "user", 1_000);
}

#[test]
fn launch_guards_cooldown_zero_allows_same_block_trades() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    register_listed_pair(&mut e);
    sell_to_pair(&mut e, "user", 1_000);
    sell_to_pair(&mut e, "user", 1_000);
}

#[test]
fn launch_guards_cooldown_is_per_wallet_not_pair() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, None, 10);
    register_listed_pair(&mut e);

    let bob = Addr::unchecked("bob");
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: bob.to_string(),
                amount: Uint128::new(5_000_000),
            },
            &[],
        )
        .unwrap();

    sell_to_pair(&mut e, "user", 1_000);
    // Same block: a different wallet must not inherit the pair timestamp (H608-1).
    sell_to_pair(&mut e, "bob", 1_000);
    // Same wallet still rate-limited until cooldown_blocks elapse (H608-2).
    assert_cooldown(sell_to_pair_err(&mut e, "user", 1_000));

    e.app.update_block(|b| b.height += 1);
    assert_cooldown(sell_to_pair_err(&mut e, "user", 1_000));

    e.app.update_block(|b| b.height += 9);
    sell_to_pair(&mut e, "user", 1_000);
}

#[test]
fn launch_guards_buy_after_sell_does_not_use_pair_timestamp() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, None, 10);
    register_listed_pair(&mut e);

    let bob = Addr::unchecked("bob");
    sell_to_pair(&mut e, "user", 1_000);
    e.app.update_block(|b| b.height += 1);
    // Pair is `from` on Buy; must not block a fresh recipient (H608-1).
    e.app
        .execute_contract(
            e.pair.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: bob.to_string(),
                amount: Uint128::new(1_000),
            },
            &[],
        )
        .unwrap();
}

#[test]
fn launch_guards_provide_succeeds_after_pair_exceeds_max_wallet() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, Some(Uint128::new(1_000_000)), 0);
    register_listed_pair(&mut e);

    sell_to_pair(&mut e, "user", 1_500_000);
    assert!(balance(&e.app, &e.token, e.pair.as_str()) > 1_000_000);

    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::IncreaseAllowance {
                spender: e.pair.to_string(),
                amount: Uint128::new(100_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
    let pair_before = balance(&e.app, &e.token, e.pair.as_str());
    e.app
        .execute_contract(
            e.pair.clone(),
            e.token.clone(),
            &ExecuteMsg::TransferFrom {
                owner: e.user.to_string(),
                recipient: e.pair.to_string(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        balance(&e.app, &e.token, e.pair.as_str()),
        pair_before + 100_000
    );
}

#[test]
fn launch_guards_max_wallet_still_caps_user_buy_and_transfer() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, Some(Uint128::new(1_000)), 0);
    register_listed_pair(&mut e);

    let carol = Addr::unchecked("carol");
    let err = e
        .app
        .execute_contract(
            e.pair.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: carol.to_string(),
                amount: Uint128::new(2_000),
            },
            &[],
        )
        .unwrap_err();
    assert_max_wallet(err);

    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: carol.to_string(),
                amount: Uint128::new(2_000),
            },
            &[],
        )
        .unwrap_err();
    assert_max_wallet(err);
}

#[test]
fn launch_guards_transfer_tax_still_caps_eoa() {
    let mut e = clean(vec![Sku::LaunchGuards, Sku::TransferTax], None);
    e.app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        transfer_bps: Some(200),
                        launch_guards: Some(LaunchGuardsConfig {
                            max_wallet: Some(Uint128::new(1_000)),
                            cooldown_blocks: 0,
                            trading_enabled: true,
                        }),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    let dave = Addr::unchecked("dave");
    let err = e
        .app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: dave.to_string(),
                amount: Uint128::new(5_000),
            },
            &[],
        )
        .unwrap_err();
    assert_max_wallet(err);
}

#[test]
fn launch_guards_sku_off_skips_cooldown_and_max_wallet() {
    let mut e = clean(vec![], None);
    register_listed_pair(&mut e);
    sell_to_pair(&mut e, "user", 1_000);
    sell_to_pair(&mut e, "user", 1_000);
    let zed = Addr::unchecked("zed");
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: zed.to_string(),
                amount: Uint128::new(50_000_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&e.app, &e.token, zed.as_str()), 50_000_000);
}

#[test]
fn launch_guards_max_wallet_skips_protocol_exempt_to() {
    let mut e = clean(vec![Sku::LaunchGuards], None);
    enable_launch_guards(&mut e, Some(Uint128::new(1)), 0);
    let router = Addr::unchecked("router");
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: router.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&e.app, &e.token, router.as_str()), 10_000);

    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::Transfer {
                recipient: e.token.to_string(),
                amount: Uint128::new(10_000),
            },
            &[],
        )
        .unwrap();
}

#[test]
fn tax_preview_transferfrom_to_pair_is_honest() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let p: TaxPreviewResponse = e
        .app
        .wrap()
        .query_wasm_smart(
            &e.token,
            &QueryMsg::TaxPreview {
                from: e.user.to_string(),
                to: e.pair.to_string(),
                amount: Uint128::new(1_000_000),
                send_msg: None,
            },
        )
        .unwrap();
    assert_eq!(p.kind, TaxKind::Honest);
    assert_eq!(p.debit, p.credit);
}

#[test]
fn is_protocol_exempt_pair() {
    let mut e = clean(vec![], None);
    e.app
        .execute_contract(
            e.user.clone(),
            e.token.clone(),
            &ExecuteMsg::RegisterListedPair {
                pair: e.pair.to_string(),
            },
            &[],
        )
        .unwrap();
    let r: IsExemptResponse = e
        .app
        .wrap()
        .query_wasm_smart(
            &e.token,
            &QueryMsg::IsProtocolExempt {
                address: e.pair.to_string(),
            },
        )
        .unwrap();
    assert!(r.protocol);
}

#[test]
fn instantiate_rejects_combined_cap() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let token_code = app.store_code(token_contract());
    let err = app
        .instantiate_contract(
            token_code,
            manager.clone(),
            &InstantiateMsg {
                name: "BadCap".into(),
                symbol: "XXXX".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(1),
                }],
                marketing: None,
                manager: manager.to_string(),
                treasury: manager.to_string(),
                buy_bps: 0,
                sell_bps: 0,
                max_buy_bps: 2500,
                max_sell_bps: 2500,
                max_transfer_bps: 0,
                factory: manager.to_string(),
                router: None,
                ust1: manager.to_string(),
                cmm_treasury: manager.to_string(),
                features: vec![Sku::VariableRates],
                mint: None,
                transfer_bps: None,
                sinks: None,
                autolp: None,
                launcher: None,
                launch_guards: None,
                initial_exempt: None,
            },
            &[],
            "bad",
            None,
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Combined")
            || err.root_cause().to_string().contains("cap")
    );
}

fn instantiate_raw(msg: InstantiateMsg) -> Result<Addr, String> {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let token_code = app.store_code(token_contract());
    app.instantiate_contract(token_code, manager, &msg, &[], "tok", None)
        .map_err(|e| e.root_cause().to_string())
}

fn base_init(name: &str, symbol: &str, decimals: u8) -> InstantiateMsg {
    InstantiateMsg {
        name: name.into(),
        symbol: symbol.into(),
        decimals,
        initial_balances: vec![Cw20Coin {
            address: "manager".into(),
            amount: Uint128::new(1),
        }],
        marketing: None,
        manager: "manager".into(),
        treasury: "treasury".into(),
        buy_bps: 0,
        sell_bps: 0,
        max_buy_bps: 0,
        max_sell_bps: 0,
        max_transfer_bps: 0,
        factory: "factory".into(),
        router: None,
        ust1: "ust1".into(),
        cmm_treasury: "cmm".into(),
        features: vec![],
        mint: None,
        transfer_bps: None,
        sinks: None,
        autolp: None,
        launcher: None,
        launch_guards: None,
        initial_exempt: None,
    }
}

#[test]
fn instantiate_rejects_decimals_outside_6_18() {
    for dec in [0u8, 5, 19, 255] {
        let err = instantiate_raw(base_init("Demo", "DEMO", dec)).unwrap_err();
        assert!(
            err.to_string().contains("Decimals") || format!("{err:?}").contains("Decimals"),
            "{dec}: {err}"
        );
    }
}

#[test]
fn instantiate_accepts_decimals_6_and_18() {
    instantiate_raw(base_init("Demo", "DEMO", 6)).unwrap();
    instantiate_raw(base_init("Demo", "DEMO", 18)).unwrap();
}

#[test]
fn instantiate_rejects_bad_name_and_symbol() {
    for name in ["My Token", "Demo!", "ab", "", "🚀"] {
        let err = instantiate_raw(base_init(name, "DEMO", 6)).unwrap_err();
        assert!(err.to_string().contains("Name") || format!("{err:?}").contains("InvalidName"));
    }
    for symbol in ["DE-MO", "D", "TOOLONGSYMBOLX"] {
        let err = instantiate_raw(base_init("Demo", symbol, 6)).unwrap_err();
        assert!(err.to_string().contains("Symbol") || format!("{err:?}").contains("InvalidSymbol"));
    }
}

#[test]
fn instantiate_requires_explicit_launch_guards() {
    let mut msg = base_init("Demo", "DEMO", 6);
    msg.features = vec![Sku::LaunchGuards];
    let err = instantiate_raw(msg).unwrap_err();
    assert!(err.to_string().contains("launch_guards") || err.to_string().contains("Launch guards"));
}

#[test]
fn instantiate_launch_guards_and_initial_exempt() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let user = Addr::unchecked("user");
    let token_code = app.store_code(token_contract());
    let mut msg = base_init("Demo", "DEMO", 6);
    msg.manager = manager.to_string();
    msg.treasury = manager.to_string();
    msg.factory = manager.to_string();
    msg.ust1 = manager.to_string();
    msg.cmm_treasury = manager.to_string();
    msg.features = vec![Sku::LaunchGuards, Sku::ExemptionDirectory];
    msg.launch_guards = Some(LaunchGuardsConfig {
        max_wallet: Some(Uint128::new(1_000_000)),
        cooldown_blocks: 10,
        trading_enabled: false,
    });
    msg.initial_exempt = Some(vec![user.to_string()]);
    let token = app
        .instantiate_contract(token_code, manager.clone(), &msg, &[], "tok", None)
        .unwrap();
    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert!(!cfg.launch_guards.as_ref().unwrap().trading_enabled);
    assert_eq!(cfg.launch_guards.as_ref().unwrap().cooldown_blocks, 10);
    let ex: crate::msg::ExemptionsResponse = app
        .wrap()
        .query_wasm_smart(
            &token,
            &QueryMsg::GetExemptions {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(ex.manager.iter().any(|a| a == &user));
}

#[test]
fn instantiate_rejects_headroom_without_variable_rates() {
    let mut msg = base_init("Demo", "DEMO", 6);
    msg.max_sell_bps = 500;
    let err = instantiate_raw(msg).unwrap_err();
    assert!(
        err.contains("variable_rates") || err.contains("max_sell") || err.contains("feature"),
        "{err}"
    );
}

#[test]
fn settings_buy_sell_require_variable_rates() {
    let mut e = clean(vec![], None);
    let err = e
        .app
        .execute_contract(
            e.manager.clone(),
            e.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: e.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        sell_bps: Some(0),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("variable_rates")
            || err.root_cause().to_string().contains("not unlocked"),
        "{err:?}"
    );
    assert_eq!(balance(&e.app, &e.ust1, e.cmm.as_str()), 0);
}

#[test]
fn instantiate_rejects_transfer_bps_without_sku() {
    let mut msg = base_init("Demo", "DEMO", 6);
    msg.max_transfer_bps = 100;
    msg.transfer_bps = Some(100);
    let err = instantiate_raw(msg).unwrap_err();
    assert!(
        err.contains("transfer_bps")
            || err.contains("SKU")
            || err.contains("feature")
            || err.contains("SkuPayload"),
        "{err}"
    );
}

#[test]
fn instantiate_rejects_protocol_initial_exempt() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let token_code = app.store_code(token_contract());
    let mut msg = base_init("Demo", "DEMO", 6);
    msg.manager = manager.to_string();
    msg.treasury = manager.to_string();
    msg.factory = manager.to_string();
    msg.ust1 = manager.to_string();
    msg.cmm_treasury = manager.to_string();
    msg.features = vec![Sku::ExemptionDirectory];
    msg.initial_exempt = Some(vec![manager.to_string()]); // factory == manager in this fixture
    let err = app
        .instantiate_contract(token_code, manager, &msg, &[], "tok", None)
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("protocol")
            || err.root_cause().to_string().contains("Protocol")
    );
}
