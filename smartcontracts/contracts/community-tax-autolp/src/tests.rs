use cosmwasm_std::{to_json_binary, Addr, Binary, Decimal, Empty, StdResult, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dex_common::factory::{PairResponse, QueryMsg as FactoryQuery};
use dex_common::pair::Cw20HookMsg;
use dex_common::types::{Asset, AssetInfo, PairInfo};
use serde::{Deserialize, Serialize};

use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::spread::default_skim_max_spread;

fn autolp_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply),
    )
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

/// Tax-side mock: cw20-base plus `RegisterListedPair` (#633 / R633-3).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum TaxTokenExec {
    Register {
        register_listed_pair: RegisterPairInner,
    },
    Cw20(cw20_base::msg::ExecuteMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RegisterPairInner {
    pair: String,
}

fn tax_token_contract() -> Box<dyn Contract<Empty>> {
    fn exec(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        msg: TaxTokenExec,
    ) -> StdResult<cosmwasm_std::Response> {
        match msg {
            TaxTokenExec::Register {
                register_listed_pair,
            } => {
                let already = deps.storage.get(b"listed").is_some();
                deps.storage
                    .set(b"listed", register_listed_pair.pair.as_bytes());
                Ok(cosmwasm_std::Response::new()
                    .add_attribute("action", "register_listed_pair")
                    .add_attribute("already", if already { "true" } else { "false" }))
            }
            TaxTokenExec::Cw20(inner) => cw20_base::contract::execute(deps, env, info, inner)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string())),
        }
    }
    fn inst(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        msg: cw20_base::msg::InstantiateMsg,
    ) -> StdResult<cosmwasm_std::Response> {
        cw20_base::contract::instantiate(deps, env, info, msg)
            .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))
    }
    fn query(
        deps: cosmwasm_std::Deps,
        env: cosmwasm_std::Env,
        msg: cw20_base::msg::QueryMsg,
    ) -> StdResult<Binary> {
        cw20_base::contract::query(deps, env, msg)
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum MockPairQuery {
    Pair {},
    LastHook {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum MockPairExecute {
    Receive(Cw20ReceiveMsg),
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        deadline: Option<u64>,
    },
    /// Test helper: quote raw the swap will pay. Below this, `min_return` fails.
    SetFloorOutput {
        amount: Uint128,
    },
    /// Test helper: force the next swap to fail as a sandwich.
    Tilt {},
}

fn mock_pair_contract() -> Box<dyn Contract<Empty>> {
    fn inst(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        _info: cosmwasm_std::MessageInfo,
        msg: (String, String, String),
    ) -> StdResult<cosmwasm_std::Response> {
        deps.storage.set(b"token", msg.0.as_bytes());
        deps.storage.set(b"other", msg.1.as_bytes());
        deps.storage.set(b"quote", msg.2.as_bytes());
        deps.storage.set(b"self", env.contract.address.as_bytes());
        deps.storage.set(b"floor", &u128::MAX.to_be_bytes());
        Ok(cosmwasm_std::Response::new())
    }
    fn exec(
        deps: cosmwasm_std::DepsMut,
        _env: cosmwasm_std::Env,
        _info: cosmwasm_std::MessageInfo,
        msg: MockPairExecute,
    ) -> StdResult<cosmwasm_std::Response> {
        match msg {
            MockPairExecute::SetFloorOutput { amount } => {
                deps.storage.set(b"floor", &amount.u128().to_be_bytes());
                Ok(cosmwasm_std::Response::new())
            }
            MockPairExecute::Tilt {} => {
                deps.storage.set(b"tilt", b"1");
                Ok(cosmwasm_std::Response::new())
            }
            MockPairExecute::Receive(rcv) => {
                if deps.storage.get(b"tilt").is_some() {
                    return Err(cosmwasm_std::StdError::generic_err("max spread exceeded"));
                }
                let hook: Cw20HookMsg = cosmwasm_std::from_json(&rcv.msg)?;
                deps.storage.set(b"hook", &to_json_binary(&hook)?);
                match hook {
                    Cw20HookMsg::Swap {
                        max_spread,
                        min_return,
                        to,
                        ..
                    } => {
                        if max_spread.is_none() && min_return.is_none() {
                            return Err(cosmwasm_std::StdError::generic_err("skim floor missing"));
                        }
                        let floor = deps
                            .storage
                            .get(b"floor")
                            .and_then(|b| <[u8; 16]>::try_from(b).ok())
                            .map(|b| Uint128::from(u128::from_be_bytes(b)))
                            .unwrap_or(Uint128::new(u128::MAX));
                        if let Some(min) = min_return {
                            if min > floor {
                                return Err(cosmwasm_std::StdError::generic_err(
                                    "min_return violated",
                                ));
                            }
                        }
                        let quote = String::from_utf8(deps.storage.get(b"quote").unwrap()).unwrap();
                        let dest = to.unwrap_or(rcv.sender);
                        if !quote.is_empty() && !floor.is_zero() && floor != Uint128::new(u128::MAX)
                        {
                            let pay = WasmMsgExec {
                                contract_addr: quote,
                                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: dest,
                                    amount: floor,
                                })?,
                            };
                            Ok(cosmwasm_std::Response::new().add_message(
                                cosmwasm_std::CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                                    contract_addr: pay.contract_addr,
                                    msg: pay.msg,
                                    funds: vec![],
                                }),
                            ))
                        } else {
                            Ok(cosmwasm_std::Response::new())
                        }
                    }
                    _ => Ok(cosmwasm_std::Response::new()),
                }
            }
            MockPairExecute::ProvideLiquidity { .. } => {
                deps.storage.set(b"provided", b"1");
                Ok(cosmwasm_std::Response::new())
            }
        }
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
            MockPairQuery::LastHook {} => {
                let raw = deps.storage.get(b"hook").unwrap_or_default();
                Ok(Binary::from(raw))
            }
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

