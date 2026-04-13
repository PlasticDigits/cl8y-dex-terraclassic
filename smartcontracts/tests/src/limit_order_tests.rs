//! Integration tests for FIFO limit orders and Pattern C hybrid swaps.

use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use super::helpers::*;

use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, HybridReverseSimulationResponse, HybridSimulationResponse,
    HybridSwapParams, LimitOrderResponse, LimitOrderSide, PausedResponse, QueryMsg,
    ReverseSimulationResponse, SimulationResponse,
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

fn count_limit_order_fill_events(events: &[cosmwasm_std::Event]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.attributes
                .iter()
                .any(|a| a.key == "action" && a.value == "limit_order_fill")
        })
        .count()
}

fn wasm_attr_last(events: &[cosmwasm_std::Event], key: &str) -> Option<String> {
    events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|a| a.key == key)
        .last()
        .map(|a| a.value.clone())
}

fn wasm_attr_in_action_event(
    events: &[cosmwasm_std::Event],
    action: &str,
    key: &str,
) -> Option<String> {
    for e in events {
        if !e
            .attributes
            .iter()
            .any(|a| a.key == "action" && a.value == action)
        {
            continue;
        }
        return e
            .attributes
            .iter()
            .find(|a| a.key == key)
            .map(|a| a.value.clone());
    }
    None
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

/// One wasm event per maker fill (`action` = `limit_order_fill`) for indexers.
#[test]
fn hybrid_swap_emits_limit_order_fill_events() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_fill_ev");
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

    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(500_000),
        Decimal::one(),
    );

    let swap_in = Uint128::new(100_000);
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: swap_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

    assert_eq!(count_limit_order_fill_events(&res.events), 1);
    let fill_ev = res
        .events
        .iter()
        .find(|e| {
            e.attributes
                .iter()
                .any(|a| a.key == "action" && a.value == "limit_order_fill")
        })
        .expect("limit_order_fill event");
    let oid = fill_ev
        .attributes
        .iter()
        .find(|a| a.key == "order_id")
        .map(|a| a.value.parse::<u64>().unwrap())
        .expect("order_id");
    assert_eq!(oid, order_id);
    let side = fill_ev
        .attributes
        .iter()
        .find(|a| a.key == "side")
        .map(|a| a.value.as_str())
        .expect("side");
    assert_eq!(side, "bid");
}

/// GitLab #83 — hybrid book leg uses the taker’s discounted `effective_fee_bps` (same as pool path).
#[test]
fn hybrid_book_fill_uses_taker_discounted_effective_fee_bps() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = Addr::unchecked("taker_disc_book");

    let cw20_code_id = app.store_code(cw20_mintable_contract());
    let fd_code_id = app.store_code(fee_discount_contract());

    let cl8y = create_cw20_token_with_decimals(
        &mut app,
        cw20_code_id,
        &env.user,
        "CL8Y",
        "CL8Y",
        18,
        Uint128::new(1_000_000_000_000_000_000_000u128),
    );

    let fd = app
        .instantiate_contract(
            fd_code_id,
            env.governance.clone(),
            &cl8y_dex_fee_discount::msg::InstantiateMsg {
                governance: env.governance.to_string(),
                cl8y_token: cl8y.to_string(),
            },
            &[],
            "fd_disc_book",
            None,
        )
        .unwrap();

    app.execute_contract(
        env.governance.clone(),
        fd.clone(),
        &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
            tier_id: 1,
            min_cl8y_balance: Uint128::zero(),
            discount_bps: 5000,
            governance_only: false,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::SetDiscountRegistry {
            pair: env.pair.to_string(),
            registry: Some(fd.to_string()),
        },
        &[],
    )
    .unwrap();

    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    transfer_tokens(
        &mut app,
        &cl8y,
        &env.user,
        &taker,
        Uint128::new(1_000_000_000_000_000_000u128),
    );
    app.execute_contract(
        taker.clone(),
        fd,
        &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
        &[],
    )
    .unwrap();

    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(500_000),
        Decimal::one(),
    );

    let base_fee_bps: u32 = 30;
    let expected_effective = (base_fee_bps * (10000 - 5000u32) / 10000) as u16;

    let swap_in = Uint128::new(100_000);
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: swap_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: swap_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

    let eff = wasm_attr_last(&res.events, "effective_fee_bps")
        .expect("effective_fee_bps")
        .parse::<u16>()
        .unwrap();
    assert_eq!(eff, expected_effective);
}

#[test]
fn hybrid_swap_two_makers_emits_two_fill_events() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_two_mk");
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

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(80_000),
        Decimal::one(),
    );
    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(80_000),
        Decimal::one(),
    );

    let book_in = Uint128::new(100_000);
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: book_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: book_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    assert_eq!(count_limit_order_fill_events(&res.events), 2);
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
fn limit_order_place_and_cancel_emit_indexer_attrs() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let place_msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrder {
        side: LimitOrderSide::Bid,
        price: Decimal::one(),
        hint_after_order_id: None,
        max_adjust_steps: 32,
        expires_at: None,
    })
    .unwrap();
    let place_res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(80_000),
                msg: place_msg,
            },
            &[],
        )
        .unwrap();

    assert_eq!(
        wasm_attr_last(&place_res.events, "action").as_deref(),
        Some("place_limit_order")
    );
    assert_eq!(
        wasm_attr_last(&place_res.events, "side").as_deref(),
        Some("bid")
    );
    assert_eq!(
        wasm_attr_last(&place_res.events, "price").as_deref(),
        Some("1")
    );
    assert_eq!(
        wasm_attr_last(&place_res.events, "owner").as_deref(),
        Some(env.user.as_str())
    );

    let order_id = parse_limit_order_placed(&place_res.events);
    let cancel_res = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .unwrap();
    assert_eq!(
        wasm_attr_in_action_event(&cancel_res.events, "cancel_limit_order", "action").as_deref(),
        Some("cancel_limit_order")
    );
    assert_eq!(
        wasm_attr_in_action_event(&cancel_res.events, "cancel_limit_order", "owner").as_deref(),
        Some(env.user.as_str())
    );
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

