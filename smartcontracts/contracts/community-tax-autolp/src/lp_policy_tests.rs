//! Synthetic economic-policy regressions and real-pair LP-ledger reconciliation.
use super::*;

fn fund_tax(h: &mut Harness, amount: u128) {
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(amount),
            },
            &[],
        )
        .unwrap();
}

fn approve_current_pair(h: &mut Harness) {
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
}

fn allowance(h: &Harness, token: &Addr) -> Uint128 {
    let response: cw20::AllowanceResponse = h
        .app
        .wrap()
        .query_wasm_smart(
            token,
            &cw20::Cw20QueryMsg::Allowance {
                owner: h.autolp.to_string(),
                spender: h.pair.to_string(),
            },
        )
        .unwrap();
    response.allowance
}

#[test]
fn lp1398_insufficient_mint_reply_rolls_back_all_balances_allowances_and_supply() {
    let mut h = setup(true);
    set_swap_output(&mut h, 1_000_000);
    fund_tax(&mut h, 2_000_000);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.pair.clone(),
            &MockPairExecute::SetIssuanceBps { bps: 9_000 },
            &[],
        )
        .unwrap();
    let pool_before: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &MockPairQuery::Pool {})
        .unwrap();
    let before = [
        bal(&h.app, &h.token, h.autolp.as_str()),
        bal(&h.app, &h.quote, h.autolp.as_str()),
        bal(&h.app, &h.token, h.pair.as_str()),
        bal(&h.app, &h.quote, h.pair.as_str()),
        bal(&h.app, &h.lp, h.manager.as_str()),
    ];
    let error = h
        .app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("LP issuance below approved minimum"),
        "{error:?}"
    );
    assert_eq!(
        before,
        [
            bal(&h.app, &h.token, h.autolp.as_str()),
            bal(&h.app, &h.quote, h.autolp.as_str()),
            bal(&h.app, &h.token, h.pair.as_str()),
            bal(&h.app, &h.quote, h.pair.as_str()),
            bal(&h.app, &h.lp, h.manager.as_str())
        ]
    );
    assert_eq!(allowance(&h, &h.token), Uint128::zero());
    assert_eq!(allowance(&h, &h.quote), Uint128::zero());
    let pool_after: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &MockPairQuery::Pool {})
        .unwrap();
    assert_eq!(pool_before, pool_after);
    let supply: cw20::TokenInfoResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.lp, &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert!(supply.total_supply.is_zero());
    assert!(!cfg(&h).skimming);
    assert!(h
        .app
        .wrap()
        .query_wasm_raw(h.autolp, b"pending_skim".as_slice())
        .unwrap()
        .is_none());
}

#[test]
fn lp1398_pre_skim_manipulation_donation_and_dilution_fail_without_reapproval() {
    for (tax, quote, shares) in [
        (1_020_000_000, 980_000_000, 1_000_000_000),
        (1_020_000_000, 1_020_000_000, 1_000_000_000),
        (1_000_000_000, 1_000_000_000, 1_020_000_000),
    ] {
        let mut h = setup(true);
        fund_tax(&mut h, 2_000_000);
        h.app
            .execute_contract(
                h.manager.clone(),
                h.pair.clone(),
                &MockPairExecute::SetReserves {
                    tax: Uint128::new(tax),
                    quote: Uint128::new(quote),
                    shares: Uint128::new(shares),
                },
                &[],
            )
            .unwrap();
        let error = h
            .app
            .execute_contract(
                Addr::unchecked("keeper"),
                h.autolp.clone(),
                &ExecuteMsg::SkimToLp {},
                &[],
            )
            .unwrap_err();
        assert!(
            format!("{error:?}").contains("changed beyond the approved policy tolerance"),
            "{error:?}"
        );
        assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 2_000_000);
        assert!(!cfg(&h).skimming);
    }
}

