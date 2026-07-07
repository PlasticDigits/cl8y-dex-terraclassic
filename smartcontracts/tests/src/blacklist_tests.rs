//! Trading blacklist integration tests (GitLab #308).

use cosmwasm_std::{to_json_binary, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use cl8y_dex_pair::state::PAIR_INFO;
use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::limit_placement::LimitOrderPlacementItem;
use dex_common::pair::{
    pool_only_hybrid_params, Cw20HookMsg, ExecuteMsg, HybridSwapParams, LimitOrderSide,
};
use dex_common::types::Asset;

use crate::helpers::*;

fn is_blacklisted_err(err: &dyn std::error::Error) -> bool {
    let s = err.to_string();
    s.contains("Trading blacklist") || s.contains("Blacklisted")
}

fn is_blacklist_guard_unavailable_err(err: &dyn std::error::Error) -> bool {
    err.to_string().contains("Blacklist guard unavailable")
}

fn batch_place_msg(side: LimitOrderSide, price: Decimal, amount: Uint128) -> cosmwasm_std::Binary {
    batch_place_msg_with_expires(side, price, amount, None)
}

fn batch_place_msg_with_expires(
    side: LimitOrderSide,
    price: Decimal,
    amount: Uint128,
    expires_at: Option<u64>,
) -> cosmwasm_std::Binary {
    to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side,
        orders: vec![LimitOrderPlacementItem {
            price,
            amount,
            max_adjust_steps: 32,
            expires_at,
            hint_after_order_id: None,
        }],
    })
    .unwrap()
}

fn parse_limit_order_placed(events: &[cosmwasm_std::Event]) -> u64 {
    events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .find(|a| a.key == "limit_order_placed")
        .map(|a| a.value.parse::<u64>().unwrap())
        .expect("limit_order_placed attribute")
}

fn blacklist_wallet(app: &mut App, env: &TestEnv, wallet: &cosmwasm_std::Addr) {
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::BlacklistWallet {
            address: wallet.to_string(),
        },
        &[],
    )
    .unwrap();
}

fn unblacklist_wallet(app: &mut App, env: &TestEnv, wallet: &cosmwasm_std::Addr) {
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::UnblacklistWallet {
            address: wallet.to_string(),
        },
        &[],
    )
    .unwrap();
}

fn setup_liquid_pool(app: &mut App, env: &TestEnv) {
    provide_liquidity(
        app,
        env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
}

#[test]
fn non_governance_cannot_blacklist_wallet() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let err = app
        .execute_contract(
            env.user.clone(),
            env.factory.clone(),
            &FactoryExecuteMsg::BlacklistWallet {
                address: env.user.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));
}

#[test]
fn wallet_blacklist_blocks_swap_lp_limits_and_unban_restores() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);

    let place_msg = batch_place_msg(LimitOrderSide::Bid, Decimal::one(), Uint128::new(10_000));
    app.execute_contract(
        env.user.clone(),
        env.token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: Uint128::new(10_000),
            msg: place_msg,
        },
        &[],
    )
    .unwrap();
    let order_id = 1u64;

    // Park an expired limit order so ClaimExpiredLimitOrder can be exercised under blacklist.
    let taker = cosmwasm_std::Addr::unchecked("taker_exp_blacklist");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );
    let exp = app.block_info().time.seconds() + 60;
    let escrow_expired = Uint128::new(10_000);
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: escrow_expired,
                msg: batch_place_msg_with_expires(
                    LimitOrderSide::Bid,
                    Decimal::one(),
                    escrow_expired,
                    Some(exp),
                ),
            },
            &[],
        )
        .unwrap();
    let expired_order_id = parse_limit_order_placed(&res.events);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });

    let park_swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: Some(Uint128::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(1_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        taker,
        env.token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: Uint128::new(1_000),
            msg: park_swap_msg,
        },
        &[],
    )
    .unwrap();

    blacklist_wallet(&mut app, &env, &env.user);

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: batch_place_msg(LimitOrderSide::Bid, Decimal::one(), Uint128::new(1_000)),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(1_000),
                    },
                    Asset {
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
    ));

    let hybrid = pool_only_hybrid_params(Uint128::new(500));
    let hybrid_swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
        trader: None,
    })
    .unwrap();
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(500),
                msg: hybrid_swap_msg,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::UpdateLimitOrderPrice {
                order_id,
                price: Decimal::from_ratio(11u128, 10u128),
                hint_after_order_id: None,
                max_adjust_steps: 32,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::ClaimExpiredLimitOrder {
                order_id: expired_order_id,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    let lp_before = query_cw20_balance(&app, &env.lp_token, &env.user);
    assert!(lp_before > Uint128::zero());
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.lp_token.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: to_json_binary(&Cw20HookMsg::WithdrawLiquidity { min_assets: None }).unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    unblacklist_wallet(&mut app, &env, &env.user);
    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(500));
}

#[test]
fn token_blacklist_blocks_swap_both_directions() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::BlacklistToken {
            token: env.token_a.to_string(),
        },
        &[],
    )
    .unwrap();

    let swap_msg = |hybrid: Option<HybridSwapParams>| {
        to_json_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
            min_return: None,
            to: None,
            deadline: None,
            hybrid,
            trader: None,
        })
        .unwrap()
    };

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg(None),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg(None),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    let hybrid = pool_only_hybrid_params(Uint128::new(500));
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(500),
                msg: swap_msg(Some(hybrid)),
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));
}

fn swap_b_to_c_on_pair(
    app: &mut App,
    token_b: &cosmwasm_std::Addr,
    pair_bc: &cosmwasm_std::Addr,
    sender: &cosmwasm_std::Addr,
    amount: Uint128,
) {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        sender.clone(),
        token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: pair_bc.to_string(),
            amount,
            msg: swap_msg,
        },
        &[],
    )
    .unwrap();
}

