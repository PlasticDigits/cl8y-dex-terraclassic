//! GitLab #518 — keep digits in LP tickers; upgrade factory LP code off classic CW20.
use crate::classic_lp_cw20::classic_lp_cw20_contract;
use crate::helpers::{
    asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
    factory_contract, pair_contract,
};
use cosmwasm_std::{Addr, Uint128};
use cw20::TokenInfoResponse;
use cw_multi_test::{App, Executor};
use dex_common::lp_symbol::{
    derive_lp_token_symbol, is_classic_cw20_lp_symbol, is_mintable_cw20_lp_symbol,
    FALLBACK_LP_TOKEN_SYMBOL,
};

fn query_lp_symbol(app: &App, lp: &Addr) -> String {
    let info: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(lp.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    info.symbol
}

fn setup_factory(app: &mut App, lp_code_id: u64, asset_code_id: u64) -> (Addr, u64) {
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let pair_code_id = app.store_code(pair_contract());
    let factory_code_id = app.store_code(factory_contract());
    let factory = app
        .instantiate_contract(
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
        .unwrap();
    (factory, pair_code_id)
}

fn create_pair(
    app: &mut App,
    factory: &Addr,
    user: &Addr,
    a: &Addr,
    b: &Addr,
) -> Result<Addr, String> {
    app.execute_contract(
        user.clone(),
        factory.clone(),
        &dex_common::factory::ExecuteMsg::CreatePair {
            asset_infos: [asset_info_token(a), asset_info_token(b)],
        },
        &[],
    )
    .map(|resp| extract_pair_address(&resp.events))
    .map_err(|e| e.root_cause().to_string())
}

fn pair_lp(app: &App, pair: &Addr) -> Addr {
    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    pair_info.liquidity_token
}

fn factory_passed_symbol(on_chain: &str) -> String {
    on_chain.chars().take(6).collect::<String>().to_uppercase()
}

#[test]
fn create_pair_ust1_reverts_on_classic_lp_until_code_upgrade() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let classic_lp = app.store_code(classic_lp_cw20_contract());
    let (factory, _) = setup_factory(&mut app, classic_lp, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let ust1 = create_cw20_token(&mut app, asset_code, &user, "UST1", "UST1", initial);
    let custc = create_cw20_token(&mut app, asset_code, &user, "cUSTC", "cUSTC", initial);

    let derived = derive_lp_token_symbol(Some([
        &factory_passed_symbol("UST1"),
        &factory_passed_symbol("cUSTC"),
    ]));
    assert_eq!(derived, "UST1-CUST-LP");
    assert!(is_mintable_cw20_lp_symbol(&derived));
    assert!(!is_classic_cw20_lp_symbol(&derived));

    let err = create_pair(&mut app, &factory, &user, &ust1, &custc).unwrap_err();
    assert!(
        err.contains("Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}"),
        "classic LP must reject digit ticker, got: {err}"
    );
}

#[test]
fn create_pair_cl8y_reverts_on_classic_lp() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let classic_lp = app.store_code(classic_lp_cw20_contract());
    let (factory, _) = setup_factory(&mut app, classic_lp, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let cl8y = create_cw20_token(&mut app, asset_code, &user, "CL8Y", "CL8Y", initial);
    let clunc = create_cw20_token(&mut app, asset_code, &user, "cLUNC", "cLUNC", initial);

    let err = create_pair(&mut app, &factory, &user, &cl8y, &clunc).unwrap_err();
    assert!(
        err.contains("[a-zA-Z\\-]{3,12}"),
        "classic LP must reject CL8Y ticker, got: {err}"
    );
}

#[test]
fn create_pair_letter_only_still_works_on_classic_lp() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let classic_lp = app.store_code(classic_lp_cw20_contract());
    let (factory, _) = setup_factory(&mut app, classic_lp, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let clunc = create_cw20_token(&mut app, asset_code, &user, "cLUNC", "cLUNC", initial);
    let custc = create_cw20_token(&mut app, asset_code, &user, "cUSTC", "cUSTC", initial);

    let pair = create_pair(&mut app, &factory, &user, &clunc, &custc).unwrap();
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "CLUN-CUST-LP");
    assert!(is_classic_cw20_lp_symbol(&symbol));
}

#[test]
fn create_pair_ust1_succeeds_on_mintable_lp() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let cw20_code = app.store_code(cw20_mintable_contract());
    let (factory, _) = setup_factory(&mut app, cw20_code, cw20_code);
    let initial = Uint128::new(1_000_000_000);
    let ust1 = create_cw20_token(&mut app, cw20_code, &user, "UST1", "UST1", initial);
    let ustr = create_cw20_token(&mut app, cw20_code, &user, "USTR", "USTR", initial);

    let pair = create_pair(&mut app, &factory, &user, &ust1, &ustr).unwrap();
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "UST1-USTR-LP");
    assert!(is_mintable_cw20_lp_symbol(&symbol));
}