#[test]
fn lp1398_quote_donation_ratio_failure_rolls_back_swap_and_approvals() {
    let mut h = setup(true);
    set_swap_output(&mut h, 1_000_000);
    fund_tax(&mut h, 2_000_000);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.quote.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(1_000_000),
            },
            &[],
        )
        .unwrap();
    let error = h
        .app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("LP ratio tolerance exceeded"),
        "{error:?}"
    );
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 2_000_000);
    assert_eq!(bal(&h.app, &h.quote, h.autolp.as_str()), 1_000_000);
    assert_eq!(bal(&h.app, &h.quote, h.pair.as_str()), 1_000_000);
    assert_eq!(allowance(&h, &h.token), Uint128::zero());
    assert_eq!(allowance(&h, &h.quote), Uint128::zero());
    assert!(!cfg(&h).skimming);
}

#[test]
fn lp1398_manager_rebind_refreshes_anchor_and_other_updates_do_not() {
    let mut h = setup(true);
    let original: Option<crate::state::LpPolicy> = h
        .app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetLpPolicy {})
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.pair.clone(),
            &MockPairExecute::SetReserves {
                tax: Uint128::new(1_100_000_000),
                quote: Uint128::new(1_100_000_000),
                shares: Uint128::new(1_000_000_000),
            },
            &[],
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: None,
                router: None,
                quote_token: None,
                threshold: Some(Uint128::new(123)),
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap();
    let unchanged: Option<crate::state::LpPolicy> = h
        .app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetLpPolicy {})
        .unwrap();
    assert_eq!(original, unchanged);
    approve_current_pair(&mut h);
    let refreshed: Option<crate::state::LpPolicy> = h
        .app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetLpPolicy {})
        .unwrap();
    assert_eq!(refreshed.unwrap().tax_reserve, Uint128::new(1_100_000_000));
    set_swap_output(&mut h, 1_000_000);
    fund_tax(&mut h, 2_000_000);
    h.app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
}

fn real_pool_harness(initialize: bool) -> Harness {
    let mut h = setup(false);
    let pair_code = h.app.store_code(Box::new(
        ContractWrapper::new(
            cl8y_dex_pair::contract::execute,
            cl8y_dex_pair::contract::instantiate,
            cl8y_dex_pair::contract::query,
        )
        .with_reply(cl8y_dex_pair::contract::reply),
    ));
    let lp_code = h.app.store_code(Box::new(ContractWrapper::new(
        cw20_mintable::contract::execute,
        cw20_mintable::contract::instantiate,
        cw20_mintable::contract::query,
    )));
    h.pair = h
        .app
        .instantiate_contract(
            pair_code,
            h.manager.clone(),
            &dex_common::pair::PairInstantiateMsg {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: h.token.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: h.quote.to_string(),
                    },
                ],
                fee_bps: 30,
                treasury: Addr::unchecked("treasury"),
                factory: h.factory.clone(),
                lp_token_code_id: lp_code,
                token_symbols: None,
                governance: h.manager.to_string(),
                max_batch_rungs: 20,
                discount_registry: None,
            },
            &[],
            "real-pair",
            None,
        )
        .unwrap();
    h.app
        .execute_contract(
            h.manager.clone(),
            h.factory.clone(),
            &h.pair.to_string(),
            &[],
        )
        .unwrap();
    if initialize {
        for token in [&h.token, &h.quote] {
            h.app
                .execute_contract(
                    h.manager.clone(),
                    token.clone(),
                    &Cw20ExecuteMsg::IncreaseAllowance {
                        spender: h.pair.to_string(),
                        amount: Uint128::new(5_000_000),
                        expires: None,
                    },
                    &[],
                )
                .unwrap();
        }
        h.app
            .execute_contract(
                h.manager.clone(),
                h.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: h.token.to_string(),
                            },
                            amount: Uint128::new(5_000_000),
                        },
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: h.quote.to_string(),
                            },
                            amount: Uint128::new(5_000_000),
                        },
                    ],
                    slippage_tolerance: Some(Decimal::percent(1)),
                    receiver: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap();
    }
    h.app
        .execute_contract(
            h.manager.clone(),
            h.autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some(h.pair.to_string()),
                router: None,
                quote_token: None,
                threshold: Some(Uint128::new(1_000)),
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap();
    let pair_info: PairInfo = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    h.lp = pair_info.liquidity_token;
    h
}

