//! Integration tests for FIFO limit orders and Pattern C hybrid swaps.

use cosmwasm_std::{to_json_binary, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use super::helpers::*;

use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, HybridSwapParams, LimitOrderResponse, LimitOrderSide, QueryMsg,
    SimulationResponse,
};
use dex_common::types::Asset;

fn place_bid_with_steps(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    from: &cosmwasm_std::Addr,
    token_b: &cosmwasm_std::Addr,
    amount: Uint128,
    price: Decimal,
    max_adjust_steps: u32,
) -> u64 {
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price,
        hint_after_order_id: None,
        max_adjust_steps,
        expires_at: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            from.clone(),
            token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount,
                msg,
            },
            &[],
        )
        .unwrap();
    parse_limit_order_placed(&res.events)
}

fn parse_limit_order_placed(events: &[cosmwasm_std::Event]) -> u64 {
    events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .find(|a| a.key == "limit_order_placed")
        .map(|a| a.value.parse::<u64>().unwrap())
        .expect("limit_order_placed attribute")
}

fn place_bid(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    from: &cosmwasm_std::Addr,
    token_b: &cosmwasm_std::Addr,
    amount: Uint128,
    price: Decimal,
) -> u64 {
    place_bid_with_steps(app, pair, from, token_b, amount, price, 32)
}

fn place_ask(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    from: &cosmwasm_std::Addr,
    token_a: &cosmwasm_std::Addr,
    amount: Uint128,
    price: Decimal,
) -> u64 {
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Ask,
        price,
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            from.clone(),
            token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: pair.to_string(),
                amount,
                msg,
            },
            &[],
        )
        .unwrap();
    parse_limit_order_placed(&res.events)
}

fn swap_a_to_b_hybrid(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    sender: &cosmwasm_std::Addr,
    token_a: &cosmwasm_std::Addr,
    amount: Uint128,
    hybrid: Option<HybridSwapParams>,
) {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid,
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        sender.clone(),
        token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: pair.to_string(),
            amount,
            msg: swap_msg,
        },
        &[],
    )
    .unwrap();
}

fn swap_b_to_a_hybrid(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    sender: &cosmwasm_std::Addr,
    token_b: &cosmwasm_std::Addr,
    amount: Uint128,
    hybrid: Option<HybridSwapParams>,
) {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid,
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        sender.clone(),
        token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: pair.to_string(),
            amount,
            msg: swap_msg,
        },
        &[],
    )
    .unwrap();
}

fn query_limit(app: &App, pair: &cosmwasm_std::Addr, order_id: u64) -> LimitOrderResponse {
    app.wrap()
        .query_wasm_smart(pair.to_string(), &QueryMsg::LimitOrder { order_id })
        .unwrap()
}

#[test]
fn bid_and_hybrid_swap_partially_fills_book() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker");
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    // Fund taker with token A for the swap
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let bid_escrow = Uint128::new(500_000);
    let price = Decimal::one();
    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
        price,
    );

    let swap_in = Uint128::new(100_000);
    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        swap_in,
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let lo: LimitOrderResponse = query_limit(&app, &env.pair, order_id);
    assert_eq!(lo.side, LimitOrderSide::Bid);
    assert!(lo.remaining < bid_escrow);
    assert!(!lo.remaining.is_zero());
}

#[test]
fn ask_and_hybrid_swap_partially_fills_book() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker2");
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let ask_escrow = Uint128::new(400_000);
    let price = Decimal::one();
    let order_id = place_ask(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_a,
        ask_escrow,
        price,
    );

    let swap_in = Uint128::new(80_000);
    swap_b_to_a_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_b,
        swap_in,
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let lo: LimitOrderResponse = query_limit(&app, &env.pair, order_id);
    assert_eq!(lo.side, LimitOrderSide::Ask);
    assert!(lo.remaining < ask_escrow);
}

/// Non-unity bid price: fee is taken in token1; treasury receives token1 commission.
#[test]
fn hybrid_bid_non_unity_price_treasury_and_escrow_coherent() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_nu_bid");
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let price = Decimal::from_ratio(2u128, 1u128);
    let bid_escrow = Uint128::new(2_000_000);
    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
        price,
    );

    let tre_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
    let swap_in = Uint128::new(100_000);
    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        swap_in,
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let cost_token1 = Uint128::new(200_000);
    let commission = cost_token1.multiply_ratio(30u128, 10_000u128);
    let net_to_taker = cost_token1.checked_sub(commission).unwrap();

    let lo = query_limit(&app, &env.pair, order_id);
    assert_eq!(lo.remaining, bid_escrow.checked_sub(cost_token1).unwrap());

    let taker_b = query_cw20_balance(&app, &env.token_b, &taker);
    assert_eq!(taker_b, net_to_taker);

    let tre_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
    assert_eq!(tre_b_after.checked_sub(tre_b_before).unwrap(), commission);
}