/// GitLab #87 / invariant L6: pause blocks pool swap, new limit placement, and cancel; `IsPaused` query
/// reflects paused state; after unpause, cancel refunds bid escrow.
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

    let unpaused: PausedResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::IsPaused {})
        .unwrap();
    assert!(!unpaused.paused);

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

    let paused: PausedResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::IsPaused {})
        .unwrap();
    assert!(paused.paused);

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

    assert!(app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .is_err());

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

    let unpaused_again: PausedResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::IsPaused {})
        .unwrap();
    assert!(!unpaused_again.paused);

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

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(50_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(50_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    assert_eq!(count_limit_order_fill_events(&res.events), 1);

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

    let bid_escrow = Uint128::new(200_000);
    let order_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
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
    let hybrid = HybridSwapParams {
        pool_input: Uint128::new(40_000),
        book_input: Uint128::new(60_000),
        max_maker_fills: 8,
        book_start_hint: None,
    };
    let sim: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: total_in,
                },
                hybrid: hybrid.clone(),
            },
        )
        .unwrap();
    assert!(
        sim.book_return_amount > Uint128::zero(),
        "L8: book leg should contribute when bid rests at price 1"
    );

    let taker_b_before = query_cw20_balance(&app, &env.token_b, &taker);
    let res = {
        let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
            to: None,
            deadline: None,
            hybrid: Some(hybrid),
            trader: None,
        })
        .unwrap();
        app.execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap()
    };
    assert!(
        count_limit_order_fill_events(&res.events) >= 1,
        "book leg should emit limit_order_fill"
    );
    let lo = query_limit(&app, &env.pair, order_id);
    assert!(lo.remaining < bid_escrow);
    let taker_b_after = query_cw20_balance(&app, &env.token_b, &taker);
    assert_eq!(
        taker_b_after.checked_sub(taker_b_before).unwrap(),
        sim.return_amount,
        "L8: executed token B out should match HybridSimulation for same snapshot"
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
fn router_simulate_swap_hybrid_matches_pool_when_book_empty() {
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

#[test]
fn router_two_hop_first_leg_hybrid_matches_simulate() {
    let mut app = App::default();
    let abc = setup_router_abc_env(&mut app);
    let env = &abc.env;

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(500_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_2hop_hybrid");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let offer_a = Uint128::new(80_000);
    let hop1_hybrid = HybridSwapParams {
        pool_input: Uint128::new(20_000),
        book_input: Uint128::new(60_000),
        max_maker_fills: 8,
        book_start_hint: None,
    };
    let operations = vec![
        cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_a),
            ask_asset_info: asset_info_token(&env.token_b),
            hybrid: Some(hop1_hybrid),
        },
        cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_b),
            ask_asset_info: asset_info_token(&abc.token_c),
            hybrid: None,
        },
    ];

    let sim: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
        .wrap()
        .query_wasm_smart(
            env.router.to_string(),
            &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                offer_amount: offer_a,
                operations: operations.clone(),
            },
        )
        .unwrap();

    let c_before = query_cw20_balance(&app, &abc.token_c, &taker);
    let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
        operations,
        max_spread: Decimal::one(),
        minimum_receive: None,
        to: None,
        deadline: None,
        unwrap_output: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.router.to_string(),
                amount: offer_a,
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();
    assert!(count_limit_order_fill_events(&res.events) >= 1);
    let c_after = query_cw20_balance(&app, &abc.token_c, &taker);
    let got_c = c_after.checked_sub(c_before).unwrap();
    assert_eq!(
        got_c, sim.amount,
        "L8: router multi-hop output should match SimulateSwapOperations (hybrid on first hop only)"
    );
}

#[test]
fn hybrid_reverse_pool_only_matches_reverse_simulation() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let ask_amt = Uint128::new(50_000);
    let rev: ReverseSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ReverseSimulation {
                ask_asset: Asset {
                    info: asset_info_token(&env.token_b),
                    amount: ask_amt,
                },
            },
        )
        .unwrap();

    let hrev: HybridReverseSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridReverseSimulation {
                ask_asset: Asset {
                    info: asset_info_token(&env.token_b),
                    amount: ask_amt,
                },
                hybrid: HybridSwapParams {
                    pool_input: Uint128::new(1u128),
                    book_input: Uint128::zero(),
                    max_maker_fills: 8,
                    book_start_hint: None,
                },
            },
        )
        .unwrap();

    let diff = if hrev.offer_amount > rev.offer_amount {
        hrev.offer_amount - rev.offer_amount
    } else {
        rev.offer_amount - hrev.offer_amount
    };
    assert!(
        diff <= Uint128::one(),
        "hybrid reverse offer should match pool reverse within 1 unit; rev={} hrev={}",
        rev.offer_amount,
        hrev.offer_amount
    );
}

#[test]
fn hybrid_forward_sim_matches_execute_when_book_empty() {
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
    let hybrid = HybridSwapParams {
        pool_input: Uint128::zero(),
        book_input: offer,
        max_maker_fills: 8,
        book_start_hint: None,
    };
    let h: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: offer,
                },
                hybrid: hybrid.clone(),
            },
        )
        .unwrap();
    let p: SimulationResponse = app
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
    assert_eq!(h.return_amount, p.return_amount);
    assert_eq!(h.book_return_amount, Uint128::zero());
}