struct WasmMsgExec {
    contract_addr: String,
    msg: Binary,
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
        msg: FactoryQuery,
    ) -> StdResult<Binary> {
        match msg {
            FactoryQuery::Pair { .. } => {
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
            _ => Err(cosmwasm_std::StdError::generic_err("unsupported")),
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

struct Harness {
    app: App,
    manager: Addr,
    token: Addr,
    quote: Addr,
    factory: Addr,
    autolp: Addr,
    pair: Addr,
    pair_code: u64,
}

fn setup(pair_at_init: bool) -> Harness {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let token_code = app.store_code(tax_token_contract());
    let quote_code = app.store_code(cw20_contract());
    let autolp_code = app.store_code(autolp_contract());
    let pair_code = app.store_code(mock_pair_contract());
    let factory_code = app.store_code(mock_factory_contract());

    let token = app
        .instantiate_contract(
            token_code,
            manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "TAX".into(),
                symbol: "TAX".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(10_000_000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "tax",
            None,
        )
        .unwrap();
    let quote = app
        .instantiate_contract(
            quote_code,
            manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "Quote".into(),
                symbol: "QTE".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(10_000_000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "quote",
            None,
        )
        .unwrap();
    let factory = app
        .instantiate_contract(
            factory_code,
            manager.clone(),
            &Empty {},
            &[],
            "factory",
            None,
        )
        .unwrap();
    let pair = app
        .instantiate_contract(
            pair_code,
            manager.clone(),
            &(token.to_string(), quote.to_string(), quote.to_string()),
            &[],
            "pair",
            None,
        )
        .unwrap();
    app.execute_contract(manager.clone(), factory.clone(), &pair.to_string(), &[])
        .unwrap();

    let autolp = app
        .instantiate_contract(
            autolp_code,
            manager.clone(),
            &InstantiateMsg {
                token: token.to_string(),
                manager: manager.to_string(),
                factory: factory.to_string(),
                router: None,
                pair: if pair_at_init {
                    Some(pair.to_string())
                } else {
                    None
                },
                quote_token: None,
                threshold: Uint128::new(1_000_000),
                lp_recipient: manager.to_string(),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
            "autolp",
            None,
        )
        .unwrap();

    Harness {
        app,
        manager,
        token,
        quote,
        factory,
        autolp,
        pair,
        pair_code,
    }
}

fn bal(app: &App, token: &Addr, owner: &str) -> u128 {
    let r: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    r.balance.u128()
}

fn cfg(h: &Harness) -> ConfigResponse {
    h.app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetConfig {})
        .unwrap()
}

#[test]
fn instantiate_without_pair_skips_until_set() {
    let mut h = setup(false);
    assert!(cfg(&h).pair.is_none());
    assert_eq!(cfg(&h).factory, h.factory);
    assert_eq!(cfg(&h).skim_max_spread, default_skim_max_spread());
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(2_000_000),
            },
            &[],
        )
        .unwrap();
    let err = h
        .app
        .execute_contract(
            Addr::unchecked("anyone"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("not configured"));
}

#[test]
fn update_config_manager_only() {
    let mut h = setup(false);
    let err = h
        .app
        .execute_contract(
            Addr::unchecked("other"),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.pair.to_string()),
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
    assert!(err.root_cause().to_string().contains("Unauthorized"));
}

#[test]
fn set_valid_factory_pair_token_as_asset0_or_1() {
    let mut h = setup(false);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.pair.to_string()),
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
    let c = cfg(&h);
    assert_eq!(c.pair, Some(h.pair.clone()));
    assert_eq!(c.quote_token, Some(h.quote.clone()));

    // Asset order flipped still lists.
    let flipped = h
        .app
        .instantiate_contract(
            h.pair_code,
            h.manager.clone(),
            &(
                h.quote.to_string(),
                h.token.to_string(),
                h.quote.to_string(),
            ),
            &[],
            "pair-flip",
            None,
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.factory.clone(),
            &flipped.to_string(),
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(flipped.to_string()),
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
    assert_eq!(cfg(&h).pair, Some(flipped));
}

#[test]
fn set_pair_missing_tax_token_reverts() {
    let mut h = setup(false);
    let other_a = Addr::unchecked("terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let other_b = Addr::unchecked("terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let wrong = h
        .app
        .instantiate_contract(
            h.pair_code,
            h.manager.clone(),
            &(other_a.to_string(), other_b.to_string(), "-".to_string()),
            &[],
            "wrong-pair",
            None,
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.factory.clone(),
            &wrong.to_string(),
            &[],
        )
        .unwrap();
    let err = h
        .app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(wrong.to_string()),
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
        err.root_cause().to_string().contains("not factory-listed")
            || err.root_cause().to_string().contains("does not hold"),
        "{err:?}"
    );
}

#[test]
fn set_random_contract_reverts() {
    let mut h = setup(false);
    let err = h
        .app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.token.to_string()),
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
        err.root_cause().to_string().contains("not factory-listed")
            || err.root_cause().to_string().contains("Error"),
        "{err:?}"
    );
}

#[test]
fn instantiate_fake_pair_reverts() {
    let mut h = setup(false);
    let code = h.app.store_code(autolp_contract());
    let fake = h
        .app
        .instantiate_contract(
            h.pair_code,
            h.manager.clone(),
            &(h.token.to_string(), h.quote.to_string(), "-".to_string()),
            &[],
            "fake",
            None,
        )
        .unwrap();
    // Factory still lists the real pair, not `fake`.
    let err = h
        .app
        .instantiate_contract(
            code,
            h.manager.clone(),
            &InstantiateMsg {
                token: h.token.to_string(),
                manager: h.manager.to_string(),
                factory: h.factory.to_string(),
                router: None,
                pair: Some(fake.to_string()),
                quote_token: None,
                threshold: Uint128::new(1),
                lp_recipient: h.manager.to_string(),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
            "autolp-fake",
            None,
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("not factory-listed")
            || format!("{err:?}").contains("PairNotListed"),
        "{err:?}"
    );
}

#[test]
fn omitted_update_fields_merge() {
    let mut h = setup(true);
    let before = cfg(&h);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
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
    let after = cfg(&h);
    assert_eq!(after.threshold, Uint128::new(5_000_000));
    assert_eq!(after.pair, before.pair);
    assert_eq!(after.factory, before.factory);
    assert_eq!(after.skim_max_spread, before.skim_max_spread);
}

#[test]
fn skim_below_threshold_is_noop() {
    let mut h = setup(true);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(10),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            Addr::unchecked("anyone"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 10);
    assert!(!cfg(&h).skimming);
}

#[test]
fn skim_sets_max_spread_floor_and_permissionless() {
    let mut h = setup(true);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(2_000_000),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
    let hook: Cw20HookMsg = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &MockPairQuery::LastHook {})
        .unwrap();
    match hook {
        Cw20HookMsg::Swap {
            max_spread,
            min_return,
            ..
        } => {
            assert_eq!(max_spread, Some(default_skim_max_spread()));
            assert!(min_return.is_none());
        }
        other => panic!("expected swap hook, got {other:?}"),
    }
    assert!(!cfg(&h).skimming);
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 1_000_000);
}

#[test]
fn skim_floor_violation_reverts_and_keeps_tax() {
    let mut h = setup(true);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: None,
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: Some(Uint128::new(500_000)),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.pair.clone(),
            &MockPairExecute::SetFloorOutput {
                amount: Uint128::new(1),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(2_000_000),
            },
            &[],
        )
        .unwrap();
    let err = h
        .app
        .execute_contract(
            Addr::unchecked("anyone"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("min_return") || format!("{err:?}").contains("spread"),
        "{err:?}"
    );
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 2_000_000);
    assert!(!cfg(&h).skimming);
}

