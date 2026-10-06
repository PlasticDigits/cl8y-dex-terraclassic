//! Synthetic trust-lifecycle and single-use settlement regression tests (#1388).
use crate::{
    adopt_multitest::{adopt_msg, balance, spawn_source},
    msg::{ConfigResponse, ExecuteMsg, MigrateMsg, QueryMsg},
};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

// #1388 synthetic router deliberately permits duplicate/stale emission to model a
// trusted-router regression. It is never a production contract or deployment asset.
#[cosmwasm_schema::cw_serde]
struct RecoveryRouterInit {
    factory: String,
    settlement_version: u16,
}
#[cosmwasm_schema::cw_serde]
enum RecoveryRouterExecute {
    Clear {},
    Emit {
        token: String,
        route_id: u64,
        recipient: String,
        amount: Uint128,
        slots: Vec<u32>,
        fail_after: bool,
    },
}
const RECOVERY_CONFIG: cw_storage_plus::Item<dex_common::route_tax::RouterConfigResponse> =
    cw_storage_plus::Item::new("recovery_cfg");
const RECOVERY_CONTEXT: cw_storage_plus::Item<dex_common::route_tax::RouteTaxContextResponse> =
    cw_storage_plus::Item::new("recovery_ctx");
fn recovery_router() -> Box<dyn Contract<Empty>> {
    fn inst(deps: DepsMut, _: Env, _: MessageInfo, msg: RecoveryRouterInit) -> StdResult<Response> {
        RECOVERY_CONFIG.save(
            deps.storage,
            &dex_common::route_tax::RouterConfigResponse {
                factory: deps.api.addr_validate(&msg.factory)?,
                funding_version: 1,
                settlement_version: msg.settlement_version,
                last_route_id: 0,
            },
        )?;
        Ok(Response::new())
    }
    fn exec(
        deps: DepsMut,
        _: Env,
        _: MessageInfo,
        msg: RecoveryRouterExecute,
    ) -> StdResult<Response> {
        if matches!(msg, RecoveryRouterExecute::Clear {}) {
            RECOVERY_CONTEXT.remove(deps.storage);
            return Ok(Response::new());
        }
        let RecoveryRouterExecute::Emit {
            token,
            route_id,
            recipient,
            amount,
            slots,
            fail_after,
        } = msg
        else {
            unreachable!()
        };
        let token_addr = deps.api.addr_validate(&token)?;
        let to = deps.api.addr_validate(&recipient)?;
        let ctx = dex_common::route_tax::RouteTaxContextResponse {
            route_id,
            funding_version: 1,
            trader: to.clone(),
            pair: Addr::unchecked("pair"),
            offer_token: token_addr.clone(),
            ask_token: token_addr.clone(),
            gross: amount,
            economic: amount,
            acquired_tokens: vec![],
            transfers: slots
                .iter()
                .map(|slot| dex_common::route_tax::RouteTransfer {
                    slot: *slot,
                    token: token_addr.clone(),
                    recipient: to.clone(),
                    amount,
                })
                .collect(),
        };
        RECOVERY_CONTEXT.save(deps.storage, &ctx)?;
        RECOVERY_CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
            cfg.last_route_id = cfg.last_route_id.max(route_id);
            Ok(cfg)
        })?;
        let mut response = Response::new();
        for slot in slots {
            response = response.add_message(cosmwasm_std::WasmMsg::Execute {
                contract_addr: token.clone(),
                msg: to_json_binary(&ExecuteMsg::RouteTransfer {
                    slot,
                    recipient: recipient.clone(),
                    amount,
                })?,
                funds: vec![],
            });
        }
        if fail_after {
            response = response.add_message(cosmwasm_std::WasmMsg::Execute {
                contract_addr: token,
                msg: to_json_binary(&ExecuteMsg::Burn {
                    amount: Uint128::MAX,
                })?,
                funds: vec![],
            });
        }
        Ok(response)
    }
    #[cosmwasm_schema::cw_serde]
    struct FullRouterConfigFixture {
        funding_version: u16,
        settlement_version: u16,
        factory: Addr,
        last_route_id: u64,
        wrap_mapper: Option<Addr>,
    }
    fn query(deps: Deps, _: Env, msg: dex_common::route_tax::RouterQueryMsg) -> StdResult<Binary> {
        match msg {
            dex_common::route_tax::RouterQueryMsg::Config {} => {
                let config = RECOVERY_CONFIG.load(deps.storage)?;
                // Real router Config includes wrap_mapper; projections must accept
                // it while still validating every selected security field.
                to_json_binary(&FullRouterConfigFixture {
                    funding_version: config.funding_version,
                    settlement_version: config.settlement_version,
                    factory: config.factory,
                    last_route_id: config.last_route_id,
                    wrap_mapper: Some(Addr::unchecked("synthetic_mapper")),
                })
            }
            dex_common::route_tax::RouterQueryMsg::RouteTaxContext {} => {
                to_json_binary(&RECOVERY_CONTEXT.load(deps.storage)?)
            }
        }
    }
    Box::new(ContractWrapper::new(exec, inst, query))
}
fn recovery_rotate(app: &App, router: &Addr) -> ExecuteMsg {
    let code_id = app.wrap().query_wasm_contract_info(router).unwrap().code_id;
    let checksum = app.wrap().query_wasm_code_info(code_id).unwrap().checksum;
    ExecuteMsg::RotateRouter {
        router: Some(router.to_string()),
        expected_code_id: Some(code_id),
        expected_checksum: Some(checksum),
    }
}
fn recovery_spawn(app: &mut App, factory: &str, version: u16) -> Addr {
    let code = app.store_code(recovery_router());
    app.instantiate_contract(
        code,
        Addr::unchecked("source_admin"),
        &RecoveryRouterInit {
            factory: factory.into(),
            settlement_version: version,
        },
        &[],
        "synthetic recovery router",
        None,
    )
    .unwrap()
}
fn recovery_emit(
    app: &mut App,
    router: &Addr,
    token: &Addr,
    route_id: u64,
    slots: Vec<u32>,
    fail_after: bool,
) -> Result<cw_multi_test::AppResponse, String> {
    app.execute_contract(
        Addr::unchecked("source_admin"),
        router.clone(),
        &RecoveryRouterExecute::Emit {
            token: token.to_string(),
            route_id,
            recipient: "holder".into(),
            amount: Uint128::new(10),
            slots,
            fail_after,
        },
        &[],
    )
    .map_err(|err| err.root_cause().to_string())
}

