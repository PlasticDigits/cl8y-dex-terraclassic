//! #1381: real token + router + factory + pair execution, no wallet intermediates.
use crate::helpers::*;
use cl8y_community_tax_token::msg as tax;
use cl8y_dex_router::msg as router;
use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use cw_multi_test::{App, ContractWrapper, Executor};
use dex_common::{pair, route_tax, types::Asset};

struct Fixture {
    app: App,
    env: TestEnv,
    trader: Addr,
    assets: Vec<Addr>,
    pairs: Vec<Addr>,
    taxed: Vec<Addr>,
}
fn fixture(first_tax: bool, two_tax: bool, final_tax: bool, cooldown: u64, burn: bool) -> Fixture {
    fixture_manager(first_tax, two_tax, final_tax, cooldown, burn, false)
}
fn fixture_manager(
    first_tax: bool,
    two_tax: bool,
    final_tax: bool,
    cooldown: u64,
    burn: bool,
    router_manager: bool,
) -> Fixture {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let trader = Addr::unchecked("synthetic_trader");
    let code = app.store_code(Box::new(ContractWrapper::new(
        cl8y_community_tax_token::contract::execute,
        cl8y_community_tax_token::contract::instantiate,
        cl8y_community_tax_token::contract::query,
    )));
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: code },
        &[],
    )
    .unwrap();
    let mut taxed = vec![];
    let mut make_tax = |name: &str| {
        let t = app
            .instantiate_contract(
                code,
                env.user.clone(),
                &tax::InstantiateMsg {
                    name: name.into(),
                    symbol: name.into(),
                    decimals: 6,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: env.user.to_string(),
                        amount: Uint128::new(1_000_000_000_000),
                    }],
                    marketing: None,
                    manager: if router_manager {
                        env.router.to_string()
                    } else {
                        env.user.to_string()
                    },
                    treasury: env.treasury.to_string(),
                    buy_bps: 450,
                    sell_bps: 100,
                    max_buy_bps: 450,
                    max_sell_bps: 100,
                    max_transfer_bps: 0,
                    factory: env.factory.to_string(),
                    router: Some(env.router.to_string()),
                    ust1: env.token_b.to_string(),
                    cmm_treasury: env.treasury.to_string(),
                    features: [
                        if cooldown > 0 {
                            Some(tax::Sku::LaunchGuards)
                        } else {
                            None
                        },
                        if burn {
                            Some(tax::Sku::SplitRouter)
                        } else {
                            None
                        },
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                    mint: None,
                    transfer_bps: None,
                    sinks: if burn {
                        Some(vec![tax::Sink {
                            kind: tax::SinkKind::Burn,
                            addr: None,
                            bps: 10_000,
                        }])
                    } else {
                        None
                    },
                    autolp: None,
                    launcher: None,
                    launch_guards: if cooldown > 0 {
                        Some(tax::LaunchGuardsConfig {
                            max_wallet: None,
                            cooldown_blocks: cooldown,
                            trading_enabled: true,
                        })
                    } else {
                        None
                    },
                    initial_exempt: None,
                },
                &[],
                name,
                None,
            )
            .unwrap();
        taxed.push(t.clone());
        t
    };
    let initial = if first_tax {
        make_tax("PAYTAX")
    } else {
        env.token_a.clone()
    };
    let middle = make_tax("ALPHA");
    let second = if two_tax {
        make_tax("BETA")
    } else {
        env.token_b.clone()
    };
    let last = if final_tax {
        make_tax("LAST")
    } else {
        env.token_b.clone()
    };
    let assets = if two_tax {
        vec![initial, middle, second, last]
    } else {
        vec![initial, middle, last]
    };
    let mut pairs = vec![];
    for tokens in assets.windows(2) {
        app.update_block(|b| {
            b.height += 1;
            b.time = b.time.plus_seconds(5);
        });
        let resp = app
            .execute_contract(
                env.user.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [asset_info_token(&tokens[0]), asset_info_token(&tokens[1])],
                },
                &[],
            )
            .unwrap();
        let p = extract_pair_address(&resp.events);
        for token in tokens {
            if taxed.contains(token) {
                app.execute_contract(
                    env.user.clone(),
                    token.clone(),
                    &tax::ExecuteMsg::RegisterListedPair {
                        pair: p.to_string(),
                    },
                    &[],
                )
                .unwrap();
            }
            app.execute_contract(
                env.user.clone(),
                token.clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: p.to_string(),
                    amount: Uint128::new(10_000_000),
                    expires: None,
                },
                &[],
            )
            .unwrap();
        }
        app.execute_contract(
            env.user.clone(),
            p.clone(),
            &pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: asset_info_token(&tokens[0]),
                        amount: Uint128::new(10_000_000),
                    },
                    Asset {
                        info: asset_info_token(&tokens[1]),
                        amount: Uint128::new(10_000_000),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap();
        pairs.push(p);
    }
    app.execute_contract(
        env.user.clone(),
        assets[0].clone(),
        &cw20::Cw20ExecuteMsg::Transfer {
            recipient: trader.to_string(),
            amount: Uint128::new(2_000_000),
        },
        &[],
    )
    .unwrap();
    Fixture {
        app,
        env,
        trader,
        assets,
        pairs,
        taxed,
    }
}
fn operations(f: &Fixture, greedy: bool, hybrid: bool, gross: u128) -> Vec<router::SwapOperation> {
    f.assets
        .windows(2)
        .enumerate()
        .map(|(i, t)| router::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&t[0]),
            ask_asset_info: asset_info_token(&t[1]),
            hybrid: if hybrid && i == 0 {
                Some(pair::pool_only_hybrid_params(Uint128::new(gross)))
            } else {
                None
            },
            greedy: if greedy {
                Some(pair::greedy_swap_params(20, None))
            } else {
                None
            },
            min_return: if greedy { Some(Uint128::one()) } else { None },
        })
        .collect()
}
fn quote(f: &Fixture, gross: u128, ops: Vec<router::SwapOperation>) -> Uint128 {
    f.app
        .wrap()
        .query_wasm_smart::<router::SimulateSwapOperationsResponse>(
            &f.env.router,
            &router::QueryMsg::SimulateSwapOperations {
                offer_amount: Uint128::new(gross),
                operations: ops,
                trader: Some(f.trader.to_string()),
                sender: Some(f.env.router.to_string()),
            },
        )
        .unwrap()
        .amount
}
fn execute(
    f: &mut Fixture,
    gross: u128,
    ops: Vec<router::SwapOperation>,
    min: Uint128,
) -> Result<(), String> {
    f.app
        .execute_contract(
            f.trader.clone(),
            f.assets[0].clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: f.env.router.to_string(),
                amount: Uint128::new(gross),
                msg: to_json_binary(&router::Cw20HookMsg::ExecuteRouteSwapOperations {
                    funding_version: 1,
                    operations: ops,
                    max_spread: Decimal::percent(50),
                    minimum_receive: Some(min),
                    to: None,
                    deadline: None,
                    unwrap_output: None,
                })
                .unwrap(),
            },
            &[],
        )
        .map(|_| ())
        .map_err(|e| e.root_cause().to_string())
}

