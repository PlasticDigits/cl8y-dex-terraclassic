//! Multitest: foreign adopt onto community-tax-token (GitLab #626).

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{
    AdoptMigrateMsg, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, MigrateOriginResponse,
    QueryMsg,
};

const GENESIS: u128 = 1_000_000_000;

fn token_with_migrate() -> Box<dyn Contract<Empty>> {
    let exec = |deps: DepsMut, env: Env, info: MessageInfo, msg: crate::msg::ExecuteMsg| {
        crate::contract::execute(deps, env, info, msg)
    };
    let inst = |deps: DepsMut, env: Env, info: MessageInfo, msg: InstantiateMsg| {
        crate::contract::instantiate(deps, env, info, msg)
    };
    let q = |deps: Deps, env: Env, msg: QueryMsg| crate::contract::query(deps, env, msg);
    let mig = |deps: DepsMut, env: Env, msg: MigrateMsg| crate::contract::migrate(deps, env, msg);
    Box::new(ContractWrapper::new(exec, inst, q).with_migrate(mig))
}

#[cosmwasm_schema::cw_serde]
struct SourceInit {
    name: String,
    symbol: String,
    decimals: u8,
    initial_balances: Vec<Cw20Coin>,
    mint: Option<MinterResponse>,
    cw2_name: String,
    write_tax_map: bool,
    write_snapshot: bool,
}

fn source_fixture() -> Box<dyn Contract<Empty>> {
    fn inst(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: SourceInit,
    ) -> StdResult<Response> {
        cw20_base::contract::instantiate(
            deps.branch(),
            env,
            info,
            Cw20InstantiateMsg {
                name: msg.name,
                symbol: msg.symbol,
                decimals: msg.decimals,
                initial_balances: msg.initial_balances,
                mint: msg.mint,
                marketing: None,
            },
        )
        .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
        set_contract_version(deps.storage, &msg.cw2_name, "0.0.0")
            .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;
        if msg.write_tax_map {
            deps.storage.set(b"tax_map", b"1");
        }
        if msg.write_snapshot {
            deps.storage.set(b"balance_at", b"leftover");
        }
        Ok(Response::new())
    }
    fn exec(_d: DepsMut, _e: Env, _i: MessageInfo, _m: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }
    fn query(deps: Deps, _env: Env, msg: cw20::Cw20QueryMsg) -> StdResult<Binary> {
        match msg {
            cw20::Cw20QueryMsg::Balance { address } => {
                to_json_binary(&cw20_base::contract::query_balance(deps, address)?)
            }
            cw20::Cw20QueryMsg::TokenInfo {} => {
                to_json_binary(&cw20_base::contract::query_token_info(deps)?)
            }
            cw20::Cw20QueryMsg::Minter {} => {
                to_json_binary(&cw20_base::contract::query_minter(deps)?)
            }
            _ => to_json_binary(&Empty {}),
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}

fn balance(app: &App, token: &Addr, who: &str) -> u128 {
    let r: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Balance {
                address: who.to_string(),
            },
        )
        .unwrap();
    r.balance.u128()
}

#[cosmwasm_schema::cw_serde]
enum TaxMapProbe {
    TaxMap {},
}

fn adopt_msg(manager: &str) -> AdoptMigrateMsg {
    AdoptMigrateMsg {
        manager: manager.into(),
        treasury: "treasury".into(),
        factory: "factory".into(),
        router: Some("router".into()),
        ust1: "ust1".into(),
        cmm_treasury: "cmm".into(),
        official_launcher: "launcher".into(),
        buy_bps: 0,
        sell_bps: 100,
        transfer_bps: None,
        max_buy_bps: 0,
        max_sell_bps: 100,
        max_transfer_bps: 0,
        source_code_id: Some(10184),
    }
}

fn spawn_source(
    cw2_name: &str,
    write_tax_map: bool,
    write_snapshot: bool,
    with_minter: bool,
) -> (App, Addr, u64) {
    let mut app = App::default();
    let admin = Addr::unchecked("source_admin");
    let holder = Addr::unchecked("holder");
    let src_code = app.store_code(source_fixture());
    let tax_code = app.store_code(token_with_migrate());
    let token = app
        .instantiate_contract(
            src_code,
            admin.clone(),
            &SourceInit {
                name: "Demo".into(),
                symbol: "DEMO".into(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: holder.to_string(),
                        amount: Uint128::new(GENESIS),
                    },
                    Cw20Coin {
                        address: admin.to_string(),
                        amount: Uint128::new(50_000_000),
                    },
                ],
                mint: if with_minter {
                    Some(MinterResponse {
                        minter: admin.to_string(),
                        cap: None,
                    })
                } else {
                    None
                },
                cw2_name: cw2_name.into(),
                write_tax_map,
                write_snapshot,
            },
            &[],
            "src",
            Some(admin.to_string()),
        )
        .unwrap();
    let _ = tax_code;
    (app, token, tax_code)
}

#[test]
fn p1_foreign_cw2_without_adopt_reverts() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let err = app
        .migrate_contract(admin, token, &MigrateMsg { adopt: None }, tax_code)
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("requires an allowlisted adopt"),
        "{err:?}"
    );
}

