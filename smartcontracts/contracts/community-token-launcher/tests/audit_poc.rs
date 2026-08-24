//! Security-audit PoC tests (internal audit, 2026-08-23).
//!
//! Each test is a minimal reproduction of a finding in `audits/INTERNAL_KIMIK3_*.md`.
//! H-1 / M-1 (#605) and H-3 / H-4 (#608) are inverted. C-1 / H-2 (**#606**)
//! are inverted (official launcher Enable Feature + unique SKUs). M-3 / M-2
//! AutoLP pair + skim floor (**#610**) is inverted. Remaining PoCs still
//! demonstrate residuals.

use cosmwasm_std::{to_json_binary, Addr, Binary, Empty, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg, MinterResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use serde::{Deserialize, Serialize};

use cl8y_community_tax_autolp::msg::{
    ConfigResponse as AutoLpConfigResponse, ExecuteMsg as AutoLpExecute,
    InstantiateMsg as AutoLpInstantiate, QueryMsg as AutoLpQuery,
};
use cl8y_community_tax_token::msg::{ConfigResponse as TokenConfigResponse, FeaturesResponse};
use cl8y_community_tax_token::msg::{
    ExecuteMsg as TokenExecute, InstantiateMsg as TokenInstantiate, InvoiceHookMsg as TokenInvoice,
    LaunchGuardsConfig, QueryMsg as TokenQuery, SettingsBatch, Sku,
};
use cl8y_community_token_launcher::msg::{
    CreateTokenMsg, ExecuteMsg as LauncherExecute, InstantiateMsg as LauncherInstantiate,
    InvoiceHookMsg as LauncherInvoice,
};
use dex_common::factory::PairResponse;
use dex_common::pair::Cw20HookMsg as PairHook;
use dex_common::types::{AssetInfo, PairInfo};

const UST1_INVOICE: u128 = 50_000_000;

// ---------- mock pair / factory (same JSON shapes as dex-common) ----------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MockPairQuery {
    Pair {},
}

fn mock_pair_contract() -> Box<dyn Contract<Empty>> {
    fn inst(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        _info: cosmwasm_std::MessageInfo,
        msg: (String, String),
    ) -> StdResult<cosmwasm_std::Response> {
        deps.storage.set(b"token", msg.0.as_bytes());
        deps.storage.set(b"other", msg.1.as_bytes());
        deps.storage.set(b"self", env.contract.address.as_bytes());
        Ok(cosmwasm_std::Response::new())
    }
    fn exec(
        _d: cosmwasm_std::DepsMut,
        _e: cosmwasm_std::Env,
        _i: cosmwasm_std::MessageInfo,
        _m: Empty,
    ) -> StdResult<cosmwasm_std::Response> {
        Ok(cosmwasm_std::Response::new())
    }
    fn query(
        deps: cosmwasm_std::Deps,
        env: cosmwasm_std::Env,
        msg: MockPairQuery,
    ) -> StdResult<Binary> {
        match msg {
            MockPairQuery::Pair {} => {
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
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MockFactoryQuery {
    Pair { asset_infos: [AssetInfo; 2] },
}

fn mock_factory_contract() -> Box<dyn Contract<Empty>> {
    fn inst(
        _d: cosmwasm_std::DepsMut,
        _e: cosmwasm_std::Env,
        _i: cosmwasm_std::MessageInfo,
        _m: Empty,
    ) -> StdResult<cosmwasm_std::Response> {
        Ok(cosmwasm_std::Response::new())
    }
    fn exec(
        deps: cosmwasm_std::DepsMut,
        _e: cosmwasm_std::Env,
        _i: cosmwasm_std::MessageInfo,
        pair: String,
    ) -> StdResult<cosmwasm_std::Response> {
        deps.storage.set(b"pair", pair.as_bytes());
        Ok(cosmwasm_std::Response::new())
    }
    fn query(
        deps: cosmwasm_std::Deps,
        _env: cosmwasm_std::Env,
        msg: MockFactoryQuery,
    ) -> StdResult<Binary> {
        match msg {
            MockFactoryQuery::Pair { .. } => {
                let pair = String::from_utf8(
                    deps.storage
                        .get(b"pair")
                        .ok_or_else(|| cosmwasm_std::StdError::generic_err("pair not listed"))?,
                )
                .unwrap();
                to_json_binary(&PairResponse {
                    pair: PairInfo {
                        asset_infos: [
                            AssetInfo::Token {
                                contract_addr: "x".to_string(),
                            },
                            AssetInfo::Token {
                                contract_addr: "y".to_string(),
                            },
                        ],
                        contract_addr: Addr::unchecked(pair),
                        liquidity_token: Addr::unchecked("lp"),
                    },
                })
            }
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

// ---------- harness ----------

struct Hub {
    app: App,
    ust1: Addr,
    launcher: Addr,
    factory: Addr,
    pair_code: u64,
    token_code: u64,
    autolp_code: u64,
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

fn token_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cl8y_community_tax_token::contract::execute,
        cl8y_community_tax_token::contract::instantiate,
        cl8y_community_tax_token::contract::query,
    ))
}

fn autolp_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            cl8y_community_tax_autolp::contract::execute,
            cl8y_community_tax_autolp::contract::instantiate,
            cl8y_community_tax_autolp::contract::query,
        )
        .with_reply(cl8y_community_tax_autolp::contract::reply),
    )
}

fn launcher_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            cl8y_community_token_launcher::contract::execute,
            cl8y_community_token_launcher::contract::instantiate,
            cl8y_community_token_launcher::contract::query,
        )
        .with_reply(cl8y_community_token_launcher::contract::reply),
    )
}