#[test]
fn issue1381_zero_intermediate_wallet_quote_execute_and_dust_isolation() {
    for (first, two, final_tax, greedy, hybrid, burn) in [
        (false, false, false, false, false, false),
        (true, true, true, false, true, false),
        (false, true, true, true, false, true),
    ] {
        let mut f = fixture(first, two, final_tax, 0, burn);
        let gross = 100_000;
        let ops = operations(&f, greedy, hybrid, gross);
        for t in f.assets.iter().skip(1) {
            assert_eq!(query_cw20_balance(&f.app, t, &f.trader), Uint128::zero());
        }
        // Existing router dust is deliberately distinct from every hop's output.
        for t in &f.assets {
            f.app
                .execute_contract(
                    f.env.user.clone(),
                    t.clone(),
                    &cw20::Cw20ExecuteMsg::Transfer {
                        recipient: f.env.router.to_string(),
                        amount: Uint128::new(123),
                    },
                    &[],
                )
                .unwrap();
        }
        let dust = f
            .assets
            .iter()
            .map(|t| query_cw20_balance(&f.app, t, &f.env.router))
            .collect::<Vec<_>>();
        let q = quote(&f, gross, ops.clone());
        assert!(q > Uint128::zero());
        let mut current = Uint128::new(gross);
        let mut expected_taxes = vec![Uint128::zero(); f.taxed.len()];
        let mut expected_commissions = vec![Uint128::zero(); f.taxed.len()];
        let mut economic_inputs = vec![];
        for (i, pair_addr) in f.pairs.iter().enumerate() {
            let mut economic = current;
            if let Some(j) = f.taxed.iter().position(|t| t == f.assets[i]) {
                let p: route_tax::RouteTaxPreviewResponse = f
                    .app
                    .wrap()
                    .query_wasm_smart(
                        &f.assets[i],
                        &route_tax::TokenQueryMsg::RouteTaxPreview {
                            kind: route_tax::RouteTaxKind::Sell,
                            amount: current,
                            trader: Some(f.trader.to_string()),
                            pair: pair_addr.to_string(),
                        },
                    )
                    .unwrap();
                economic = p.economic;
                expected_taxes[j] += p.tax;
            }
            economic_inputs.push(economic);
            let sim: pair::HybridSimulationResponse = f
                .app
                .wrap()
                .query_wasm_smart(
                    pair_addr,
                    &pair::QueryMsg::HybridSimulation {
                        offer_asset: Asset {
                            info: asset_info_token(&f.assets[i]),
                            amount: economic,
                        },
                        hybrid: Some(pair::pool_only_hybrid_params(economic)),
                        greedy: None,
                        trader: Some(f.trader.to_string()),
                        sender: Some(f.env.router.to_string()),
                        belief_price: None,
                    },
                )
                .unwrap();
            current = sim.return_amount;
            if let Some(j) = f.taxed.iter().position(|t| t == f.assets[i + 1]) {
                let p: route_tax::RouteTaxPreviewResponse = f
                    .app
                    .wrap()
                    .query_wasm_smart(
                        &f.assets[i + 1],
                        &route_tax::TokenQueryMsg::RouteTaxPreview {
                            kind: route_tax::RouteTaxKind::Buy,
                            amount: current,
                            trader: Some(f.trader.to_string()),
                            pair: pair_addr.to_string(),
                        },
                    )
                    .unwrap();
                expected_taxes[j] += p.tax;
                // Existing direct pair→treasury buy classification remains; the
                // commission recipient and treasury sink are the same account.
                if burn {
                    expected_taxes[j] += sim.commission_amount.multiply_ratio(450u128, 10_000u128);
                }
                expected_commissions[j] += sim.commission_amount;
                current = p.net;
            }
        }
        assert_eq!(current, q);
        let pair_inputs_before = f
            .pairs
            .iter()
            .enumerate()
            .map(|(i, p)| query_cw20_balance(&f.app, &f.assets[i], p))
            .collect::<Vec<_>>();
        let treasury_before = f
            .taxed
            .iter()
            .map(|t| query_cw20_balance(&f.app, t, &f.env.treasury))
            .collect::<Vec<_>>();
        let input_before = query_cw20_balance(&f.app, &f.assets[0], &f.trader);
        let supply_before = f
            .taxed
            .iter()
            .map(|t| {
                f.app
                    .wrap()
                    .query_wasm_smart::<cw20::TokenInfoResponse>(
                        t,
                        &cw20::Cw20QueryMsg::TokenInfo {},
                    )
                    .unwrap()
                    .total_supply
            })
            .collect::<Vec<_>>();
        execute(&mut f, gross, ops, q).unwrap();
        assert_eq!(
            query_cw20_balance(&f.app, f.assets.last().unwrap(), &f.trader),
            q
        );
        assert!(
            input_before
                .checked_sub(query_cw20_balance(&f.app, &f.assets[0], &f.trader))
                .unwrap()
                <= Uint128::new(gross)
        );
        for (i, t) in f.assets.iter().enumerate() {
            assert_eq!(query_cw20_balance(&f.app, t, &f.env.router), dust[i]);
        }
        for t in f.assets.iter().skip(1).take(f.assets.len() - 2) {
            assert!(
                query_cw20_balance(&f.app, t, &f.trader) <= Uint128::one(),
                "only owned rounding refund"
            );
        }
        for (i, p) in f.pairs.iter().enumerate() {
            assert_eq!(
                query_cw20_balance(&f.app, &f.assets[i], p)
                    .checked_sub(pair_inputs_before[i])
                    .unwrap(),
                economic_inputs[i],
                "pair credit equals economic hook amount"
            );
        }
        for (i, t) in f.taxed.iter().enumerate() {
            if burn {
                assert_eq!(
                    supply_before[i]
                        .checked_sub(
                            f.app
                                .wrap()
                                .query_wasm_smart::<cw20::TokenInfoResponse>(
                                    t,
                                    &cw20::Cw20QueryMsg::TokenInfo {}
                                )
                                .unwrap()
                                .total_supply
                        )
                        .unwrap(),
                    expected_taxes[i],
                    "every configured tax burns exactly once"
                );
            } else {
                assert_eq!(
                    query_cw20_balance(&f.app, t, &f.env.treasury)
                        .checked_sub(treasury_before[i])
                        .unwrap(),
                    expected_taxes[i] + expected_commissions[i],
                    "configured tax sinks plus pool fees reconcile"
                );
            }
        }
        assert!(f
            .app
            .wrap()
            .query_wasm_smart::<route_tax::RouteTaxContextResponse>(
                &f.env.router,
                &route_tax::RouterQueryMsg::RouteTaxContext {}
            )
            .is_err());
    }
}
#[test]
fn issue1381_reverse_sufficient_gross_and_atomic_min_return_rollback() {
    let mut f = fixture(true, true, true, 0, false);
    let ops = operations(&f, false, false, 100_000);
    let net = quote(&f, 100_000, ops.clone());
    let reverse = f
        .app
        .wrap()
        .query_wasm_smart::<router::SimulateSwapOperationsResponse>(
            &f.env.router,
            &router::QueryMsg::ReverseSimulateSwapOperations {
                ask_amount: net,
                operations: ops.clone(),
                trader: Some(f.trader.to_string()),
                sender: Some(f.env.router.to_string()),
            },
        )
        .unwrap()
        .amount;
    assert!(quote(&f, reverse.u128(), ops.clone()) >= net);
    let accounts = [
        f.trader.clone(),
        f.env.router.clone(),
        f.env.treasury.clone(),
    ];
    let before = f
        .assets
        .iter()
        .flat_map(|t| accounts.iter().map(|a| query_cw20_balance(&f.app, t, a)))
        .collect::<Vec<_>>();
    assert!(execute(&mut f, 100_000, ops.clone(), net + Uint128::one())
        .unwrap_err()
        .contains("Minimum"));
    let after = f
        .assets
        .iter()
        .flat_map(|t| accounts.iter().map(|a| query_cw20_balance(&f.app, t, a)))
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    execute(&mut f, 100_000, ops, net).unwrap();
}
#[test]
fn issue1381_authenticated_context_legacy_rejection_and_route_cooldown() {
    let mut f = fixture(false, false, false, 10, false);
    let ops = operations(&f, false, false, 100_000);
    let q = quote(&f, 100_000, ops.clone());
    // Intra-route buy then sell succeeds; an independent route in the same block cannot skip cooldown.
    execute(&mut f, 100_000, ops.clone(), q).unwrap();
    assert!(execute(&mut f, 100_000, ops.clone(), Uint128::zero())
        .unwrap_err()
        .contains("cooldown"));
    let t = f.taxed[0].clone();
    assert!(f
        .app
        .execute_contract(
            f.trader.clone(),
            t.clone(),
            &tax::ExecuteMsg::RouteTransfer {
                recipient: f.trader.to_string(),
                amount: Uint128::one()
            },
            &[]
        )
        .unwrap_err()
        .root_cause()
        .to_string()
        .contains("Unauthorized"));
    assert!(f
        .app
        .execute_contract(
            f.env.router.clone(),
            t,
            &tax::ExecuteMsg::Send {
                contract: f.pairs[1].to_string(),
                amount: Uint128::one(),
                msg: to_json_binary(&pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: Some(Decimal::percent(50)),
                    min_return: None,
                    to: None,
                    deadline: None,
                    trader: Some(f.trader.to_string()),
                    hybrid: None,
                    greedy: None
                })
                .unwrap()
            },
            &[]
        )
        .unwrap_err()
        .root_cause()
        .to_string()
        .contains("Route-local"));
}