#[test]
fn p2_same_crate_empty_migrate_ok() {
    let mut app = App::default();
    let manager = Addr::unchecked("manager");
    let tax_a = app.store_code(token_with_migrate());
    let tax_b = app.store_code(token_with_migrate());
    let token = app
        .instantiate_contract(
            tax_a,
            manager.clone(),
            &InstantiateMsg {
                name: "Demo".into(),
                symbol: "DEMO".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: manager.to_string(),
                    amount: Uint128::new(GENESIS),
                }],
                marketing: None,
                manager: manager.to_string(),
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
                launcher: Some("launcher".into()),
                launch_guards: None,
                initial_exempt: None,
            },
            &[],
            "tax",
            Some("cmm_gov".into()),
        )
        .unwrap();
    app.migrate_contract(
        Addr::unchecked("cmm_gov"),
        token,
        &MigrateMsg { adopt: None },
        tax_b,
    )
    .unwrap();
}

#[test]
fn p3_s3_cw20_base_keeps_supply_and_writes_config() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let holder = Addr::unchecked("holder");
    let before_holder = balance(&app, &token, holder.as_str());
    let before_admin = balance(&app, &token, admin.as_str());
    let info_before: cw20::TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(&token, &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();

    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg {
            adopt: Some(adopt_msg(admin.as_str())),
        },
        tax_code,
    )
    .unwrap();

    assert_eq!(balance(&app, &token, holder.as_str()), before_holder);
    assert_eq!(balance(&app, &token, admin.as_str()), before_admin);
    let info_after: cw20::TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(info_after.total_supply, info_before.total_supply);

    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(cfg.manager, admin);
    assert_eq!(cfg.sell_bps, 100);
    assert!(!cfg.mint_revoked);

    let origin: crate::msg::LauncherOriginResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetLauncherOrigin {})
        .unwrap();
    assert_eq!(origin.launcher, Some(Addr::unchecked("launcher")));

    let mig: MigrateOriginResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetMigrateOrigin {})
        .unwrap();
    assert_eq!(mig.source_cw2.as_deref(), Some("crates.io:cw20-base"));

    let tax_map_err = app
        .wrap()
        .query_wasm_smart::<Empty>(&token, &TaxMapProbe::TaxMap {})
        .unwrap_err();
    assert!(
        tax_map_err.to_string().contains("unknown variant")
            || tax_map_err.to_string().contains("Error parsing"),
        "{tax_map_err}"
    );

    // Inbound Transfer to a stand-in pair address stays 1:1 (H-01 / T592-1).
    let pair = Addr::unchecked("cl8y_pair");
    app.execute_contract(
        holder.clone(),
        token.clone(),
        &ExecuteMsg::Transfer {
            recipient: pair.to_string(),
            amount: Uint128::new(1_000_000),
        },
        &[],
    )
    .unwrap();
    assert_eq!(balance(&app, &token, pair.as_str()), 1_000_000);
    assert_eq!(
        balance(&app, &token, holder.as_str()),
        before_holder - 1_000_000
    );
}

#[test]
fn p4_s3_8266_terraport_leftover_snapshot_unread() {
    let (mut app, token, tax_code) = spawn_source("crates.io:terraport-token", false, true, false);
    let admin = Addr::unchecked("source_admin");
    let holder = Addr::unchecked("holder");
    let before = balance(&app, &token, holder.as_str());
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg {
            adopt: Some(adopt_msg(admin.as_str())),
        },
        tax_code,
    )
    .unwrap();
    assert_eq!(balance(&app, &token, holder.as_str()), before);
    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(cfg.sell_bps, 100);
    let mig: MigrateOriginResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetMigrateOrigin {})
        .unwrap();
    assert_eq!(mig.source_cw2.as_deref(), Some("crates.io:terraport-token"));
}

#[test]
fn p5_tax_map_and_unknown_cw2_and_caps_revert() {
    let admin = Addr::unchecked("source_admin");

    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", true, false, false);
    let before = balance(&app, &token, "holder");
    let err = app
        .migrate_contract(
            admin.clone(),
            token.clone(),
            &MigrateMsg {
                adopt: Some(adopt_msg(admin.as_str())),
            },
            tax_code,
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("tax_map"), "{err:?}");
    assert_eq!(balance(&app, &token, "holder"), before);

    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-taxed", true, false, false);
    let err = app
        .migrate_contract(
            admin.clone(),
            token,
            &MigrateMsg {
                adopt: Some(adopt_msg(admin.as_str())),
            },
            tax_code,
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("not an allowlisted"),
        "{err:?}"
    );

    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let mut bad = adopt_msg(admin.as_str());
    bad.max_buy_bps = 2500;
    bad.max_sell_bps = 2500;
    bad.buy_bps = 2500;
    bad.sell_bps = 2500;
    let err = app
        .migrate_contract(admin, token, &MigrateMsg { adopt: Some(bad) }, tax_code)
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Combined max tax"),
        "{err:?}"
    );
}

#[test]
fn a6_minter_revoked_not_reenabled() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-mintable", false, false, true);
    let admin = Addr::unchecked("source_admin");
    let minter_before: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(&token, &cw20::Cw20QueryMsg::Minter {})
        .unwrap();
    assert!(minter_before.is_some());
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg {
            adopt: Some(adopt_msg(admin.as_str())),
        },
        tax_code,
    )
    .unwrap();
    let minter_after: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::Minter {})
        .unwrap();
    assert!(minter_after.is_none());
    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert!(cfg.mint_revoked);
    let feats: crate::msg::FeaturesResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetFeatures {})
        .unwrap();
    assert!(!feats.mint_control);
}

#[test]
fn a2_total_supply_unchanged_on_adopt() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let before: cw20::TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(&token, &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg {
            adopt: Some(adopt_msg(admin.as_str())),
        },
        tax_code,
    )
    .unwrap();
    let after: cw20::TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(after.total_supply, before.total_supply);
}