#[test]
fn lp1398_real_existing_pool_reconciles_lp_supply_and_receiver() {
    let mut h = real_pool_harness(true);
    fund_tax(&mut h, 10_000);
    let receiver_before = bal(&h.app, &h.lp, h.manager.as_str());
    let pool_before: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &dex_common::pair::QueryMsg::Pool {})
        .unwrap();
    h.app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
    let receiver_delta = bal(&h.app, &h.lp, h.manager.as_str()) - receiver_before;
    assert!(receiver_delta >= 4_950);
    let pool_after: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &dex_common::pair::QueryMsg::Pool {})
        .unwrap();
    assert_eq!(
        pool_after.total_share - pool_before.total_share,
        Uint128::new(receiver_delta)
    );
    assert_eq!(allowance(&h, &h.token), Uint128::zero());
    assert_eq!(allowance(&h, &h.quote), Uint128::zero());
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 0);
    assert_eq!(bal(&h.app, &h.quote, h.autolp.as_str()), 0);
    assert!(!cfg(&h).skimming);
}

#[test]
fn lp1398_real_first_liquidity_is_explicitly_unsupported_and_keeps_funds() {
    let mut h = real_pool_harness(false);
    fund_tax(&mut h, 10_000);
    let error = h
        .app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("requires an initialized CW20 pool"),
        "{error:?}"
    );
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 10_000);
    assert_eq!(bal(&h.app, &h.token, h.pair.as_str()), 0);
    assert!(!cfg(&h).skimming);
}

#[test]
fn lp1398_real_ratio_failure_rolls_back_swap_treasury_pool_supply_and_allowances() {
    let mut h = real_pool_harness(true);
    fund_tax(&mut h, 10_000);
    h.app
        .execute_contract(
            h.manager.clone(),
            h.quote.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.autolp.to_string(),
                amount: Uint128::new(1_000),
            },
            &[],
        )
        .unwrap();
    let pool_before: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &dex_common::pair::QueryMsg::Pool {})
        .unwrap();
    let lp_before = bal(&h.app, &h.lp, h.manager.as_str());
    let error = h
        .app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("Slippage"), "{error:?}");
    assert_eq!(bal(&h.app, &h.quote, "treasury"), 0);
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 10_000);
    assert_eq!(bal(&h.app, &h.quote, h.autolp.as_str()), 1_000);
    assert_eq!(bal(&h.app, &h.lp, h.manager.as_str()), lp_before);
    let pool_after: dex_common::pair::PoolResponse = h
        .app
        .wrap()
        .query_wasm_smart(&h.pair, &dex_common::pair::QueryMsg::Pool {})
        .unwrap();
    assert_eq!(pool_before, pool_after);
    assert_eq!(allowance(&h, &h.token), Uint128::zero());
    assert_eq!(allowance(&h, &h.quote), Uint128::zero());
    assert!(!cfg(&h).skimming);
}

#[test]
fn lp1398_zero_dust_and_odd_tax_balances_preserve_explicit_receiver() {
    for amount in [0, 1] {
        let mut h = setup(true);
        h.app
            .execute_contract(
                h.manager.clone(),
                h.autolp.clone(),
                &ExecuteMsg::UpdateConfig {
                    pair: None,
                    router: None,
                    quote_token: None,
                    threshold: Some(Uint128::zero()),
                    lp_recipient: None,
                    skim_max_spread: None,
                    skim_min_return: None,
                },
                &[],
            )
            .unwrap();
        if amount > 0 {
            fund_tax(&mut h, amount);
        }
        h.app
            .execute_contract(
                Addr::unchecked("keeper"),
                h.autolp.clone(),
                &ExecuteMsg::SkimToLp {},
                &[],
            )
            .unwrap();
        assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), amount);
        assert!(!cfg(&h).skimming);
    }
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
                lp_recipient: Some("recipient".into()),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap();
    set_swap_output(&mut h, 1_000_000);
    fund_tax(&mut h, 2_000_001);
    h.app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap();
    assert!(bal(&h.app, &h.lp, "recipient") >= 990_001);
    assert_eq!(bal(&h.app, &h.lp, h.manager.as_str()), 0);
    assert!(!cfg(&h).skimming);
}

