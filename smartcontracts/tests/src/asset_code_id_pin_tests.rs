//! GitLab #582 — pair pins asset CW20 `code_id` at instantiate and re-checks
//! pin + factory whitelist on write paths.
use crate::adversarial_token;
use crate::helpers::{
    asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
    factory_contract, pair_contract, pair_contract_with_migrate, provide_liquidity, setup_full_env,
    swap_a_to_b, wasm_attr_for_action, TestEnv,
};
use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
use cw2::set_contract_version;
use cw_multi_test::{App, Executor};

fn query_pins(app: &App, pair: &Addr) -> [u64; 2] {
    let resp: dex_common::pair::AssetCodeIdsResponse = app
        .wrap()
        .query_wasm_smart(
            pair.to_string(),
            &dex_common::pair::QueryMsg::GetAssetCodeIds {},
        )
        .unwrap();
    resp.code_ids
}

fn try_swap_a_to_b(
    app: &mut App,
    env: &TestEnv,
    sender: &Addr,
    amount: Uint128,
) -> Result<cw_multi_test::AppResponse, String> {
    let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        greedy: None,
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        sender.clone(),
        env.token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount,
            msg: swap_msg,
        },
        &[],
    )
    .map_err(|e| e.root_cause().to_string())
}

fn create_honest_adv_with_admin(
    app: &mut App,
    code_id: u64,
    owner: &Addr,
    name: &str,
    symbol: &str,
    initial: Uint128,
) -> Addr {
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &adversarial_token::InstantiateMsg {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: owner.to_string(),
                amount: initial,
            }],
            mint: None,
            mode: adversarial_token::AdversarialMode::Honest,
        },
        &[],
        name,
        Some(owner.to_string()),
    )
    .unwrap()
}

fn instantiate_factory(
    app: &mut App,
    governance: &Addr,
    treasury: &Addr,
    pair_code_id: u64,
    lp_code_id: u64,
    whitelisted: Vec<u64>,
) -> Addr {
    let factory_code_id = app.store_code(factory_contract());
    app.instantiate_contract(
        factory_code_id,
        governance.clone(),
        &dex_common::factory::InstantiateMsg {
            governance: governance.to_string(),
            treasury: treasury.to_string(),
            default_fee_bps: 30,
            pair_code_id,
            lp_token_code_id: lp_code_id,
            whitelisted_code_ids: whitelisted,
            default_limit_batch_max_rungs:
                dex_common::pair::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS,
            pair_creation_fee_uluna: Uint128::zero(),
        },
        &[],
        "factory",
        None,
    )
    .unwrap()
}

fn seed_pool(app: &mut App, env: &TestEnv) {
    provide_liquidity(
        app,
        env,
        &env.user,
        Uint128::new(1_000_000_000),
        Uint128::new(1_000_000_000),
    );
}

#[test]
fn create_pair_pins_live_code_ids_and_is_whitelisted_query() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let info = app
        .wrap()
        .query_wasm_contract_info(env.token_a.to_string())
        .unwrap();
    let pins = query_pins(&app, &env.pair);
    assert_eq!(pins[0], info.code_id);
    assert_eq!(pins[1], info.code_id);

    let yes: dex_common::factory::CodeIdWhitelistedResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::IsCodeIdWhitelisted {
                code_id: info.code_id,
            },
        )
        .unwrap();
    assert!(yes.whitelisted);
    assert_eq!(yes.code_id, info.code_id);

    let no: dex_common::factory::CodeIdWhitelistedResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::IsCodeIdWhitelisted { code_id: 99_999 },
        )
        .unwrap();
    assert!(!no.whitelisted);
}

#[test]
fn honest_create_pair_then_migrate_to_fot_swap_must_fail() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");

    let lp_code = app.store_code(cw20_mintable_contract());
    let honest_code = app.store_code(adversarial_token::adversarial_cw20_contract_with_migrate());
    let fot_code = app.store_code(adversarial_token::adversarial_cw20_contract_with_fot_migrate());
    let pair_code = app.store_code(pair_contract());
    let factory = instantiate_factory(
        &mut app,
        &governance,
        &treasury,
        pair_code,
        lp_code,
        vec![honest_code],
    );

    let initial = Uint128::new(10_000_000_000);
    let token_a =
        create_honest_adv_with_admin(&mut app, honest_code, &user, "Token A", "TKNA", initial);
    let token_b =
        create_honest_adv_with_admin(&mut app, honest_code, &user, "Token B", "TKNB", initial);

    let resp = app
        .execute_contract(
            user.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
            },
            &[],
        )
        .unwrap();
    let pair = extract_pair_address(&resp.events);
    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    let env = TestEnv {
        factory: factory.clone(),
        token_a: token_a.clone(),
        token_b: token_b.clone(),
        pair,
        lp_token: pair_info.liquidity_token,
        router: Addr::unchecked("unused"),
        governance: governance.clone(),
        treasury,
        user: user.clone(),
    };
    seed_pool(&mut app, &env);

    let pinned = query_pins(&app, &env.pair);
    assert_eq!(pinned[0], honest_code);

    app.migrate_contract(user.clone(), token_a, &Empty {}, fot_code)
        .unwrap();

    let live = app
        .wrap()
        .query_wasm_contract_info(env.token_a.to_string())
        .unwrap();
    assert_eq!(live.code_id, fot_code);

    let err = try_swap_a_to_b(&mut app, &env, &user, Uint128::new(1_000_000)).unwrap_err();
    assert!(
        err.contains("Asset CW20 code_id drifted"),
        "expected pin drift, got {err}"
    );

    let refresh_err = app
        .execute_contract(
            governance,
            factory,
            &dex_common::factory::ExecuteMsg::RefreshPairAssetCodeIds {
                pair: env.pair.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(
        refresh_err.contains("is not factory-whitelisted"),
        "refresh must refuse unlisted FoT live id, got {refresh_err}"
    );
}

#[test]
fn remove_whitelisted_code_id_blocks_swap_until_restored() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    seed_pool(&mut app, &env);
    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000_000));

    let pinned = query_pins(&app, &env.pair);
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::RemoveWhitelistedCodeId { code_id: pinned[0] },
        &[],
    )
    .unwrap();

    let err = try_swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000_000)).unwrap_err();
    assert!(
        err.contains("is not factory-whitelisted"),
        "expected whitelist freeze, got {err}"
    );

    let provide_err = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(1_000),
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(1_000),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(
        provide_err.contains("is not factory-whitelisted"),
        "provide must fail-closed too, got {provide_err}"
    );

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: pinned[0] },
        &[],
    )
    .unwrap();
    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000_000));
}