fn setup(router: Option<&str>) -> Hub {
    let mut app = App::default();
    let admin = Addr::unchecked("admin");
    let cw20_code = app.store_code(cw20_contract());
    let token_code = app.store_code(token_contract());
    let autolp_code = app.store_code(autolp_contract());
    let launcher_code = app.store_code(launcher_contract());
    let pair_code = app.store_code(mock_pair_contract());
    let factory_code = app.store_code(mock_factory_contract());

    let ust1 = app
        .instantiate_contract(
            cw20_code,
            admin.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "UST".into(),
                symbol: "UST".into(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: "manager".into(),
                        amount: Uint128::new(10_000_000_000),
                    },
                    Cw20Coin {
                        address: "alice".into(),
                        amount: Uint128::new(10_000_000_000),
                    },
                    Cw20Coin {
                        address: "bob".into(),
                        amount: Uint128::new(10_000_000_000),
                    },
                ],
                mint: Some(MinterResponse {
                    minter: admin.to_string(),
                    cap: None,
                }),
                marketing: None,
            },
            &[],
            "ust1",
            None,
        )
        .unwrap();

    let factory = app
        .instantiate_contract(factory_code, admin.clone(), &Empty {}, &[], "factory", None)
        .unwrap();

    let launcher = app
        .instantiate_contract(
            launcher_code,
            admin.clone(),
            &LauncherInstantiate {
                token_code_id: token_code,
                autolp_code_id: Some(autolp_code),
                ust1: ust1.to_string(),
                cmm_treasury: "cmm".into(),
                cmm_governance: "cmm".into(),
                factory: factory.to_string(),
                router: router.map(|r| r.to_string()),
            },
            &[],
            "launcher",
            None,
        )
        .unwrap();

    Hub {
        app,
        ust1,
        launcher,
        factory,
        pair_code,
        token_code,
        autolp_code,
    }
}

fn bal(app: &App, token: &Addr, addr: &str) -> u128 {
    app.wrap()
        .query_wasm_smart::<BalanceResponse>(
            token,
            &Cw20QueryMsg::Balance {
                address: addr.into(),
            },
        )
        .unwrap()
        .balance
        .u128()
}