#[test]
fn issue1381_book_fill_and_post_tax_hybrid_allocation_execute_parity() {
    for greedy in [false, true] {
        let mut f = fixture(false, false, false, 0, false);
        let info: dex_common::types::PairInfo = f
            .app
            .wrap()
            .query_wasm_smart(&f.pairs[1], &pair::QueryMsg::Pair {})
            .unwrap();
        let maker_asset = asset_info_token(&f.assets[2]);
        let side = if info.asset_infos[0] == maker_asset {
            pair::LimitOrderSide::Ask
        } else {
            pair::LimitOrderSide::Bid
        };
        f.app
            .execute_contract(
                f.env.user.clone(),
                f.assets[2].clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: f.pairs[1].to_string(),
                    amount: Uint128::new(40_000),
                    msg: to_json_binary(&pair::Cw20HookMsg::PlaceLimitOrderBatch {
                        side,
                        orders: vec![pair::LimitOrderPlacementItem {
                            price: Decimal::one(),
                            amount: Uint128::new(40_000),
                            max_adjust_steps: 32,
                            expires_at: None,
                            hint_after_order_id: None,
                        }],
                    })
                    .unwrap(),
                },
                &[],
            )
            .unwrap();
        let mut ops = operations(&f, false, false, 100_000);
        let middle_gross = quote(&f, 100_000, vec![ops[0].clone()]);
        if let router::SwapOperation::TerraSwap {
            hybrid,
            greedy: g,
            min_return,
            ..
        } = &mut ops[1]
        {
            *min_return = Some(Uint128::one());
            if greedy {
                *g = Some(pair::greedy_swap_params(20, None));
            } else {
                *hybrid = Some(pair::HybridSwapParams {
                    pool_input: middle_gross / Uint128::new(2),
                    book_input: middle_gross - middle_gross / Uint128::new(2),
                    max_maker_fills: 20,
                    book_start_hint: None,
                });
            }
        }
        let q = quote(&f, 100_000, ops.clone());
        let reverse = f
            .app
            .wrap()
            .query_wasm_smart::<router::SimulateSwapOperationsResponse>(
                &f.env.router,
                &router::QueryMsg::ReverseSimulateSwapOperations {
                    ask_amount: q,
                    operations: ops.clone(),
                    trader: Some(f.trader.to_string()),
                    sender: Some(f.env.router.to_string()),
                },
            )
            .unwrap()
            .amount;
        let mut reverse_ops = ops.clone();
        if !greedy {
            // Explicit reverse hybrid ratios are templates; rebuild the frozen
            // forward allocation against the quoted gross preceding-hop output.
            let g = quote(&f, reverse.u128(), vec![reverse_ops[0].clone()]);
            if let router::SwapOperation::TerraSwap {
                hybrid: Some(h), ..
            } = &mut reverse_ops[1]
            {
                h.pool_input = g / Uint128::new(2);
                h.book_input = g - h.pool_input;
            }
        }
        assert!(
            quote(&f, reverse.u128(), reverse_ops) >= q,
            "reverse funds actual book and token taxes"
        );
        execute(&mut f, 100_000, ops, q).unwrap();
        assert_eq!(
            query_cw20_balance(&f.app, f.assets.last().unwrap(), &f.trader),
            q
        );
        assert!(query_cw20_balance(&f.app, &f.assets[1], &f.trader) <= Uint128::one());
    }
}