#[test]
fn issue1388_adopted_governance_rotation_hash_factory_revocation_and_atomic_replay() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let old = recovery_spawn(&mut app, "factory", 1);
    let candidate = recovery_spawn(&mut app, "factory", 1);
    let mismatch = recovery_spawn(&mut app, "wrong_factory", 1);
    let unsupported = recovery_spawn(&mut app, "factory", 0);
    let mut adopt = adopt_msg("manager");
    adopt.router = Some(old.to_string());
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg { adopt: Some(adopt) },
        tax_code,
    )
    .unwrap();
    app.execute_contract(
        Addr::unchecked("holder"),
        token.clone(),
        &ExecuteMsg::Transfer {
            recipient: candidate.to_string(),
            amount: Uint128::new(1000),
        },
        &[],
    )
    .unwrap();
    let initial = balance(&app, &token, "holder");
    let rotation = recovery_rotate(&app, &candidate);
    assert!(app
        .execute_contract(Addr::unchecked("manager"), token.clone(), &rotation, &[])
        .is_err());
    for bad in [&mismatch, &unsupported] {
        let msg = recovery_rotate(&app, bad);
        assert!(app
            .execute_contract(admin.clone(), token.clone(), &msg, &[])
            .is_err());
    }
    let mut bad = rotation.clone();
    if let ExecuteMsg::RotateRouter {
        expected_checksum, ..
    } = &mut bad
    {
        *expected_checksum = Some(cosmwasm_std::HexBinary::from(vec![0; 32]));
    }
    assert!(app
        .execute_contract(admin.clone(), token.clone(), &bad, &[])
        .is_err());
    let mut wrong_code = rotation.clone();
    if let ExecuteMsg::RotateRouter {
        expected_code_id, ..
    } = &mut wrong_code
    {
        *expected_code_id = Some(u64::MAX);
    }
    assert!(app
        .execute_contract(admin.clone(), token.clone(), &wrong_code, &[])
        .is_err());
    let unchanged: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(unchanged.router, Some(old.clone()));
    app.execute_contract(admin.clone(), token.clone(), &rotation, &[])
        .unwrap();
    let revoked: crate::msg::IsExemptResponse = app
        .wrap()
        .query_wasm_smart(
            &token,
            &QueryMsg::IsProtocolExempt {
                address: old.to_string(),
            },
        )
        .unwrap();
    assert!(!revoked.protocol);
    assert!(recovery_emit(&mut app, &old, &token, 1, vec![0], false).is_err());
    // Duplicate messages revert the first credit and replay consumption atomically.
    assert!(recovery_emit(&mut app, &candidate, &token, 1, vec![0, 0], false).is_err());
    assert_eq!(balance(&app, &token, "holder"), initial);
    recovery_emit(&mut app, &candidate, &token, 1, vec![0], false).unwrap();
    assert_eq!(balance(&app, &token, "holder"), initial + 10);
    assert!(recovery_emit(&mut app, &candidate, &token, 1, vec![0], false).is_err());
    // Identical amounts/recipient and same height are valid under a genuine new route ID.
    recovery_emit(&mut app, &candidate, &token, 2, vec![0], false).unwrap();
    assert!(recovery_emit(&mut app, &candidate, &token, 1, vec![1], false).is_err());
    assert!(recovery_emit(&mut app, &candidate, &token, 3, vec![0], true).is_err());
    recovery_emit(&mut app, &candidate, &token, 3, vec![0], false).unwrap();
    assert_eq!(balance(&app, &token, candidate.as_str()), 970); // pre-existing router dust remains.
    assert!(recovery_emit(&mut app, &candidate, &token, 4, vec![5], false).is_err());
    assert!(recovery_emit(&mut app, &candidate, &token, 0, vec![0], false).is_err());
    // Rotation cannot inherit an in-flight context; clearing it establishes the
    // candidate monotonic floor, and same-router rotation preserves consumption.
    assert!(app
        .execute_contract(admin.clone(), token.clone(), &rotation, &[])
        .is_err());
    app.execute_contract(
        admin.clone(),
        candidate.clone(),
        &RecoveryRouterExecute::Clear {},
        &[],
    )
    .unwrap();
    app.execute_contract(admin.clone(), token.clone(), &rotation, &[])
        .unwrap();
    assert!(recovery_emit(&mut app, &candidate, &token, 3, vec![1], false).is_err());
    recovery_emit(&mut app, &candidate, &token, 4, vec![0, 1], false).unwrap();
    let raw = app
        .wrap()
        .query_wasm_raw(&token, b"route_settlement_consumption")
        .unwrap()
        .unwrap();
    let state: crate::router_trust::SettlementConsumption = cosmwasm_std::from_json(raw).unwrap();
    assert_eq!(state.route_id, 4);
    assert_eq!(state.slots, vec![0, 1]);
    assert!(state.slots.len() <= dex_common::route_tax::MAX_SETTLEMENT_SLOTS);
    app.execute_contract(
        admin.clone(),
        token.clone(),
        &ExecuteMsg::RotateRouter {
            router: None,
            expected_code_id: None,
            expected_checksum: None,
        },
        &[],
    )
    .unwrap();
    assert!(recovery_emit(&mut app, &candidate, &token, 5, vec![0], false).is_err());
}