fn create_token_msg(features: Vec<Sku>) -> CreateTokenMsg {
    CreateTokenMsg {
        name: "Community".into(),
        symbol: "COMM".into(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: "manager".into(),
            amount: Uint128::new(1_000_000_000),
        }],
        manager: "manager".into(),
        treasury: "treasury".into(),
        buy_bps: 0,
        sell_bps: 500,
        max_buy_bps: 0,
        max_sell_bps: 500,
        max_transfer_bps: 0,
        features,
        mint: None,
        transfer_bps: None,
        sinks: None,
        launch_guards: None,
        autolp_threshold: None,
        autolp_lp_recipient: None,
        initial_exempt: None,
    }
}

/// Extract the token address from a launcher create tx: the `instantiate`
/// event whose `code_id` matches the token code.
fn token_from_events(res: &cw_multi_test::AppResponse, token_code: u64) -> Addr {
    for e in &res.events {
        if e.ty != "instantiate" {
            continue;
        }
        let mut code = None;
        let mut addr = None;
        for a in &e.attributes {
            if a.key == "code_id" {
                code = Some(a.value.clone());
            }
            if a.key == "_contract_address" {
                addr = Some(a.value.clone());
            }
        }
        if code.as_deref() == Some(token_code.to_string().as_str()) {
            if let Some(a) = addr {
                return Addr::unchecked(a);
            }
        }
    }
    panic!("token instantiate event not found: {:?}", res.events);
}

fn create_paid(hub: &mut Hub, payer: &str, msg: CreateTokenMsg, pay: u128) -> Addr {
    let hook = to_json_binary(&LauncherInvoice::CreateToken(Box::new(msg))).unwrap();
    let res = hub
        .app
        .execute_contract(
            Addr::unchecked(payer),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: hub.launcher.to_string(),
                amount: Uint128::new(pay),
                msg: hook,
            },
            &[],
        )
        .unwrap();
    token_from_events(&res, hub.token_code)
}

fn create_free(hub: &mut Hub, payer: &str, msg: CreateTokenMsg) -> Addr {
    let res = hub
        .app
        .execute_contract(
            Addr::unchecked(payer),
            hub.launcher.clone(),
            &LauncherExecute::CreateToken(Box::new(msg)),
            &[],
        )
        .unwrap();
    token_from_events(&res, hub.token_code)
}

fn register_pair(hub: &mut Hub, token: &Addr) -> Addr {
    let pair = hub
        .app
        .instantiate_contract(
            hub.pair_code,
            Addr::unchecked("admin"),
            &(token.to_string(), hub.ust1.to_string()),
            &[],
            "pair",
            None,
        )
        .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("admin"),
            hub.factory.clone(),
            &pair.to_string(),
            &[],
        )
        .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            token.clone(),
            &TokenExecute::RegisterListedPair {
                pair: pair.to_string(),
            },
            &[],
        )
        .unwrap();
    pair
}