#[test]
fn issue1381_route_sinks_cannot_pollute_pair_credit_or_router_budget() {
    for kind in [tax::SinkKind::Wallet, tax::SinkKind::Treasury] {
        let mut f = fixture(false, false, false, 0, true);
        let bad_pair = f.pairs[1].to_string();
        let settings = tax::SettingsBatch {
            treasury: Some(bad_pair.clone()),
            sinks: Some(vec![tax::Sink {
                kind: kind.clone(),
                addr: if matches!(kind, tax::SinkKind::Wallet) {
                    Some(bad_pair)
                } else {
                    None
                },
                bps: 10_000,
            }]),
            ..Default::default()
        };
        f.app
            .execute_contract(
                f.env.user.clone(),
                f.env.token_b.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: f.taxed[0].to_string(),
                    amount: Uint128::new(tax::INVOICE_UST1),
                    msg: to_json_binary(&tax::InvoiceHookMsg::UpdateSettings { settings }).unwrap(),
                },
                &[],
            )
            .unwrap();
        let before = query_cw20_balance(&f.app, &f.assets[0], &f.trader);
        let ops = operations(&f, false, false, 100_000);
        assert!(execute(&mut f, 100_000, ops, Uint128::zero())
            .unwrap_err()
            .contains("Route-local"));
        assert_eq!(query_cw20_balance(&f.app, &f.assets[0], &f.trader), before);
    }
}

