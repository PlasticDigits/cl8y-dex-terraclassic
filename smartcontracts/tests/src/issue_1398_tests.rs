//! Synthetic net-output regressions for #1398. All balances and contracts are local.
use crate::helpers::*;
use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{App, Executor};
use dex_common::pair::{Cw20HookMsg, HybridSimulationResponse, HybridSwapParams, QueryMsg};
use dex_common::types::Asset;

fn install_tax_hooks(app: &mut App, env: &TestEnv, ask: &Addr, rates: &[u16]) {
    let code = app.store_code(tax_hook_contract_with_reply());
    let mut hooks = vec![];
    for (i, rate) in rates.iter().enumerate() {
        let hook = app
            .instantiate_contract(
                code,
                env.governance.clone(),
                &cl8y_dex_tax_hook::msg::InstantiateMsg {
                    recipient: format!("sink{i}"),
                    tax_percentage_bps: *rate,
                    tax_token: ask.to_string(),
                    admin: env.governance.to_string(),
                },
                &[],
                format!("hook{i}"),
                None,
            )
            .unwrap();
        app.execute_contract(
            env.governance.clone(),
            hook.clone(),
            &cl8y_dex_tax_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![env.pair.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();
        hooks.push(hook.to_string());
    }
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &dex_common::factory::ExecuteMsg::SetPairHooks {
            pair: env.pair.to_string(),
            hooks,
        },
        &[],
    )
    .unwrap();
}

fn quote(
    app: &App,
    env: &TestEnv,
    offer: &Addr,
    hybrid: Option<HybridSwapParams>,
) -> HybridSimulationResponse {
    app.wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(offer),
                    amount: Uint128::new(10_000),
                },
                hybrid,
                greedy: None,
                trader: None,
                sender: None,
                belief_price: None,
            },
        )
        .unwrap()
}

fn send(
    app: &mut App,
    env: &TestEnv,
    offer: &Addr,
    minimum: Option<Uint128>,
    hybrid: Option<HybridSwapParams>,
) -> Result<cw_multi_test::AppResponse, String> {
    app.execute_contract(
        env.user.clone(),
        offer.clone(),
        &Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: Uint128::new(10_000),
            msg: to_json_binary(&Cw20HookMsg::Swap {
                belief_price: Some(Decimal::one()),
                max_spread: Some(Decimal::percent(50)),
                min_return: minimum,
                to: None,
                deadline: None,
                trader: None,
                hybrid,
                greedy: None,
            })
            .unwrap(),
        },
        &[],
    )
    .map_err(|e| e.root_cause().to_string())
}

#[test]
fn issue_1398_hook_swaps_require_signed_net_floor_and_rollback() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    install_tax_hooks(&mut app, &env, &env.token_b, &[1000]);
    let sim = quote(&app, &env, &env.token_a, None);
    let pool = query_pool(&app, &env.pair);
    let a = query_cw20_balance(&app, &env.token_a, &env.user);
    let b = query_cw20_balance(&app, &env.token_b, &env.user);
    for minimum in [
        None,
        Some(Uint128::zero()),
        Some(sim.return_amount + Uint128::one()),
    ] {
        let err = send(&mut app, &env, &env.token_a, minimum, None).unwrap_err();
        assert!(
            err.contains("positive net min_return") || err.contains("Min return"),
            "{err}"
        );
        assert_eq!(query_pool(&app, &env.pair), pool);
        assert_eq!(query_cw20_balance(&app, &env.token_a, &env.user), a);
        assert_eq!(query_cw20_balance(&app, &env.token_b, &env.user), b);
        assert_eq!(
            query_cw20_balance(&app, &env.token_b, &Addr::unchecked("sink0")),
            Uint128::zero()
        );
    }
    send(&mut app, &env, &env.token_a, Some(sim.return_amount), None).unwrap();
    assert_eq!(
        query_cw20_balance(&app, &env.token_b, &env.user) - b,
        sim.return_amount
    );
    assert_eq!(
        sim.pool_return_amount - sim.return_amount,
        query_cw20_balance(&app, &env.token_b, &Addr::unchecked("sink0"))
    );
}

