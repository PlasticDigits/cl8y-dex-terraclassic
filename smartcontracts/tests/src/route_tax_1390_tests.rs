//! #1390: repeated-token policy, authenticated owned refunds and guard parity.
use super::*;

fn issue1390_settings(f: &mut Fixture, token: Addr, settings: tax::SettingsBatch) {
    f.app
        .execute_contract(
            f.env.user.clone(),
            f.env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: token.to_string(),
                amount: Uint128::new(tax::INVOICE_UST1),
                msg: to_json_binary(&tax::InvoiceHookMsg::UpdateSettings { settings }).unwrap(),
            },
            &[],
        )
        .unwrap();
}

fn issue1390_path(tokens: &[Addr]) -> Vec<router::SwapOperation> {
    tokens
        .windows(2)
        .map(|t| router::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&t[0]),
            ask_asset_info: asset_info_token(&t[1]),
            hybrid: None,
            greedy: None,
            min_return: None,
        })
        .collect()
}

fn issue1390_effects(f: &Fixture) -> Vec<Uint128> {
    let accounts = [
        f.trader.clone(),
        f.env.router.clone(),
        f.env.treasury.clone(),
    ]
    .into_iter()
    .chain(f.pairs.iter().cloned())
    .collect::<Vec<_>>();
    assert!(
        f.app
            .wrap()
            .query_wasm_smart::<route_tax::RouteTaxContextResponse>(
                &f.env.router,
                &route_tax::RouterQueryMsg::RouteTaxContext {}
            )
            .is_err(),
        "completed or rejected route has no active context"
    );
    f.assets
        .iter()
        .flat_map(|t| accounts.iter().map(|a| query_cw20_balance(&f.app, t, a)))
        .chain(f.taxed.iter().map(|t| {
            f.app
                .wrap()
                .query_wasm_smart::<cw20::TokenInfoResponse>(t, &cw20::Cw20QueryMsg::TokenInfo {})
                .unwrap()
                .total_supply
        }))
        .collect()
}

#[test]
fn issue1390_cyclic_capable_assets_reject_in_execute_forward_and_reverse() {
    for cooldown in [0, 1, 10] {
        // Start/end repeat and a repeated intermediate, before or at final output.
        for indices in [vec![0, 1, 0], vec![0, 1, 2, 1], vec![0, 1, 2, 1, 3]] {
            for tax_free in [false, true] {
                let mut f = fixture(true, true, true, cooldown, false);
                if tax_free {
                    let trader = f.trader.to_string();
                    for token in f.taxed.clone() {
                        issue1390_settings(
                            &mut f,
                            token,
                            tax::SettingsBatch {
                                buy_bps: Some(0),
                                sell_bps: Some(0),
                                add_exempt: Some(vec![trader.clone()]),
                                ..Default::default()
                            },
                        );
                    }
                }
                let path = indices
                    .iter()
                    .map(|i| f.assets[*i].clone())
                    .collect::<Vec<_>>();
                let ops = issue1390_path(&path);
                for query in [
                    router::QueryMsg::SimulateSwapOperations {
                        offer_amount: Uint128::new(10_000),
                        operations: ops.clone(),
                        trader: Some(f.trader.to_string()),
                        sender: Some(f.env.router.to_string()),
                    },
                    router::QueryMsg::ReverseSimulateSwapOperations {
                        ask_amount: Uint128::new(10_000),
                        operations: ops.clone(),
                        trader: Some(f.trader.to_string()),
                        sender: Some(f.env.router.to_string()),
                    },
                ] {
                    let err = f
                        .app
                        .wrap()
                        .query_wasm_smart::<router::SimulateSwapOperationsResponse>(
                            &f.env.router,
                            &query,
                        )
                        .unwrap_err()
                        .to_string();
                    assert!(err.contains("repeated route-tax-capable asset"), "{err}");
                }
                let before = issue1390_effects(&f);
                let err = execute(&mut f, 10_000, ops, Uint128::zero()).unwrap_err();
                assert!(err.contains("repeated route-tax-capable asset"), "{err}");
                assert_eq!(issue1390_effects(&f), before, "cyclic failure is atomic");
                let ops = operations(&f, false, false, 10_000);
                let q = quote(&f, 10_000, ops.clone());
                execute(&mut f, 10_000, ops, q).unwrap();
            }
        }
    }
}