/// Non-unity ask price: fee on token0 output; treasury receives token0 commission (ask-side fix).
#[test]
fn hybrid_ask_non_unity_price_treasury_fee_in_token0() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_nu_ask");
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let price = Decimal::from_ratio(1u128, 10u128);
    let ask_escrow = Uint128::new(1_000_000);
    let order_id = place_ask(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_a,
        ask_escrow,
        price,
    );

    let tre_a_before = query_cw20_balance(&app, &env.token_a, &env.treasury);
    let swap_in = Uint128::new(50_000);
    swap_b_to_a_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_b,
        swap_in,
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let fill_t0 = Uint128::new(500_000);
    let commission = fill_t0.multiply_ratio(30u128, 10_000u128);
    let net_t0 = fill_t0.checked_sub(commission).unwrap();

    let lo = query_limit(&app, &env.pair, order_id);
    assert_eq!(lo.remaining, ask_escrow.checked_sub(fill_t0).unwrap());

    let taker_a = query_cw20_balance(&app, &env.token_a, &taker);
    assert_eq!(taker_a, net_t0);

    let tre_a_after = query_cw20_balance(&app, &env.token_a, &env.treasury);
    assert_eq!(tre_a_after.checked_sub(tre_a_before).unwrap(), commission);
}

#[test]
fn place_limit_order_expiry_not_future_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let now = app.block_info().time.seconds();
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::one(),
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: Some(now),
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(10_000),
                msg,
            },
            &[],
        )
        .unwrap_err();
    // `InvalidHybridParams` display is generic; `reason` is only in Debug.
    let s = format!("{:?}", err.root_cause());
    assert!(s.contains("expires_at") || s.contains("future"), "{}", s);
}

#[test]
fn expired_bid_skipped_on_hybrid_swap_without_maker_credit() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp");
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let exp = app.block_info().time.seconds() + 120;
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::one(),
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: Some(exp),
    })
    .unwrap();
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(10_000),
                msg,
            },
            &[],
        )
        .unwrap();
    let order_id = parse_limit_order_placed(&res.events);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        Uint128::new(5_000),
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(5_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let res: Result<LimitOrderResponse, _> = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::LimitOrder { order_id });
    assert!(
        res.is_err(),
        "expired order should be unlinked, not queryable"
    );
}

#[test]
fn cancel_limit_order_refunds_escrow() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let escrow = Uint128::new(250_000);
    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        escrow,
        Decimal::one(),
    );

    let before = query_cw20_balance(&app, &env.token_b, &env.user);
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::CancelLimitOrder { order_id },
        &[],
    )
    .unwrap();
    let after = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(after.checked_sub(before).unwrap(), escrow);
}

#[test]
fn place_limit_order_wrong_escrow_token_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    // Bid must escrow token_b; sending token_a must fail.
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::one(),
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: None,
    })
    .unwrap();
    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(10_000),
                msg,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .to_lowercase()
            .contains("invalid")
            || err.root_cause().to_string().contains("token"),
        "{}",
        err
    );
}

// --- L3 / sad-path & L5 / L6 / L8 coverage (see docs/contracts-security-audit.md) ---

#[test]
fn cancel_limit_order_non_owner_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(100_000),
        Decimal::one(),
    );

    let attacker = cosmwasm_std::Addr::unchecked("attacker");
    let err = app
        .execute_contract(
            attacker,
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Unauthorized"),
        "{}",
        err
    );
}

#[test]
fn hybrid_split_mismatch_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::new(30_000),
            book_input: Uint128::new(50_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(100_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err();
    let msg = err.root_cause().to_string();
    assert!(
        msg.contains("Hybrid swap split") || msg.contains("mismatch"),
        "{}",
        msg
    );
}

#[test]
fn hybrid_max_maker_zero_with_book_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(10_000),
            max_maker_fills: 0,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(10_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(
        s.contains("max_maker") || s.contains("Invalid hybrid"),
        "{}",
        s
    );
}

#[test]
fn pause_blocks_swap_and_place_cancel_refunds_escrow() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(50_000),
        Decimal::one(),
    );

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::SetPairPaused {
            pair: env.pair.to_string(),
            paused: true,
        },
        &[],
    )
    .unwrap();

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: None,
        trader: None,
    })
    .unwrap();
    assert!(app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .is_err());

    let place_msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::one(),
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: None,
    })
    .unwrap();
    assert!(app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: place_msg,
            },
            &[],
        )
        .is_err());

    // Cancel is allowed while paused so makers can recover escrow.
    let bal_before = query_cw20_balance(&app, &env.token_b, &env.user);
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::CancelLimitOrder { order_id },
        &[],
    )
    .unwrap();
    let bal_after = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(
        bal_after.checked_sub(bal_before).unwrap(),
        Uint128::new(50_000)
    );

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::SetPairPaused {
            pair: env.pair.to_string(),
            paused: false,
        },
        &[],
    )
    .unwrap();
}