/// Operator path: UpdateConfig.lp_token_code_id from classic → mintable, then UST1 create_pair.
#[test]
fn factory_update_config_lp_code_unblocks_ust1_create_pair() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let classic_lp = app.store_code(classic_lp_cw20_contract());
    let mintable_lp = app.store_code(cw20_mintable_contract());
    let (factory, pair_code) = setup_factory(&mut app, classic_lp, asset_code);
    let initial = Uint128::new(1_000_000_000);
    let ust1 = create_cw20_token(&mut app, asset_code, &user, "UST1", "UST1", initial);
    let custc = create_cw20_token(&mut app, asset_code, &user, "cUSTC", "cUSTC", initial);

    assert!(create_pair(&mut app, &factory, &user, &ust1, &custc).is_err());

    app.execute_contract(
        governance,
        factory.clone(),
        &dex_common::factory::ExecuteMsg::UpdateConfig {
            governance: None,
            treasury: None,
            default_fee_bps: None,
            default_limit_batch_max_rungs: None,
            pair_code_id: Some(pair_code),
            lp_token_code_id: Some(mintable_lp),
        },
        &[],
    )
    .unwrap();

    let cfg: dex_common::factory::ConfigResponse = app
        .wrap()
        .query_wasm_smart(
            factory.to_string(),
            &dex_common::factory::QueryMsg::Config {},
        )
        .unwrap();
    assert_eq!(cfg.lp_token_code_id, mintable_lp);
    assert_eq!(cfg.pair_code_id, pair_code);

    app.update_block(|b| b.height += 1);
    let pair = create_pair(&mut app, &factory, &user, &ust1, &custc).unwrap();
    let symbol = query_lp_symbol(&app, &pair_lp(&app, &pair));
    assert_eq!(symbol, "UST1-CUST-LP");
}

#[test]
fn update_config_rejects_zero_code_ids() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");
    let cw20_code = app.store_code(cw20_mintable_contract());
    let (factory, _) = setup_factory(&mut app, cw20_code, cw20_code);

    let err = app
        .execute_contract(
            governance,
            factory,
            &dex_common::factory::ExecuteMsg::UpdateConfig {
                governance: None,
                treasury: None,
                default_fee_bps: None,
                default_limit_batch_max_rungs: None,
                pair_code_id: Some(0),
                lp_token_code_id: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid code ID"));
}

#[test]
fn pair_instantiate_none_symbols_uses_cl8y_lp_on_mintable() {
    let mut app = App::default();
    let user = Addr::unchecked("user");
    let asset_code = app.store_code(cw20_mintable_contract());
    let lp_code = app.store_code(cw20_mintable_contract());
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
    assert_eq!(symbol, "CL8Y-LP");
}

#[test]
fn factory_passed_symbols_keep_digits() {
    assert_eq!(factory_passed_symbol("UST1"), "UST1");
    assert_eq!(factory_passed_symbol("CL8Y"), "CL8Y");
    assert_eq!(
        derive_lp_token_symbol(Some(["UST1", "CUSTC"])),
        "UST1-CUST-LP"
    );
    assert_eq!(
        derive_lp_token_symbol(Some(["CL8Y", "CLUNC"])),
        "CL8Y-CLUN-LP"
    );
}
