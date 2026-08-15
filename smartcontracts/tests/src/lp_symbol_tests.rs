//! GitLab #518 — create_pair must succeed for tickers that contain digits.
use crate::classic_lp_cw20::classic_lp_cw20_contract;
use crate::helpers::{
    asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
    factory_contract, pair_contract,
};
use cosmwasm_std::{Addr, Uint128};
use cw20::TokenInfoResponse;
use cw_multi_test::{App, Executor};
use dex_common::lp_symbol::{
    derive_lp_token_symbol, is_classic_cw20_lp_symbol, legacy_unsanitized_lp_symbol,
    FALLBACK_LP_TOKEN_SYMBOL,
};

fn query_lp_symbol(app: &App, lp: &Addr) -> String {
    let info: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(lp.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    info.symbol
}

fn setup_factory_with_lp_code(app: &mut App, lp_code_id: u64, asset_code_id: u64) -> Addr {
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let pair_code_id = app.store_code(pair_contract());
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
            whitelisted_code_ids: vec![asset_code_id],
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

fn create_pair_ok(app: &mut App, factory: &Addr, user: &Addr, a: &Addr, b: &Addr) -> Addr {
    let resp = app
        .execute_contract(
            user.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(a), asset_info_token(b)],
            },
            &[],
        )
        .unwrap();
    extract_pair_address(&resp.events)
}

fn pair_lp(app: &App, pair: &Addr) -> Addr {
    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    pair_info.liquidity_token
}

/// Factory uppercases + take(6) before pair derive. Mirrors factory `truncate`.
fn factory_passed_symbol(on_chain: &str) -> String {
    on_chain.chars().take(6).collect::<String>().to_uppercase()
}

#[test]
fn create_pair_ust1_custc_succeeds_on_classic_lp_cw20() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let lp_code = app.store_code(classic_lp_cw20_contract());
    let factory = setup_factory_with_lp_code(&mut app, lp_code, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let ust1 = create_cw20_token(&mut app, asset_code, &user, "UST1", "UST1", initial);
    let custc = create_cw20_token(&mut app, asset_code, &user, "cUSTC", "cUSTC", initial);

    assert!(!is_classic_cw20_lp_symbol(&legacy_unsanitized_lp_symbol(
        &factory_passed_symbol("UST1"),
        &factory_passed_symbol("cUSTC"),
    )));

    let pair = create_pair_ok(&mut app, &factory, &user, &ust1, &custc);
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "UST-CUST-LP");
    assert!(is_classic_cw20_lp_symbol(&symbol));
}

#[test]
fn create_pair_cl8y_clunc_succeeds_on_classic_lp_cw20() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let lp_code = app.store_code(classic_lp_cw20_contract());
    let factory = setup_factory_with_lp_code(&mut app, lp_code, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let cl8y = create_cw20_token(&mut app, asset_code, &user, "CL8Y", "CL8Y", initial);
    let clunc = create_cw20_token(&mut app, asset_code, &user, "cLUNC", "cLUNC", initial);

    let pair = create_pair_ok(&mut app, &factory, &user, &cl8y, &clunc);
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "CLY-CLUN-LP");
    assert!(is_classic_cw20_lp_symbol(&symbol));
}

#[test]
fn create_pair_letter_only_pair_keeps_readable_ticker() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let lp_code = app.store_code(classic_lp_cw20_contract());
    let factory = setup_factory_with_lp_code(&mut app, lp_code, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let clunc = create_cw20_token(&mut app, asset_code, &user, "cLUNC", "cLUNC", initial);
    let custc = create_cw20_token(&mut app, asset_code, &user, "cUSTC", "cUSTC", initial);

    let pair = create_pair_ok(&mut app, &factory, &user, &clunc, &custc);
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "CLUN-CUST-LP");
    assert!(is_classic_cw20_lp_symbol(&symbol));
}

#[test]
fn create_pair_digit_tickers_also_succeed_on_mintable_lp() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let cw20_code = app.store_code(cw20_mintable_contract());
    let factory = setup_factory_with_lp_code(&mut app, cw20_code, cw20_code);
    let initial = Uint128::new(1_000_000_000);
    let ust1 = create_cw20_token(&mut app, cw20_code, &user, "UST1", "UST1", initial);
    let ustr = create_cw20_token(&mut app, cw20_code, &user, "USTR", "USTR", initial);

    let pair = create_pair_ok(&mut app, &factory, &user, &ust1, &ustr);
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "UST-USTR-LP");
    assert!(is_classic_cw20_lp_symbol(&symbol));
}

#[test]
fn pair_instantiate_none_symbols_uses_digit_free_fallback() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let lp_code = app.store_code(classic_lp_cw20_contract());
    let pair_code = app.store_code(pair_contract());
    let initial = Uint128::new(1_000_000);
    let token_a = create_cw20_token(&mut app, asset_code, &user, "TokenA", "TKNA", initial);
    let token_b = create_cw20_token(&mut app, asset_code, &user, "TokenB", "TKNB", initial);

    let pair = app
        .instantiate_contract(
            pair_code,
            user.clone(),
            &dex_common::pair::PairInstantiateMsg {
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
                fee_bps: 30,
                treasury: Addr::unchecked("treasury"),
                factory: Addr::unchecked("factory"),
                lp_token_code_id: lp_code,
                token_symbols: None,
                governance: "governance".to_string(),
                max_batch_rungs: dex_common::pair::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS,
            },
            &[],
            "pair-none-symbols",
            None,
        )
        .unwrap();

    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, FALLBACK_LP_TOKEN_SYMBOL);
    assert_eq!(symbol, derive_lp_token_symbol(None));
}

#[test]
fn factory_passed_symbols_match_documented_launch_pairs() {
    assert_eq!(factory_passed_symbol("UST1"), "UST1");
    assert_eq!(factory_passed_symbol("CL8Y"), "CL8Y");
    assert_eq!(factory_passed_symbol("cUSTC"), "CUSTC");
    assert_eq!(factory_passed_symbol("cLUNC"), "CLUNC");
    assert_eq!(
        derive_lp_token_symbol(Some(["UST1", "CUSTC"])),
        "UST-CUST-LP"
    );
    assert_eq!(
        derive_lp_token_symbol(Some(["CL8Y", "CLUNC"])),
        "CLY-CLUN-LP"
    );
}