#[test]
fn issue1390_adjacent_buy_sell_checks_prior_trade_and_independent_routes() {
    for cooldown in [0, 1, 10] {
        for exempt in [false, true] {
            let mut f = fixture(false, false, false, cooldown, false);
            if exempt {
                let trader = f.trader.to_string();
                let token = f.taxed[0].clone();
                issue1390_settings(
                    &mut f,
                    token,
                    tax::SettingsBatch {
                        add_exempt: Some(vec![trader]),
                        ..Default::default()
                    },
                );
            }
            let ops = operations(&f, false, false, 10_000);
            let q = quote(&f, 10_000, ops.clone());
            execute(&mut f, 10_000, ops.clone(), q).unwrap();
            let before = issue1390_effects(&f);
            let next = execute(&mut f, 10_000, ops.clone(), Uint128::zero());
            if cooldown == 0 {
                next.unwrap();
            } else {
                assert!(next.unwrap_err().contains("cooldown"));
                assert_eq!(issue1390_effects(&f), before);
                f.app.update_block(|b| b.height += cooldown - 1);
                assert!(execute(&mut f, 10_000, ops.clone(), Uint128::zero())
                    .unwrap_err()
                    .contains("cooldown"));
                f.app.update_block(|b| b.height += 1);
                let q = quote(&f, 10_000, ops.clone());
                execute(&mut f, 10_000, ops, q).unwrap();
            }
        }
    }
}

#[test]
fn issue1390_original_owned_input_residue_restores_above_lowered_max_wallet() {
    let mut f = fixture(true, false, false, 1, false);
    let token = f.assets[0].clone();
    issue1390_settings(
        &mut f,
        token,
        tax::SettingsBatch {
            launch_guards: Some(tax::LaunchGuardsConfig {
                max_wallet: Some(Uint128::new(100)),
                cooldown_blocks: 1,
                trading_enabled: true,
            }),
            ..Default::default()
        },
    );
    // G=100 has S=99, tax=0 and one unit of the original debit to restore.
    let gross = 100;
    let ops = operations(&f, false, false, gross);
    let q = quote(&f, gross, ops.clone());
    let before = query_cw20_balance(&f.app, &f.assets[0], &f.trader);
    execute(&mut f, gross, ops, q).unwrap();
    assert_eq!(
        query_cw20_balance(&f.app, &f.assets[0], &f.trader),
        before - Uint128::new(99)
    );
    assert_eq!(
        query_cw20_balance(&f.app, &f.assets[0], &f.env.router),
        Uint128::zero()
    );
}

#[test]
fn issue1390_purchased_intermediate_residue_and_final_output_stay_capped() {
    for final_output in [false, true] {
        for exempt in [false, true] {
            let mut f = fixture(false, false, true, 1, true);
            let token = if final_output {
                f.assets[2].clone()
            } else {
                f.assets[1].clone()
            };
            f.app
                .execute_contract(
                    f.env.user.clone(),
                    token.clone(),
                    &cw20::Cw20ExecuteMsg::Transfer {
                        recipient: f.trader.to_string(),
                        amount: Uint128::new(1_000),
                    },
                    &[],
                )
                .unwrap();
            let trader = f.trader.to_string();
            issue1390_settings(
                &mut f,
                token.clone(),
                tax::SettingsBatch {
                    launch_guards: Some(tax::LaunchGuardsConfig {
                        max_wallet: Some(Uint128::new(100)),
                        cooldown_blocks: 1,
                        trading_enabled: true,
                    }),
                    // Exempt the final output only: exempting the intermediate sell
                    // removes its integer residue, which cannot exercise this guard.
                    add_exempt: if exempt && final_output {
                        Some(vec![trader])
                    } else {
                        None
                    },
                    ..Default::default()
                },
            );
            // Find a small quote whose acquired intermediate leaves one purchased unit.
            let gross = if final_output {
                10_000
            } else {
                (100..1_000)
                    .find(|g| {
                        let mid = quote(&f, *g, vec![operations(&f, false, false, *g)[0].clone()]);
                        let p = route_tax::sell_from_gross(mid, 100).unwrap();
                        mid > p.economic + p.tax
                    })
                    .unwrap()
            };
            let ops = operations(&f, false, false, gross);
            let q = quote(&f, gross, ops.clone());
            let before = issue1390_effects(&f);
            assert!(execute(&mut f, gross, ops.clone(), q)
                .unwrap_err()
                .contains("Max wallet"));
            assert_eq!(
                issue1390_effects(&f),
                before,
                "cap failure rolls all hops and taxes back"
            );
            issue1390_settings(
                &mut f,
                token,
                tax::SettingsBatch {
                    launch_guards: Some(tax::LaunchGuardsConfig {
                        max_wallet: None,
                        cooldown_blocks: 1,
                        trading_enabled: true,
                    }),
                    ..Default::default()
                },
            );
            // The failed route must not leave an acquired marker or cooldown behind.
            let q = quote(&f, gross, ops.clone());
            execute(&mut f, gross, ops, q).unwrap();
        }
    }
}