#[test]
fn sandwich_tilt_reverts_skim() {
    let mut h = setup(true);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.pair.clone(),
            &MockPairExecute::Tilt {},
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(2_000_000),
            },
            &[],
        )
        .unwrap();
    let err = h
        .app
        .execute_contract(
            Addr::unchecked("attacker"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(format!("{err:?}").contains("spread"), "{err:?}");
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 2_000_000);
}

#[test]
fn manager_cannot_loosen_spread_past_cap() {
    let mut h = setup(false);
    let err = h
        .app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: None,
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
                skim_max_spread: Some(Decimal::percent(3)),
                skim_min_return: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("200 bps"), "{err:?}");
}

#[test]
fn skim_then_provide_when_quote_leg_present() {
    let mut h = setup(true);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.quote.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.pair.to_string(),
                amount: Uint128::new(400_000),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.pair.clone(),
            &MockPairExecute::SetFloorOutput {
                amount: Uint128::new(400_000),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(2_000_000),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            Addr::unchecked("anyone"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
    assert!(
        bal(&h.app, &h.quote, h.autolp.as_str()) == 0
            || bal(&h.app, &h.token, h.autolp.as_str()) < 2_000_000
    );
    assert!(!cfg(&h).skimming);
}

fn listed_pair(h: &Harness) -> Option<String> {
    h.app
        .wrap()
        .query_wasm_raw(h.token.clone(), b"listed".as_slice())
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
}

#[test]
fn set_pair_registers_token_listed_pair() {
    let mut h = setup(false);
    assert!(listed_pair(&h).is_none());
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.pair.to_string()),
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
    assert_eq!(listed_pair(&h).as_deref(), Some(h.pair.as_str()));
    // Re-bind same pair is idempotent (token already: true).
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.pair.to_string()),
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
    assert_eq!(listed_pair(&h).as_deref(), Some(h.pair.as_str()));
}

#[test]
fn instantiate_with_pair_registers_token() {
    let h = setup(true);
    assert_eq!(listed_pair(&h).as_deref(), Some(h.pair.as_str()));
}