fn swap_hook() -> Binary {
    to_json_binary(&PairHook::Swap {
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

fn swap_hook_trader(trader: &str) -> Binary {
    to_json_binary(&PairHook::Swap {
        belief_price: None,
        max_spread: None,
        min_return: None,
        to: None,
        deadline: None,
        trader: Some(trader.into()),
        hybrid: None,
    })
    .unwrap()
}

fn default_instantiate(
    hub: &Hub,
    guards: Option<LaunchGuardsConfig>,
    sell_bps: u16,
    buy_bps: u16,
) -> TokenInstantiate {
    TokenInstantiate {
        name: "Community".into(),
        symbol: "COMM".into(),
        decimals: 6,
        initial_balances: vec![
            Cw20Coin {
                address: "router".into(),
                amount: Uint128::new(1_000_000),
            },
            Cw20Coin {
                address: "alice".into(),
                amount: Uint128::new(10_000_000),
            },
            Cw20Coin {
                address: "bob".into(),
                amount: Uint128::new(10_000_000),
            },
        ],
        marketing: None,
        manager: "manager".into(),
        treasury: "treasury".into(),
        buy_bps,
        sell_bps,
        max_buy_bps: buy_bps,
        max_sell_bps: sell_bps,
        max_transfer_bps: 0,
        factory: hub.factory.to_string(),
        router: Some("router".into()),
        ust1: hub.ust1.to_string(),
        cmm_treasury: "cmm".into(),
        features: if guards.is_some() {
            vec![Sku::LaunchGuards]
        } else {
            vec![]
        },
        mint: None,
        transfer_bps: None,
        sinks: None,
        autolp: None,
        launcher: None,
        launch_guards: guards,
        initial_exempt: None,
    }
}

// ============================================================================
// PoC 1 (C-1): inverted (#606) — official launcher Enable Feature succeeds.
// ============================================================================
#[test]
fn poc_launcher_enable_feature_always_unauthorized() {
    let mut hub = setup(Some("router"));
    let token = create_free(&mut hub, "manager", create_token_msg(vec![]));
    let cmm_before = bal(&hub.app, &hub.ust1, "cmm");

    let hook = to_json_binary(&LauncherInvoice::EnableFeature {
        token: token.to_string(),
        sku: Sku::TransferTax,
    })
    .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: hub.launcher.to_string(),
                amount: Uint128::new(UST1_INVOICE),
                msg: hook,
            },
            &[],
        )
        .unwrap();

    let feats: FeaturesResponse = hub
        .app
        .wrap()
        .query_wasm_smart(token.clone(), &TokenQuery::GetFeatures {})
        .unwrap();
    assert!(feats.transfer_tax);
    assert_eq!(
        bal(&hub.app, &hub.ust1, "cmm") - cmm_before,
        UST1_INVOICE,
        "CMM must receive the 50 UST1 invoice"
    );

    // Non-manager cannot unlock via launcher (T606-2).
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: "attacker".into(),
                amount: Uint128::new(UST1_INVOICE),
            },
            &[],
        )
        .unwrap();
    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("attacker"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: hub.launcher.to_string(),
                amount: Uint128::new(UST1_INVOICE),
                msg: to_json_binary(&LauncherInvoice::EnableFeature {
                    token: token.to_string(),
                    sku: Sku::VariableRates,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Unauthorized"),
        "non-manager launcher EnableFeature: {err:?}"
    );
    let feats: FeaturesResponse = hub
        .app
        .wrap()
        .query_wasm_smart(token, &TokenQuery::GetFeatures {})
        .unwrap();
    assert!(!feats.variable_rates);
}

// ============================================================================
// PoC 2 (H-2): inverted (#606) — duplicate SKUs are rejected; fee not kept.
// ============================================================================
#[test]
fn poc_launcher_duplicate_sku_double_charge() {
    let mut hub = setup(Some("router"));
    let cmm_before = bal(&hub.app, &hub.ust1, "cmm");
    let msg = create_token_msg(vec![Sku::TransferTax, Sku::TransferTax]);
    let hook = to_json_binary(&LauncherInvoice::CreateToken(Box::new(msg))).unwrap();
    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: hub.launcher.to_string(),
                amount: Uint128::new(UST1_INVOICE * 2),
                msg: hook,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Duplicate SKU"),
        "expected DuplicateSku, got: {err:?}"
    );
    assert_eq!(
        bal(&hub.app, &hub.ust1, "cmm"),
        cmm_before,
        "duplicate SKU must not take the invoice"
    );

    let token = create_paid(
        &mut hub,
        "manager",
        create_token_msg(vec![Sku::TransferTax, Sku::VariableRates]),
        UST1_INVOICE * 2,
    );
    let feats: FeaturesResponse = hub
        .app
        .wrap()
        .query_wasm_smart(token, &TokenQuery::GetFeatures {})
        .unwrap();
    assert!(feats.transfer_tax && feats.variable_rates);
    assert_eq!(
        bal(&hub.app, &hub.ust1, "cmm") - cmm_before,
        UST1_INVOICE * 2
    );
}