#[test]
fn issue1390_protocol_exempt_trader_preview_cannot_quote_unexecutable_sell() {
    let f = fixture(false, false, false, 0, false);
    for trader in [&f.env.router, &f.pairs[0], &f.taxed[0], &f.env.factory] {
        for kind in [route_tax::RouteTaxKind::Buy, route_tax::RouteTaxKind::Sell] {
            let err = f
                .app
                .wrap()
                .query_wasm_smart::<route_tax::RouteTaxPreviewResponse>(
                    &f.taxed[0],
                    &route_tax::TokenQueryMsg::RouteTaxPreview {
                        kind,
                        amount: Uint128::new(10_000),
                        trader: Some(trader.to_string()),
                        pair: f.pairs[0].to_string(),
                    },
                )
                .unwrap_err()
                .to_string();
            assert!(err.contains("trader"), "{err}");
        }
        let ops = operations(&f, false, false, 10_000);
        for query in [
            router::QueryMsg::SimulateSwapOperations {
                offer_amount: Uint128::new(10_000),
                operations: ops.clone(),
                trader: Some(trader.to_string()),
                sender: Some(f.env.router.to_string()),
            },
            router::QueryMsg::ReverseSimulateSwapOperations {
                ask_amount: Uint128::new(10_000),
                operations: ops.clone(),
                trader: Some(trader.to_string()),
                sender: Some(f.env.router.to_string()),
            },
        ] {
            let err = f
                .app
                .wrap()
                .query_wasm_smart::<router::SimulateSwapOperationsResponse>(&f.env.router, &query)
                .unwrap_err()
                .to_string();
            assert!(err.contains("trader"), "{err}");
        }
    }
}