#[test]
fn lp1398_real_whole_transaction_front_run_cannot_refresh_keeper_policy() {
    let mut h = real_pool_harness(true);
    let approved: Option<crate::state::LpPolicy> = h
        .app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetLpPolicy {})
        .unwrap();
    // A distinct transaction executes before skim. This is not interleaving
    // between the swap and its reply; the keeper must retain the manager anchor.
    h.app
        .execute_contract(
            h.manager.clone(),
            h.token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: h.pair.to_string(),
                amount: Uint128::new(100_000),
                msg: to_json_binary(&Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: Some(Decimal::percent(5)),
                    min_return: None,
                    to: Some(h.manager.to_string()),
                    deadline: None,
                    trader: None,
                    hybrid: Some(dex_common::pair::pool_only_hybrid_params(Uint128::new(
                        100_000,
                    ))),
                    greedy: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    fund_tax(&mut h, 10_000);
    let error = h
        .app
        .execute_contract(
            Addr::unchecked("keeper"),
            h.autolp.clone(),
            &ExecuteMsg::SkimToLp {},
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("changed beyond the approved policy tolerance"),
        "{error:?}"
    );
    let after: Option<crate::state::LpPolicy> = h
        .app
        .wrap()
        .query_wasm_smart(&h.autolp, &QueryMsg::GetLpPolicy {})
        .unwrap();
    assert_eq!(approved, after);
    assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 10_000);
}

#[test]
fn lp1398_legacy_migration_keeps_missing_anchor_and_rejects_wrong_version_identity() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    let mut deps = mock_dependencies();
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:cl8y-community-tax-autolp",
        "1.0.0",
    )
    .unwrap();
    crate::contract::migrate(
        deps.as_mut(),
        mock_env(),
        crate::msg::MigrateMsg { factory: None },
    )
    .unwrap();
    assert_eq!(
        cw2::get_contract_version(deps.as_ref().storage)
            .unwrap()
            .version,
        "1.1.0"
    );
    assert!(crate::state::LP_POLICY
        .may_load(deps.as_ref().storage)
        .unwrap()
        .is_none());
    for (name, version) in [
        ("foreign", "1.0.0"),
        ("crates.io:cl8y-community-tax-autolp", "1.2.0"),
        ("crates.io:cl8y-community-tax-autolp", "1.1.0"),
    ] {
        cw2::set_contract_version(deps.as_mut().storage, name, version).unwrap();
        assert!(crate::contract::migrate(
            deps.as_mut(),
            mock_env(),
            crate::msg::MigrateMsg { factory: None }
        )
        .is_err());
    }
}

#[test]
fn lp1398_provide_reentry_cannot_restart_skim_or_refresh_approved_policy() {
    for config_update in [false, true] {
        let mut h = setup(true);
        set_swap_output(&mut h, 1_000_000);
        fund_tax(&mut h, 2_000_000);
        h.app
            .execute_contract(
                h.manager.clone(),
                h.pair.clone(),
                &MockPairExecute::SetReentry {
                    target: h.autolp.to_string(),
                    config_update,
                },
                &[],
            )
            .unwrap();
        let error = h
            .app
            .execute_contract(
                Addr::unchecked("keeper"),
                h.autolp.clone(),
                &ExecuteMsg::SkimToLp {},
                &[],
            )
            .unwrap_err();
        assert!(
            format!("{error:?}").contains("Skim already in progress"),
            "{error:?}"
        );
        assert_eq!(bal(&h.app, &h.token, h.autolp.as_str()), 2_000_000);
        assert_eq!(allowance(&h, &h.token), Uint128::zero());
        assert!(!cfg(&h).skimming);
    }
}