// ============================================================================
// PoC 3 (H-1): AutoV2Lp SKU is payable at create but AutoLP is never bound,
// and no on-chain path can bind it later. Tracked on #605 (not a new issue).
// ============================================================================
#[test]
fn poc_autov2lp_paid_but_never_bound() {
    let mut hub = setup(Some("router"));
    let mut msg = create_token_msg(vec![Sku::AutoV2Lp]);
    msg.autolp_threshold = Some(Uint128::new(1_000_000));
    msg.autolp_lp_recipient = Some("manager".into());
    let token = create_paid(&mut hub, "manager", msg, UST1_INVOICE);

    let cfg: TokenConfigResponse = hub
        .app
        .wrap()
        .query_wasm_smart(token.clone(), &TokenQuery::GetConfig {})
        .unwrap();
    assert!(
        cfg.autolp.is_some(),
        "AutoLP SKU instantiates and binds sister when autolp_code_id is set"
    );
}

// ============================================================================
// T592-13 / #607 improved option 2 (was audit C-2 / option 1 Honest hops).
// Official router Send+Swap extra-debits authenticated Swap.trader; missing
// trader fail-closes. Pair→router stays 1:1; router→user is buy outbound split.
// Address named "router" (PROTOCOL_EXEMPT + config.router), not router wasm.
// ============================================================================
#[test]
fn poc_router_exemption_full_tax_bypass() {
    let mut hub = setup(Some("router"));
    let token = hub
        .app
        .instantiate_contract(
            hub.token_code,
            Addr::unchecked("admin"),
            &default_instantiate(&hub, None, 500, 500),
            &[],
            "token",
            None,
        )
        .unwrap();
    let pair = register_pair(&mut hub, &token);

    // Evasion: router Send+Swap without trader must not skip tax.
    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("router"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("trusted non-exempt trader"),
        "{err:?}"
    );

    // Sell via router with authenticated trader (official router wasm sets this).
    hub.app
        .execute_contract(
            Addr::unchecked("router"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook_trader("alice"),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        bal(&hub.app, &token, "router"),
        900_000,
        "router debited declared amount only"
    );
    assert_eq!(bal(&hub.app, &token, pair.as_str()), 100_000);
    assert_eq!(
        bal(&hub.app, &token, "alice"),
        10_000_000 - 5_000,
        "alice extra-debit 5% sell tax"
    );
    assert_eq!(
        bal(&hub.app, &token, "treasury"),
        5_000,
        "sell tax collected on router hop"
    );

    // Pair-direct EOA sell still extra-debits the sender (spoofed trader ignored).
    hub.app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook_trader("bob"),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        bal(&hub.app, &token, "alice"),
        10_000_000 - 5_000 - 105_000,
        "EOA paid 5% sell tax on pair-direct"
    );
    assert_eq!(
        bal(&hub.app, &token, "bob"),
        10_000_000,
        "spoofed trader not debited"
    );

    // Buy leg: pair → router 1:1; router → user outbound split.
    hub.app
        .execute_contract(
            Addr::unchecked(pair.clone()),
            token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: "router".into(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        bal(&hub.app, &token, "router"),
        900_000 + 100_000,
        "router buy credit 1:1"
    );
    hub.app
        .execute_contract(
            Addr::unchecked("router"),
            token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: "bob".into(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        bal(&hub.app, &token, "bob"),
        10_000_000 + 95_000,
        "user received amount minus 5% buy tax"
    );
}

// ============================================================================
// PoC 5 (H-3 / #608 inverted): cooldown is per user wallet. A second wallet
// can trade after alice; the same wallet still hits cooldown (H608-1 / H608-2).
// ============================================================================
#[test]
fn poc_cooldown_bricks_pair() {
    let mut hub = setup(Some("router"));
    let token = hub
        .app
        .instantiate_contract(
            hub.token_code,
            Addr::unchecked("admin"),
            &default_instantiate(
                &hub,
                Some(LaunchGuardsConfig {
                    max_wallet: None,
                    cooldown_blocks: 10,
                    trading_enabled: true,
                }),
                500,
                0,
            ),
            &[],
            "token",
            None,
        )
        .unwrap();
    let pair = register_pair(&mut hub, &token);

    hub.app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();

    hub.app.update_block(|b| b.height += 1);
    hub.app
        .execute_contract(
            Addr::unchecked("bob"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();

    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("cooldown active"),
        "same wallet still rate-limited: {err:?}"
    );
}

// ============================================================================
// PoC 6 (H-4 / #608 inverted): max_wallet skips listed-pair `to`, so provide
// TransferFrom succeeds after sells grow the pair above the cap (H608-4).
// Stand-in is pair.TransferFrom, not pair ProvideLiquidity wasm.
// ============================================================================
#[test]
fn poc_max_wallet_bricks_provide() {
    let mut hub = setup(Some("router"));
    let token = hub
        .app
        .instantiate_contract(
            hub.token_code,
            Addr::unchecked("admin"),
            &default_instantiate(
                &hub,
                Some(LaunchGuardsConfig {
                    max_wallet: Some(Uint128::new(1_000_000)),
                    cooldown_blocks: 0,
                    trading_enabled: true,
                }),
                0,
                0,
            ),
            &[],
            "token",
            None,
        )
        .unwrap();
    let pair = register_pair(&mut hub, &token);

    // Sell bypasses max_wallet (T592-11 / H608-6): pair balance climbs above the cap.
    hub.app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(1_500_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(bal(&hub.app, &token, pair.as_str()), 1_500_000);

    hub.app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::IncreaseAllowance {
                spender: pair.to_string(),
                amount: Uint128::new(100_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked(pair.clone()),
            token.clone(),
            &Cw20ExecuteMsg::TransferFrom {
                owner: "alice".into(),
                recipient: pair.to_string(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();
    assert_eq!(bal(&hub.app, &token, pair.as_str()), 1_600_000);
}

// ============================================================================
// PoC 7 (H-5): trading_enabled=false reverts pair→EOA Transfer (Buy).
// Inferred withdraw/cancel/claim lock via T592-7 classify; mock pair has no
// book. Documented residual (T592-11 / 11611 A9/E8) — do not file as a new bug.
// ============================================================================
#[test]
fn poc_trading_disabled_locks_withdrawals() {
    let mut hub = setup(Some("router"));
    let token = hub
        .app
        .instantiate_contract(
            hub.token_code,
            Addr::unchecked("admin"),
            &default_instantiate(
                &hub,
                Some(LaunchGuardsConfig {
                    max_wallet: None,
                    cooldown_blocks: 0,
                    trading_enabled: true,
                }),
                500,
                500,
            ),
            &[],
            "token",
            None,
        )
        .unwrap();
    let pair = register_pair(&mut hub, &token);

    hub.app
        .execute_contract(
            Addr::unchecked("alice"),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount: Uint128::new(500_000),
                msg: swap_hook(),
            },
            &[],
        )
        .unwrap();

    let settings_hook = to_json_binary(&TokenInvoice::UpdateSettings {
        settings: SettingsBatch {
            launch_guards: Some(LaunchGuardsConfig {
                max_wallet: None,
                cooldown_blocks: 0,
                trading_enabled: false,
            }),
            ..Default::default()
        },
    })
    .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: token.to_string(),
                amount: Uint128::new(UST1_INVOICE),
                msg: settings_hook,
            },
            &[],
        )
        .unwrap();

    let err = hub
        .app
        .execute_contract(
            Addr::unchecked(pair.clone()),
            token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: "alice".into(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Trading is not enabled"),
        "withdrawal locked while trading disabled: {err:?}"
    );
}

// ============================================================================
// PoC 8 (M-1 / #605 inverted): settings buy/sell require VariableRates SKU.
// ============================================================================
#[test]
fn poc_variable_rates_sku_is_theater() {
    let mut hub = setup(Some("router"));
    let mut msg = create_token_msg(vec![]);
    msg.sell_bps = 0;
    msg.max_sell_bps = 0;
    let token = create_free(&mut hub, "manager", msg);

    let hook = to_json_binary(&TokenInvoice::UpdateSettings {
        settings: SettingsBatch {
            sell_bps: Some(500),
            ..Default::default()
        },
    })
    .unwrap();
    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: token.to_string(),
                amount: Uint128::new(UST1_INVOICE),
                msg: hook,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("variable_rates")
            || format!("{err:?}").contains("not unlocked"),
        "settings buy/sell require VariableRates SKU: {err:?}"
    );
}

// ============================================================================
// PoC 9 (M-3 / #610 inverted): AutoLP `pair` must be factory-listed with this
// tax token. Fake / wrong-token pointers revert on set, not only on skim.
// ============================================================================
#[test]
fn poc_autolp_manager_can_skim_to_fake_pair() {
    let mut hub = setup(Some("router"));
    let token = hub
        .app
        .instantiate_contract(
            hub.token_code,
            Addr::unchecked("admin"),
            &default_instantiate(&hub, None, 500, 500),
            &[],
            "token",
            None,
        )
        .unwrap();

    let fake_pair = hub
        .app
        .instantiate_contract(
            hub.pair_code,
            Addr::unchecked("manager"),
            &(token.to_string(), hub.ust1.to_string()),
            &[],
            "fake-pair",
            None,
        )
        .unwrap();

    let err = hub
        .app
        .instantiate_contract(
            hub.autolp_code,
            Addr::unchecked("admin"),
            &AutoLpInstantiate {
                token: token.to_string(),
                manager: "manager".into(),
                factory: hub.factory.to_string(),
                router: Some("router".into()),
                pair: Some(fake_pair.to_string()),
                quote_token: None,
                threshold: Uint128::new(1_000_000),
                lp_recipient: "manager".into(),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
            "autolp-fake",
            None,
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("not factory-listed")
            || format!("{err:?}").contains("PairNotListed")
            || format!("{err:?}").contains("pair not listed"),
        "fake pair must revert on instantiate: {err:?}"
    );

    let autolp = hub
        .app
        .instantiate_contract(
            hub.autolp_code,
            Addr::unchecked("admin"),
            &AutoLpInstantiate {
                token: token.to_string(),
                manager: "manager".into(),
                factory: hub.factory.to_string(),
                router: Some("router".into()),
                pair: None,
                quote_token: None,
                threshold: Uint128::new(1_000_000),
                lp_recipient: "manager".into(),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
            "autolp",
            None,
        )
        .unwrap();

    let err = hub
        .app
        .execute_contract(
            Addr::unchecked("manager"),
            autolp.clone(),
            &AutoLpExecute::UpdateConfig {
                pair: Some(fake_pair.to_string()),
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("not factory-listed")
            || format!("{err:?}").contains("PairNotListed")
            || format!("{err:?}").contains("pair not listed"),
        "fake pair must revert on UpdateConfig: {err:?}"
    );

    let listed = hub
        .app
        .instantiate_contract(
            hub.pair_code,
            Addr::unchecked("manager"),
            &(token.to_string(), hub.ust1.to_string()),
            &[],
            "listed-pair",
            None,
        )
        .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            hub.factory.clone(),
            &listed.to_string(),
            &[],
        )
        .unwrap();
    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            autolp.clone(),
            &AutoLpExecute::UpdateConfig {
                pair: Some(listed.to_string()),
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap();

    hub.app
        .execute_contract(
            Addr::unchecked("manager"),
            autolp.clone(),
            &AutoLpExecute::UpdateConfig {
                pair: None,
                router: None,
                quote_token: None,
                threshold: Some(Uint128::new(5_000_000)),
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap();
    let cfg: AutoLpConfigResponse = hub
        .app
        .wrap()
        .query_wasm_smart(autolp.clone(), &AutoLpQuery::GetConfig {})
        .unwrap();
    assert_eq!(cfg.threshold, Uint128::new(5_000_000));
    assert_eq!(cfg.pair, Some(listed.clone()));
    assert_eq!(cfg.factory, hub.factory);
    assert!(cfg.skim_max_spread > cosmwasm_std::Decimal::zero());
}
