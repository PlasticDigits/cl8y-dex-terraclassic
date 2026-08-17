//! GitLab #536 — factory snapshots `discount_registry` so new pairs inherit it at `CreatePair`.
use crate::helpers::{
    asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
    fee_discount_contract, setup_full_env, TestEnv,
};
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::{App, Executor};

fn instantiate_fee_discount(app: &mut App, env: &TestEnv) -> Addr {
    let fd_code = app.store_code(fee_discount_contract());
    let cw20_code = app.store_code(cw20_mintable_contract());
    let cl8y = create_cw20_token(
        app,
        cw20_code,
        &env.user,
        "CL8Y",
        "CL8Y",
        Uint128::new(1_000_000),
    );
    app.instantiate_contract(
        fd_code,
        env.governance.clone(),
        &cl8y_dex_fee_discount::msg::InstantiateMsg {
            governance: env.governance.to_string(),
            cl8y_token: cl8y.to_string(),
        },
        &[],
        "fee_discount",
        None,
    )
    .unwrap()
}

fn query_factory_registry(app: &App, factory: &Addr) -> Option<Addr> {
    let cfg: dex_common::factory::ConfigResponse = app
        .wrap()
        .query_wasm_smart(
            factory.to_string(),
            &dex_common::factory::QueryMsg::Config {},
        )
        .unwrap();
    cfg.discount_registry
}

fn query_pair_registry(app: &App, pair: &Addr) -> Option<Addr> {
    let resp: dex_common::pair::DiscountRegistryResponse = app
        .wrap()
        .query_wasm_smart(
            pair.to_string(),
            &dex_common::pair::QueryMsg::GetDiscountRegistry {},
        )
        .unwrap();
    resp.registry
}

/// Extra CW20 + `CreatePair` on a new block (one create flow per height).
fn create_extra_pair(app: &mut App, env: &TestEnv, symbol: &str) -> Addr {
    let cw20_code = app.store_code(cw20_mintable_contract());
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: cw20_code },
        &[],
    )
    .unwrap();
    let token_c = create_cw20_token(
        app,
        cw20_code,
        &env.user,
        symbol,
        symbol,
        Uint128::new(1_000_000_000_000),
    );
    app.update_block(|b| b.height += 1);
    let resp = app
        .execute_contract(
            env.user.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(&env.token_b), asset_info_token(&token_c)],
            },
            &[],
        )
        .unwrap();
    extract_pair_address(&resp.events)
}

#[test]
fn create_pair_before_factory_pointer_leaves_registry_none() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);

    assert!(query_factory_registry(&app, &env.factory).is_none());
    assert!(
        query_pair_registry(&app, &env.pair).is_none(),
        "CreatePair with no factory pointer must instantiate DISCOUNT_REGISTRY = None"
    );

    let extra = create_extra_pair(&mut app, &env, "TKNC");
    assert!(query_pair_registry(&app, &extra).is_none());
}

#[test]
fn get_discount_registry_returns_stored_option_addr() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    assert_eq!(query_pair_registry(&app, &env.pair), None);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
            pair: env.pair.to_string(),
            registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();
    assert_eq!(query_pair_registry(&app, &env.pair), Some(fd.clone()));

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
            pair: env.pair.to_string(),
            registry: None,
        },
        &[],
    )
    .unwrap();
    assert_eq!(query_pair_registry(&app, &env.pair), None);
}

#[test]
fn set_discount_registry_single_pair_does_not_change_factory_pointer() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
            pair: env.pair.to_string(),
            registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_pair_registry(&app, &env.pair), Some(fd));
    assert!(
        query_factory_registry(&app, &env.factory).is_none(),
        "single-pair SetDiscountRegistry must not write factory config.discount_registry"
    );

    let extra = create_extra_pair(&mut app, &env, "TKND");
    assert!(
        query_pair_registry(&app, &extra).is_none(),
        "new pair must not inherit a pointer that was only set on one existing pair"
    );
}

#[test]
fn create_pair_after_set_discount_registry_all_inherits_registry() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistryAll {
            registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_factory_registry(&app, &env.factory), Some(fd.clone()));
    assert_eq!(query_pair_registry(&app, &env.pair), Some(fd.clone()));

    let extra = create_extra_pair(&mut app, &env, "TKNE");
    assert_eq!(
        query_pair_registry(&app, &extra),
        Some(fd),
        "CreatePair after All must copy factory pointer without a follow-up SetDiscountRegistry"
    );
}

#[test]
fn create_pair_after_set_discount_registry_batch_inherits_registry() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistryBatch {
            registry: Some(fd.to_string()),
            start_after: None,
            limit: Some(10),
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_factory_registry(&app, &env.factory), Some(fd.clone()));
    assert_eq!(query_pair_registry(&app, &env.pair), Some(fd.clone()));

    let extra = create_extra_pair(&mut app, &env, "TKNF");
    assert_eq!(query_pair_registry(&app, &extra), Some(fd));
}

#[test]
fn update_config_sets_factory_pointer_without_touching_indexed_pairs() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::UpdateConfig {
            governance: None,
            treasury: None,
            default_fee_bps: None,
            default_limit_batch_max_rungs: None,
            pair_code_id: None,
            lp_token_code_id: None,
            discount_registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_factory_registry(&app, &env.factory), Some(fd.clone()));
    assert!(
        query_pair_registry(&app, &env.pair).is_none(),
        "UpdateConfig must not fan SetDiscountRegistry to existing pairs"
    );

    let extra = create_extra_pair(&mut app, &env, "TKNG");
    assert_eq!(query_pair_registry(&app, &extra), Some(fd));
}

#[test]
fn set_discount_registry_all_none_clears_factory_pointer() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let fd = instantiate_fee_discount(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistryAll {
            registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetDiscountRegistryAll { registry: None },
        &[],
    )
    .unwrap();

    assert!(query_factory_registry(&app, &env.factory).is_none());
    assert!(query_pair_registry(&app, &env.pair).is_none());

    let extra = create_extra_pair(&mut app, &env, "TKNH");
    assert!(query_pair_registry(&app, &extra).is_none());
}