#[test]
fn issue1390_ordinary_asset_cycle_keeps_single_capable_intermediate_allowed() {
    let mut f = fixture(false, false, false, 10, false);
    let ops = issue1390_path(&[
        f.assets[0].clone(),
        f.assets[1].clone(),
        f.assets[0].clone(),
    ]);
    let q = quote(&f, 10_000, ops.clone());
    let before = query_cw20_balance(&f.app, &f.assets[0], &f.trader);
    let response = execute_to_response(&mut f, 10_000, ops, q, None).unwrap();
    let actual_output: Uint128 = response
        .events
        .iter()
        .find_map(|event| {
            if event
                .attributes
                .iter()
                .any(|a| a.key == "action" && a.value == "swap_complete")
            {
                event
                    .attributes
                    .iter()
                    .find(|a| a.key == "output_amount")
                    .map(|a| a.value.parse().unwrap())
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        query_cw20_balance(&f.app, &f.assets[0], &f.trader),
        before - Uint128::new(10_000) + actual_output
    );
    // A roundtrip through the same ordinary pair changes reserves between hops;
    // its independent static simulations need not equal the executed output.
    // The control proves the policy permits it and honors the quoted minimum.
    assert!(
        query_cw20_balance(&f.app, &f.assets[0], &f.trader) >= before - Uint128::new(10_000) + q
    );
    assert_eq!(
        query_cw20_balance(&f.app, &f.assets[0], &f.env.router),
        Uint128::zero()
    );
}

#[test]
fn issue1390_third_party_output_capped_and_owned_refund_stays_with_payer() {
    let mut f = fixture(true, false, true, 1, false);
    let recipient = Addr::unchecked("synthetic_recipient");
    let original = f.assets[0].clone();
    let final_token = f.assets[2].clone();
    f.app
        .execute_contract(
            f.env.user.clone(),
            final_token.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::new(1_000),
            },
            &[],
        )
        .unwrap();
    let trader = f.trader.to_string();
    for token in [original.clone(), final_token.clone()] {
        issue1390_settings(
            &mut f,
            token.clone(),
            tax::SettingsBatch {
                launch_guards: Some(tax::LaunchGuardsConfig {
                    max_wallet: Some(Uint128::new(100)),
                    cooldown_blocks: 1,
                    trading_enabled: true,
                }),
                add_exempt: if token == final_token {
                    Some(vec![trader.clone()])
                } else {
                    None
                },
                ..Default::default()
            },
        );
    }
    let ops = operations(&f, false, false, 100);
    let q = quote(&f, 100, ops.clone());
    let before = issue1390_effects(&f);
    let recipient_before = query_cw20_balance(&f.app, &final_token, &recipient);
    assert!(
        execute_to(&mut f, 100, ops.clone(), q, Some(recipient.to_string()))
            .unwrap_err()
            .contains("Max wallet")
    );
    assert_eq!(issue1390_effects(&f), before);
    assert_eq!(
        query_cw20_balance(&f.app, &final_token, &recipient),
        recipient_before
    );
    issue1390_settings(
        &mut f,
        final_token.clone(),
        tax::SettingsBatch {
            launch_guards: Some(tax::LaunchGuardsConfig {
                max_wallet: None,
                cooldown_blocks: 1,
                trading_enabled: true,
            }),
            ..Default::default()
        },
    );
    let input_before = query_cw20_balance(&f.app, &original, &f.trader);
    execute_to(&mut f, 100, ops, q, Some(recipient.to_string())).unwrap();
    assert_eq!(
        query_cw20_balance(&f.app, &original, &f.trader),
        input_before - Uint128::new(99)
    );
    assert_eq!(
        query_cw20_balance(&f.app, &original, &recipient),
        Uint128::zero()
    );
    assert_eq!(
        query_cw20_balance(&f.app, &final_token, &recipient),
        recipient_before + q
    );
    assert_eq!(
        query_cw20_balance(&f.app, &final_token, &f.trader),
        Uint128::zero()
    );
}

#[test]
fn issue1390_cyclic_capability_policy_survives_disabled_launch_guard_sku() {
    let mut f = fixture_features(true, true, true, 0, false, false, false);
    for token in &f.taxed {
        let features: tax::FeaturesResponse = f
            .app
            .wrap()
            .query_wasm_smart(token, &tax::QueryMsg::GetFeatures {})
            .unwrap();
        assert!(!features.launch_guards);
    }
    let ops = issue1390_path(&[
        f.assets[0].clone(),
        f.assets[1].clone(),
        f.assets[0].clone(),
    ]);
    for query in [
        router::QueryMsg::SimulateSwapOperations {
            offer_amount: Uint128::new(10_000),
            operations: ops.clone(),
            trader: Some(f.trader.to_string()),
            sender: Some(f.env.router.to_string()),
        },
        router::QueryMsg::ReverseSimulateSwapOperations {
            ask_amount: Uint128::new(10_000),
            operations: ops.clone(),
            trader: Some(f.trader.to_string()),
            sender: Some(f.env.router.to_string()),
        },
    ] {
        assert!(f
            .app
            .wrap()
            .query_wasm_smart::<router::SimulateSwapOperationsResponse>(&f.env.router, &query)
            .unwrap_err()
            .to_string()
            .contains("repeated route-tax-capable asset"));
    }
    let before = issue1390_effects(&f);
    assert!(execute(&mut f, 10_000, ops, Uint128::zero())
        .unwrap_err()
        .contains("repeated route-tax-capable asset"));
    assert_eq!(issue1390_effects(&f), before);
}

#[test]
fn issue1390_prior_direct_trade_cooldown_cannot_be_waived_by_route_continuation() {
    for cooldown in [1, 10] {
        for sell in [false, true] {
            let mut f = fixture(false, false, false, cooldown, false);
            if sell {
                f.app
                    .execute_contract(
                        f.env.user.clone(),
                        f.assets[1].clone(),
                        &cw20::Cw20ExecuteMsg::Transfer {
                            recipient: f.trader.to_string(),
                            amount: Uint128::new(20_000),
                        },
                        &[],
                    )
                    .unwrap();
            }
            f.app
                .execute_contract(
                    f.trader.clone(),
                    f.assets[usize::from(sell)].clone(),
                    &cw20::Cw20ExecuteMsg::Send {
                        contract: f.pairs[0].to_string(),
                        amount: Uint128::new(10_000),
                        msg: to_json_binary(&pair::Cw20HookMsg::Swap {
                            belief_price: None,
                            max_spread: Some(Decimal::percent(50)),
                            min_return: Some(Uint128::one()),
                            to: None,
                            deadline: None,
                            trader: None,
                            hybrid: None,
                            greedy: None,
                        })
                        .unwrap(),
                    },
                    &[],
                )
                .unwrap();
            let ops = operations(&f, false, false, 10_000);
            let before = issue1390_effects(&f);
            assert!(execute(&mut f, 10_000, ops.clone(), Uint128::zero())
                .unwrap_err()
                .contains("cooldown"));
            assert_eq!(issue1390_effects(&f), before);
            f.app.update_block(|b| b.height += cooldown);
            let q = quote(&f, 10_000, ops.clone());
            execute(&mut f, 10_000, ops, q).unwrap();
        }
    }
}

#[test]
fn issue1390_protocol_trader_execution_rejects_at_authenticated_buy_boundary() {
    for token_self in [false, true] {
        let mut f = fixture(false, false, false, 1, false);
        f.trader = if token_self {
            f.taxed[0].clone()
        } else {
            f.env.factory.clone()
        };
        f.app
            .execute_contract(
                f.env.user.clone(),
                f.assets[0].clone(),
                &cw20::Cw20ExecuteMsg::Transfer {
                    recipient: f.trader.to_string(),
                    amount: Uint128::new(10_000),
                },
                &[],
            )
            .unwrap();
        let ops = operations(&f, false, false, 10_000);
        let before = issue1390_effects(&f);
        assert!(execute(&mut f, 10_000, ops, Uint128::zero())
            .unwrap_err()
            .contains("trader"));
        assert_eq!(issue1390_effects(&f), before);
    }
}