#[test]
fn migrate_to_other_whitelisted_template_fails_until_refresh() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");

    let lp_code = app.store_code(cw20_mintable_contract());
    let code_a = app.store_code(adversarial_token::adversarial_cw20_contract_with_migrate());
    let code_b = app.store_code(adversarial_token::adversarial_cw20_contract_with_migrate());
    let pair_code = app.store_code(pair_contract());
    let factory = instantiate_factory(
        &mut app,
        &governance,
        &treasury,
        pair_code,
        lp_code,
        vec![code_a, code_b],
    );

    let initial = Uint128::new(10_000_000_000);
    let token_a = create_honest_adv_with_admin(&mut app, code_a, &user, "Token A", "TKNA", initial);
    let token_b = create_honest_adv_with_admin(&mut app, code_a, &user, "Token B", "TKNB", initial);

    let resp = app
        .execute_contract(
            user.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
            },
            &[],
        )
        .unwrap();
    let pair = extract_pair_address(&resp.events);
    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    let env = TestEnv {
        factory: factory.clone(),
        token_a: token_a.clone(),
        token_b,
        pair: pair.clone(),
        lp_token: pair_info.liquidity_token,
        router: Addr::unchecked("unused"),
        governance: governance.clone(),
        treasury,
        user: user.clone(),
    };
    seed_pool(&mut app, &env);
    assert_eq!(query_pins(&app, &pair)[0], code_a);

    app.migrate_contract(user.clone(), token_a, &Empty {}, code_b)
        .unwrap();

    let err = try_swap_a_to_b(&mut app, &env, &user, Uint128::new(1_000_000)).unwrap_err();
    assert!(
        err.contains("Asset CW20 code_id drifted"),
        "pin must reject migrate onto another whitelisted template, got {err}"
    );

    let unauth = app
        .execute_contract(
            user.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::RefreshPairAssetCodeIds {
                pair: pair.to_string(),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(
        unauth.contains("Unauthorized"),
        "only governance may refresh pins, got {unauth}"
    );

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &dex_common::factory::ExecuteMsg::RefreshPairAssetCodeIds {
            pair: pair.to_string(),
        },
        &[],
    )
    .unwrap();
    assert_eq!(query_pins(&app, &pair)[0], code_b);
    swap_a_to_b(&mut app, &env, &user, Uint128::new(1_000_000));
}

#[test]
fn pair_migrate_backfills_missing_asset_code_id_pins() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");

    let cw20_code = app.store_code(cw20_mintable_contract());
    let pair_code = app.store_code(pair_contract_with_migrate());
    let factory = instantiate_factory(
        &mut app,
        &governance,
        &treasury,
        pair_code,
        cw20_code,
        vec![cw20_code],
    );
    let initial = Uint128::new(10_000_000_000);
    let token_a = create_cw20_token(&mut app, cw20_code, &user, "Token A", "TKNA", initial);
    let token_b = create_cw20_token(&mut app, cw20_code, &user, "Token B", "TKNB", initial);
    let resp = app
        .execute_contract(
            user.clone(),
            factory,
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
            },
            &[],
        )
        .unwrap();
    let pair = extract_pair_address(&resp.events);
    let expected = query_pins(&app, &pair);

    {
        let mut storage = app.contract_storage_mut(&pair);
        cl8y_dex_pair::state::ASSET_CODE_IDS.remove(&mut *storage);
        set_contract_version(&mut *storage, "cl8y-dex-pair", "1.14.0").unwrap();
    }

    app.wrap()
        .query_wasm_smart::<dex_common::pair::AssetCodeIdsResponse>(
            pair.to_string(),
            &dex_common::pair::QueryMsg::GetAssetCodeIds {},
        )
        .unwrap_err();

    app.migrate_contract(
        governance,
        pair.clone(),
        &cl8y_dex_pair::msg::MigrateMsg {},
        pair_code,
    )
    .unwrap();
    assert_eq!(query_pins(&app, &pair), expected);
}

#[test]
fn refresh_pair_asset_code_ids_batch_covers_indexed_pairs() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    seed_pool(&mut app, &env);

    let resp = app
        .execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::RefreshPairAssetCodeIdsBatch {
                start_after: None,
                limit: Some(10),
            },
            &[],
        )
        .unwrap();
    let updated = wasm_attr_for_action(
        &resp.events,
        "refresh_pair_asset_code_ids_batch",
        "pairs_updated",
    )
    .unwrap();
    assert_eq!(updated, "1");
    let has_more = wasm_attr_for_action(
        &resp.events,
        "refresh_pair_asset_code_ids_batch",
        "has_more",
    )
    .unwrap();
    assert_eq!(has_more, "false");
}