#[test]
fn fifo_two_bids_same_price_older_filled_first() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let alice = cosmwasm_std::Addr::unchecked("alice_fifo");
    let bob = cosmwasm_std::Addr::unchecked("bob_fifo");
    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &alice,
        Uint128::new(2_000_000),
    );
    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &bob,
        Uint128::new(2_000_000),
    );

    let id_alice = place_bid(
        &mut app,
        &env.pair,
        &alice,
        &env.token_b,
        Uint128::new(100_000),
        Decimal::one(),
    );
    let id_bob = place_bid(
        &mut app,
        &env.pair,
        &bob,
        &env.token_b,
        Uint128::new(100_000),
        Decimal::one(),
    );
    assert!(id_alice < id_bob);

    let taker = cosmwasm_std::Addr::unchecked("taker_fifo");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(200_000),
    );

    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        Uint128::new(50_000),
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(50_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    let lo_a = query_limit(&app, &env.pair, id_alice);
    let lo_b = query_limit(&app, &env.pair, id_bob);
    assert_eq!(lo_a.remaining, Uint128::new(50_000));
    assert_eq!(lo_b.remaining, Uint128::new(100_000));
}

#[test]
fn hybrid_pool_and_book_legs_one_swap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let _bid = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(200_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_hybrid");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let total_in = Uint128::new(100_000);
    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        total_in,
        Some(HybridSwapParams {
            pool_input: Uint128::new(40_000),
            book_input: Uint128::new(60_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );
}

#[test]
fn match_invalid_book_start_hint_falls_back_to_head() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(80_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_hint");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(100_000),
    );

    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        Uint128::new(20_000),
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(20_000),
            max_maker_fills: 8,
            book_start_hint: Some(999_999),
        }),
    );

    let lo = query_limit(&app, &env.pair, order_id);
    assert!(lo.remaining < Uint128::new(80_000));
}

#[test]
fn place_limit_insert_steps_exceeded() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    for _ in 0..10 {
        place_bid(
            &mut app,
            &env.pair,
            &env.user,
            &env.token_b,
            Uint128::new(1_000),
            Decimal::one(),
        );
    }

    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::from_ratio(5u128, 10u128),
        hint_after_order_id: None,
        max_adjust_steps: 5,
        expires_at: None,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg,
            },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(
        s.contains("max adjust steps") || s.contains("Limit order insert"),
        "{}",
        s
    );
}

#[test]
fn router_simulate_swap_hybrid_field_ignored() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let offer = Uint128::new(100_000);
    let ops_base = cl8y_dex_router::msg::SwapOperation::TerraSwap {
        offer_asset_info: asset_info_token(&env.token_a),
        ask_asset_info: asset_info_token(&env.token_b),
        hybrid: None,
    };
    let sim_none: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
        .wrap()
        .query_wasm_smart(
            env.router.to_string(),
            &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                offer_amount: offer,
                operations: vec![ops_base.clone()],
            },
        )
        .unwrap();

    let ops_hybrid = cl8y_dex_router::msg::SwapOperation::TerraSwap {
        offer_asset_info: asset_info_token(&env.token_a),
        ask_asset_info: asset_info_token(&env.token_b),
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: offer,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    };
    let sim_hybrid: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
        .wrap()
        .query_wasm_smart(
            env.router.to_string(),
            &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                offer_amount: offer,
                operations: vec![ops_hybrid],
            },
        )
        .unwrap();

    assert_eq!(sim_none.amount, sim_hybrid.amount);

    let direct: SimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::Simulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: offer,
                },
            },
        )
        .unwrap();
    assert_eq!(direct.return_amount, sim_none.amount);
}

#[test]
fn router_single_hop_forwards_hybrid_to_pair() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
        operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_a),
            ask_asset_info: asset_info_token(&env.token_b),
            hybrid: Some(HybridSwapParams {
                pool_input: Uint128::new(5_000),
                book_input: Uint128::new(5_000),
                max_maker_fills: 8,
                book_start_hint: None,
            }),
        }],
        max_spread: Decimal::one(),
        minimum_receive: None,
        to: None,
        deadline: None,
        unwrap_output: None,
    })
    .unwrap();

    app.execute_contract(
        env.user.clone(),
        env.token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.router.to_string(),
            amount: Uint128::new(10_000),
            msg: hook_msg,
        },
        &[],
    )
    .unwrap();
}