#[test]
fn issue1381_exemptions_bind_only_the_trader_for_each_route_token() {
    // A token manager role on the protocol router cannot waive another user's
    // buy/sell taxes or change the gross/economic preview accounting.
    let mut f = fixture_manager(false, false, false, 0, false, true);
    let ops = operations(&f, false, false, 100_000);
    let q = quote(&f, 100_000, ops.clone());
    execute(&mut f, 100_000, ops, q).unwrap();
    assert_eq!(
        query_cw20_balance(&f.app, f.assets.last().unwrap(), &f.trader),
        q
    );
    assert!(query_cw20_balance(&f.app, &f.taxed[0], &f.env.treasury) > Uint128::zero());

    // A valid trader manager exemption is token-specific and quote/execute agree.
    let mut f = fixture(false, true, true, 0, false);
    let ops = operations(&f, false, false, 100_000);
    let conservative = quote(&f, 100_000, ops.clone());
    f.trader = f.env.user.clone();
    let q = quote(&f, 100_000, ops.clone());
    assert!(q > conservative);
    let before = f
        .assets
        .iter()
        .skip(1)
        .map(|t| query_cw20_balance(&f.app, t, &f.trader))
        .collect::<Vec<_>>();
    execute(&mut f, 100_000, ops, q).unwrap();
    for (i, t) in f.assets.iter().skip(1).enumerate() {
        assert_eq!(
            query_cw20_balance(&f.app, t, &f.trader),
            before[i]
                + if i == f.assets.len() - 2 {
                    q
                } else {
                    Uint128::zero()
                },
            "no unrelated intermediate wallet debit even when balances exist"
        );
    }
}