#[test]
fn issue_1398_pool_book_mixed_both_directions_quote_settlement_parity() {
    for reverse in [false, true] {
        for pool_input in [0u128, 5000, 10_000] {
            for rates in [vec![], vec![0], vec![1000], vec![500, 700], vec![10_000]] {
                let mut app = App::default();
                let env = setup_full_env(&mut app);
                provide_liquidity(
                    &mut app,
                    &env,
                    &env.user,
                    Uint128::new(1_000_000),
                    Uint128::new(1_000_000),
                );
                let (offer, ask) = if reverse {
                    (&env.token_b, &env.token_a)
                } else {
                    (&env.token_a, &env.token_b)
                };
                let book_input = 10_000 - pool_input;
                if book_input > 0 {
                    let item = dex_common::limit_placement::LimitOrderPlacementItem {
                        price: Decimal::one(),
                        amount: Uint128::new(100_000),
                        max_adjust_steps: 32,
                        expires_at: None,
                        hint_after_order_id: None,
                    };
                    app.execute_contract(
                        env.user.clone(),
                        ask.clone(),
                        &Cw20ExecuteMsg::Send {
                            contract: env.pair.to_string(),
                            amount: item.amount,
                            msg: to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
                                side: if reverse {
                                    dex_common::pair::LimitOrderSide::Ask
                                } else {
                                    dex_common::pair::LimitOrderSide::Bid
                                },
                                orders: vec![item],
                            })
                            .unwrap(),
                        },
                        &[],
                    )
                    .unwrap();
                }
                install_tax_hooks(&mut app, &env, ask, &rates);
                let hybrid = Some(HybridSwapParams {
                    pool_input: Uint128::new(pool_input),
                    book_input: Uint128::new(book_input),
                    max_maker_fills: 8,
                    book_start_hint: None,
                });
                let sim = quote(&app, &env, offer, hybrid.clone());
                let protocol_net = sim.book_return_amount + sim.pool_return_amount;
                let hook_total = rates.iter().fold(Uint128::zero(), |total, bps| {
                    total + protocol_net.multiply_ratio(*bps as u128, 10_000u128)
                });
                assert_eq!(sim.return_amount, protocol_net - hook_total);
                let before = query_cw20_balance(&app, ask, &env.user);
                if sim.return_amount.is_zero() {
                    let pool = query_pool(&app, &env.pair);
                    assert!(send(&mut app, &env, offer, Some(Uint128::one()), hybrid)
                        .unwrap_err()
                        .contains("Min return"));
                    assert_eq!(query_pool(&app, &env.pair), pool);
                    assert_eq!(
                        query_cw20_balance(&app, ask, &Addr::unchecked("sink0")),
                        Uint128::zero()
                    );
                } else {
                    // Separate maker from taker, so maker payouts cannot mask receiver net output.
                    let taker = Addr::unchecked("matrix_taker");
                    transfer_tokens(&mut app, offer, &env.user, &taker, Uint128::new(10_000));
                    let taker_env = TestEnv {
                        user: taker.clone(),
                        factory: env.factory.clone(),
                        governance: env.governance.clone(),
                        treasury: env.treasury.clone(),
                        token_a: env.token_a.clone(),
                        token_b: env.token_b.clone(),
                        pair: env.pair.clone(),
                        lp_token: env.lp_token.clone(),
                        router: env.router.clone(),
                    };
                    send(&mut app, &taker_env, offer, Some(sim.return_amount), hybrid).unwrap();
                    assert_eq!(query_cw20_balance(&app, ask, &taker), sim.return_amount);
                    assert!(query_cw20_balance(&app, ask, &env.user) >= before);
                }
            }
        }
    }
}

#[test]
fn issue_1398_router_net_quotes_and_per_hop_minimum_cannot_be_waived() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    install_tax_hooks(&mut app, &env, &env.token_b, &[1000, 500]);
    let sim = quote(&app, &env, &env.token_a, None);
    let operation = |min_return| cl8y_dex_router::msg::SwapOperation::TerraSwap {
        offer_asset_info: asset_info_token(&env.token_a),
        ask_asset_info: asset_info_token(&env.token_b),
        hybrid: None,
        greedy: None,
        min_return,
    };
    let route: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
        .wrap()
        .query_wasm_smart(
            env.router.to_string(),
            &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                offer_amount: Uint128::new(10_000),
                operations: vec![operation(None)],
                trader: None,
                sender: None,
            },
        )
        .unwrap();
    assert_eq!(route.amount, sim.return_amount);
    let pool = query_pool(&app, &env.pair);
    let a = query_cw20_balance(&app, &env.token_a, &env.user);
    for min in [
        None,
        Some(sim.return_amount + Uint128::one()),
        Some(sim.return_amount),
    ] {
        let result = app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &Cw20ExecuteMsg::Send {
                contract: env.router.to_string(),
                amount: Uint128::new(10_000),
                msg: to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
                    operations: vec![operation(min)],
                    max_spread: Decimal::percent(50),
                    minimum_receive: Some(sim.return_amount),
                    to: None,
                    deadline: None,
                    unwrap_output: None,
                })
                .unwrap(),
            },
            &[],
        );
        if min == Some(sim.return_amount) {
            result.unwrap();
        } else {
            assert!(result.is_err());
            assert_eq!(query_pool(&app, &env.pair), pool);
            assert_eq!(query_cw20_balance(&app, &env.token_a, &env.user), a);
            assert_eq!(
                query_cw20_balance(&app, &env.token_b, &env.router),
                Uint128::zero()
            );
        }
    }
}

#[test]
fn issue_1398_reverse_rejects_deducting_hooks_explicitly() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    install_tax_hooks(&mut app, &env, &env.token_b, &[1000]);
    let result: Result<dex_common::pair::HybridReverseSimulationResponse, _> =
        app.wrap().query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridReverseSimulation {
                ask_asset: Asset {
                    info: asset_info_token(&env.token_b),
                    amount: Uint128::new(1000),
                },
                hybrid: dex_common::pair::pool_only_hybrid_template(),
                trader: None,
                sender: None,
                belief_price: None,
            },
        );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("reverse simulation unsupported"));
}