#[test]
fn issue1388_rotation_preserves_overlapping_factory_protocol_role() {
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let old = recovery_spawn(&mut app, "factory", 1);
    let candidate = recovery_spawn(&mut app, old.as_str(), 1);
    let mut adopt = adopt_msg("manager");
    adopt.router = Some(old.to_string());
    adopt.factory = old.to_string();
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg { adopt: Some(adopt) },
        tax_code,
    )
    .unwrap();
    let msg = recovery_rotate(&app, &candidate);
    app.execute_contract(admin, token.clone(), &msg, &[])
        .unwrap();
    let retained: crate::msg::IsExemptResponse = app
        .wrap()
        .query_wasm_smart(
            &token,
            &QueryMsg::IsProtocolExempt {
                address: old.to_string(),
            },
        )
        .unwrap();
    assert!(retained.protocol);
}

#[test]
fn issue1388_reviewed_router_code_rejects_attacker_custody_and_accepts_factory_governance() {
    #[cosmwasm_schema::cw_serde]
    struct GovernanceFixture {
        governance: Addr,
        treasury: Addr,
        default_fee_bps: u16,
    }
    fn factory_query(_: Deps, _: Env, _: dex_common::factory::QueryMsg) -> StdResult<Binary> {
        to_json_binary(&GovernanceFixture {
            governance: Addr::unchecked("factory_governance"),
            treasury: Addr::unchecked("synthetic_treasury"),
            default_fee_bps: 30,
        })
    }
    let (mut app, token, tax_code) = spawn_source("crates.io:cw20-base", false, false, false);
    let admin = Addr::unchecked("source_admin");
    let factory_code = app.store_code(Box::new(ContractWrapper::new(
        |_: DepsMut, _: Env, _: MessageInfo, _: Empty| -> StdResult<Response> {
            Ok(Response::new())
        },
        |_: DepsMut, _: Env, _: MessageInfo, _: Empty| -> StdResult<Response> {
            Ok(Response::new())
        },
        factory_query,
    )));
    let factory = app
        .instantiate_contract(
            factory_code,
            admin.clone(),
            &Empty {},
            &[],
            "synthetic governance factory",
            None,
        )
        .unwrap();
    // Both candidates use the same reviewed artifact, factory and capabilities.
    // Their only security distinction is the live administrator.
    let router_code = app.store_code(recovery_router());
    let mut candidate = |owner: &str| {
        app.instantiate_contract(
            router_code,
            admin.clone(),
            &RecoveryRouterInit {
                factory: factory.to_string(),
                settlement_version: 1,
            },
            &[],
            "reviewed router under different custody",
            Some(owner.into()),
        )
        .unwrap()
    };
    let attacker = candidate("attacker");
    let governance = candidate("factory_governance");
    let mut adopt = adopt_msg("manager");
    adopt.factory = factory.to_string();
    app.migrate_contract(
        admin.clone(),
        token.clone(),
        &MigrateMsg { adopt: Some(adopt) },
        tax_code,
    )
    .unwrap();
    let before: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    let same_reviewed_code = recovery_rotate(&app, &attacker);
    assert!(app
        .execute_contract(admin.clone(), token.clone(), &same_reviewed_code, &[])
        .is_err());
    let after: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(before, after);
    let untrusted: crate::msg::IsExemptResponse = app
        .wrap()
        .query_wasm_smart(
            &token,
            &QueryMsg::IsProtocolExempt {
                address: attacker.to_string(),
            },
        )
        .unwrap();
    assert!(!untrusted.protocol);
    assert_eq!(
        app.wrap()
            .query_wasm_contract_info(&attacker)
            .unwrap()
            .code_id,
        app.wrap()
            .query_wasm_contract_info(&governance)
            .unwrap()
            .code_id
    );
    // The token's wasm admin differs from factory governance; this path must work
    // for CMM-owned tokens without granting its token manager router ownership.
    assert_ne!(admin.as_str(), "factory_governance");
    let governed = recovery_rotate(&app, &governance);
    app.execute_contract(admin.clone(), token.clone(), &governed, &[])
        .unwrap();
    let recovered: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(recovered.router, Some(governance.clone()));

    // An opaque contract administrator emits no owned-contract messages. Its
    // migration custody must not require a third-party generic execute upgrade.
    let opaque = app
        .instantiate_contract(
            factory_code,
            admin.clone(),
            &Empty {},
            &[],
            "opaque admin without execute dispatcher",
            None,
        )
        .unwrap();
    app.execute(
        admin.clone(),
        cosmwasm_std::WasmMsg::UpdateAdmin {
            contract_addr: token.to_string(),
            admin: opaque.to_string(),
        }
        .into(),
    )
    .unwrap();
    assert_eq!(
        app.wrap().query_wasm_contract_info(&token).unwrap().admin,
        Some(opaque.to_string())
    );
    for unrelated in [
        Addr::unchecked("manager"),
        Addr::unchecked("attacker"),
        admin,
    ] {
        assert!(app
            .execute_contract(unrelated, token.clone(), &governed, &[])
            .is_err());
    }
    let untouched: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(untouched, recovered);
    let factory_governance = Addr::unchecked("factory_governance");
    assert!(app
        .execute_contract(
            factory_governance.clone(),
            token.clone(),
            &same_reviewed_code,
            &[]
        )
        .is_err());
    app.execute_contract(
        factory_governance.clone(),
        token.clone(),
        &ExecuteMsg::RotateRouter {
            router: None,
            expected_code_id: None,
            expected_checksum: None,
        },
        &[],
    )
    .unwrap();
    let revoked: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert!(revoked.router.is_none());
    app.execute_contract(factory_governance, token.clone(), &governed, &[])
        .unwrap();
    let restored: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(restored.router, Some(governance));
    assert_eq!(restored.manager, Addr::unchecked("manager"));
}