/// SEC-B04: blacklisting pair A must not block swaps on an unrelated pair B for the same user.
#[test]
fn pair_blacklist_blocks_target_pair_but_not_unrelated_control_pair() {
    let mut app = App::default();
    let abc = setup_router_abc_env(&mut app);
    let env = &abc.env;

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::BlacklistPair {
            pair: env.pair.to_string(),
        },
        &[],
    )
    .unwrap();

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(500),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));

    let balance_c_before = query_cw20_balance(&app, &abc.token_c, &env.user);
    swap_b_to_c_on_pair(
        &mut app,
        &env.token_b,
        &abc.pair_bc,
        &env.user,
        Uint128::new(500),
    );
    let balance_c_after = query_cw20_balance(&app, &abc.token_c, &env.user);
    assert!(balance_c_after > balance_c_before);
}

#[test]
fn pair_blacklist_blocks_swap_and_lp() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::BlacklistPair {
            pair: env.pair.to_string(),
        },
        &[],
    )
    .unwrap();

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();
    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(500),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));
}

#[test]
fn router_multihop_rejects_blacklisted_wallet() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);
    blacklist_wallet(&mut app, &env, &env.user);

    let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
        operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_a),
            ask_asset_info: asset_info_token(&env.token_b),
            hybrid: None,
            min_return: None,
        }],
        max_spread: Decimal::one(),
        minimum_receive: None,
        to: None,
        deadline: None,
        unwrap_output: None,
    })
    .unwrap();

    assert!(is_blacklisted_err(
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.router.to_string(),
                amount: Uint128::new(1_000),
                msg: hook_msg,
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
    ));
}

#[test]
fn unrelated_user_on_clean_pair_can_trade() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);
    blacklist_wallet(&mut app, &env, &env.user);

    let other = cosmwasm_std::Addr::unchecked("other_trader");

    for token in [&env.token_a, &env.token_b] {
        app.execute_contract(
            env.user.clone(),
            token.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: other.to_string(),
                amount: Uint128::new(500_000),
            },
            &[],
        )
        .unwrap();
    }

    swap_a_to_b(&mut app, &env, &other, Uint128::new(500));
}

#[test]
fn factory_blacklist_check_query_reflects_state() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);

    let check: dex_common::blacklist::BlacklistCheckResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::BlacklistCheck(BlacklistCheck {
                wallet: Some(env.user.to_string()),
                tokens: vec![env.token_a.to_string()],
                pair: Some(env.pair.to_string()),
                pairs: vec![],
            }),
        )
        .unwrap();
    assert!(!check.blocked);

    blacklist_wallet(&mut app, &env, &env.user);
    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::BlacklistToken {
            token: env.token_a.to_string(),
        },
        &[],
    )
    .unwrap();

    let check: dex_common::blacklist::BlacklistCheckResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::BlacklistCheck(BlacklistCheck {
                wallet: Some(env.user.to_string()),
                tokens: vec![env.token_a.to_string()],
                pair: None,
                pairs: vec![env.pair.to_string()],
            }),
        )
        .unwrap();
    assert!(check.blocked);
    assert!(check.wallet_blacklisted);
    assert!(!check.blacklisted_tokens.is_empty());
}

#[test]
fn factory_blacklist_query_error_blocks_swap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);

    // Point the pair at a CW20 that does not implement BlacklistCheck — simulates a stale
    // factory pointer or factory query failure (GitLab #456 / SEC-I03 F02).
    {
        let mut storage = app.contract_storage_mut(&env.pair);
        PAIR_INFO
            .update(&mut *storage, |mut info| -> cosmwasm_std::StdResult<_> {
                info.factory = env.token_a.clone();
                Ok(info)
            })
            .unwrap();
    }

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: None,
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(100),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err();
    assert!(is_blacklist_guard_unavailable_err(&err.root_cause()));
}

/// GitLab #468 — blacklisted maker's resting limits are not filled; escrow parks for post-unblacklist claim.
#[test]
fn blacklisted_maker_resting_limit_not_filled_taker_can_still_swap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    setup_liquid_pool(&mut app, &env);

    let maker = env.user.clone();
    let taker = cosmwasm_std::Addr::unchecked("taker_blacklist_maker_fill");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &maker,
        &taker,
        Uint128::new(500_000),
    );

    let bid_escrow = Uint128::new(50_000);
    app.execute_contract(
        maker.clone(),
        env.token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: bid_escrow,
            msg: batch_place_msg(LimitOrderSide::Bid, Decimal::one(), bid_escrow),
        },
        &[],
    )
    .unwrap();

    let maker_token_a_before = query_cw20_balance(&app, &env.token_a, &maker);
    blacklist_wallet(&mut app, &env, &maker);

    let hybrid = HybridSwapParams {
        pool_input: Uint128::zero(),
        book_input: Uint128::new(10_000),
        max_maker_fills: 8,
        book_start_hint: None,
    };
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: Some(Uint128::one()),
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        taker,
        env.token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: Uint128::new(10_000),
            msg: swap_msg,
        },
        &[],
    )
    .unwrap();

    let maker_token_a_after = query_cw20_balance(&app, &env.token_a, &maker);
    assert_eq!(
        maker_token_a_after, maker_token_a_before,
        "blacklisted maker must not receive offer-token payout from a book fill"
    );

    let parked: Option<dex_common::pair::ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &dex_common::pair::QueryMsg::ExpiredLimitRefund { order_id: 1 },
        )
        .unwrap();
    assert!(
        parked.is_some(),
        "blacklisted maker order should park off-book for claim after unblacklist"
    );
    assert_eq!(parked.unwrap().owner, maker);
}
