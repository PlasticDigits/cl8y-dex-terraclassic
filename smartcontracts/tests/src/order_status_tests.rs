//! Integration tests for pair `QueryMsg::OrderStatus` (GitLab #505).
//!
//! Read-only custody classification from existing `ORDERS` /
//! `EXPIRED_LIMIT_CLAIMS` maps. `Unknown` ≠ proof of fill.

use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::limit_placement::LimitOrderPlacementItem;
use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, ExpiredLimitParkReason, ExpiredLimitRefundResponse, HybridSwapParams,
    LimitOrderResponse, LimitOrderSide, OrderStatus, OrderStatusResponse, PausedResponse, QueryMsg,
};

use crate::helpers::*;

fn batch_place_msg(
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

fn place_bid(
    app: &mut App,
    pair: &Addr,
    from: &Addr,
    token_b: &Addr,
    amount: Uint128,
    price: Decimal,
) -> u64 {
    let msg = batch_place_msg(LimitOrderSide::Bid, price, amount, None);
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

fn place_ask(
    app: &mut App,
    pair: &Addr,
    from: &Addr,
    token_a: &Addr,
    amount: Uint128,
    price: Decimal,
) -> u64 {
    let msg = batch_place_msg(LimitOrderSide::Ask, price, amount, None);
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

fn place_bid_expiring(app: &mut App, env: &TestEnv, amount: Uint128, expires_at: u64) -> u64 {
    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        amount,
        Some(expires_at),
    );
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount,
                msg,
            },
            &[],
        )
        .unwrap();
    parse_limit_order_placed(&res.events)
}

fn query_order_status(app: &App, pair: &Addr, order_id: u64) -> OrderStatusResponse {
    app.wrap()
        .query_wasm_smart(pair.to_string(), &QueryMsg::OrderStatus { order_id })
        .unwrap()
}

fn query_limit(app: &App, pair: &Addr, order_id: u64) -> LimitOrderResponse {
    app.wrap()
        .query_wasm_smart(pair.to_string(), &QueryMsg::LimitOrder { order_id })
        .unwrap()
}

fn assert_unknown(st: &OrderStatusResponse, order_id: u64) {
    assert_eq!(st.status, OrderStatus::Unknown);
    assert_eq!(st.order_id, order_id);
    assert!(st.owner.is_none(), "A3: Unknown must not invent owner");
    assert!(st.side.is_none());
    assert!(st.price.is_none());
    assert!(st.remaining.is_none());
    assert!(st.expires_at.is_none());
}

fn hybrid_min_return(hybrid: &HybridSwapParams) -> Option<Uint128> {
    if hybrid.book_input.is_zero() {
        None
    } else {
        Some(Uint128::one())
    }
}

fn swap_a_to_b_hybrid(
    app: &mut App,
    pair: &Addr,
    sender: &Addr,
    token_a: &Addr,
    amount: Uint128,
    hybrid: HybridSwapParams,
) {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        min_return: hybrid_min_return(&hybrid),
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
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

/// T1 — resting bid → Active with metadata matching LimitOrder.
#[test]
fn order_status_active_bid() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(100_000),
        Decimal::one(),
    );
    let lo = query_limit(&app, &env.pair, id);
    let st = query_order_status(&app, &env.pair, id);
    assert_eq!(st.status, OrderStatus::Active);
    assert_eq!(st.order_id, id);
    assert_eq!(st.owner.as_ref(), Some(&lo.owner));
    assert_eq!(st.side, Some(lo.side));
    assert_eq!(st.price, Some(lo.price));
    assert_eq!(st.remaining, Some(lo.remaining));
    assert_eq!(st.expires_at, lo.expires_at);
}

/// T2 — resting ask → Active.
#[test]
fn order_status_active_ask() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let price = Decimal::from_ratio(11u128, 10u128);
    let id = place_ask(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_a,
        Uint128::new(50_000),
        price,
    );
    let lo = query_limit(&app, &env.pair, id);
    let st = query_order_status(&app, &env.pair, id);
    assert_eq!(st.status, OrderStatus::Active);
    assert_eq!(st.side, Some(LimitOrderSide::Ask));
    assert_eq!(st.price, Some(lo.price));
    assert_eq!(st.remaining, Some(lo.remaining));
}

/// T3 — partial fill keeps Active with decreased remaining.
#[test]
fn order_status_active_after_partial_fill() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let bid_escrow = Uint128::new(500_000);
    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
        Decimal::one(),
    );
    let before = query_order_status(&app, &env.pair, id);
    let rem_before = before.remaining.unwrap();

    let taker = Addr::unchecked("taker");
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
        Uint128::new(100_000),
        HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(100_000),
            max_maker_fills: 8,
            book_start_hint: None,
        },
    );

    let after = query_order_status(&app, &env.pair, id);
    assert_eq!(after.status, OrderStatus::Active);
    let rem_after = after.remaining.unwrap();
    assert!(rem_after < rem_before, "{rem_after} < {rem_before}");
}

/// T4 — full fill → Unknown; LimitOrder still errors.
#[test]
fn order_status_unknown_after_full_fill() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let bid_escrow = Uint128::new(100_000);
    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
        Decimal::one(),
    );
    let rem = query_limit(&app, &env.pair, id).remaining;

    let taker = Addr::unchecked("taker_full");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );
    // Offer enough token0 to exhaust the bid remaining (price = 1).
    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        rem,
        HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: rem,
            max_maker_fills: 8,
            book_start_hint: None,
        },
    );

    assert_unknown(&query_order_status(&app, &env.pair, id), id);
    assert!(
        app.wrap()
            .query_wasm_smart::<LimitOrderResponse>(
                env.pair.to_string(),
                &QueryMsg::LimitOrder { order_id: id },
            )
            .is_err(),
        "LimitOrder must still error on missing id"
    );
}

/// T5 — cancel → Unknown.
#[test]
fn order_status_unknown_after_cancel() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(25_000),
        Decimal::one(),
    );
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::CancelLimitOrder { order_id: id },
        &[],
    )
    .unwrap();
    assert_unknown(&query_order_status(&app, &env.pair, id), id);
}

/// T6 — batch cancel → each id Unknown.
#[test]
fn order_status_unknown_after_batch_cancel() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let id1 = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(10_000),
        Decimal::one(),
    );
    let id2 = place_ask(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_a,
        Uint128::new(10_000),
        Decimal::one(),
    );
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::CancelLimitOrders {
            order_ids: vec![id1, id2],
        },
        &[],
    )
    .unwrap();
    assert_unknown(&query_order_status(&app, &env.pair, id1), id1);
    assert_unknown(&query_order_status(&app, &env.pair, id2), id2);
}

/// T7 — park via CleanLimitBook → ParkedRefund; ExpiredLimitRefund is Some.
#[test]
fn order_status_parked_after_clean_limit_book() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let exp = app.block_info().time.seconds() + 60;
    let id = place_bid_expiring(&mut app, &env, Uint128::new(20_000), exp);
    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });

    app.execute_contract(
        Addr::unchecked("keeper"),
        env.pair.clone(),
        &ExecuteMsg::CleanLimitBook {
            side: LimitOrderSide::Bid,
            max_orders: 10,
            start_hint: None,
            max_steps: None,
        },
        &[],
    )
    .unwrap();

    let refund: Option<ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ExpiredLimitRefund { order_id: id },
        )
        .unwrap();
    let refund = refund.expect("parked refund row");
    assert_eq!(
        refund.reason,
        Some(ExpiredLimitParkReason::Expired),
        "CleanLimitBook TTL park must set reason=Expired (#504)"
    );

    let st = query_order_status(&app, &env.pair, id);
    assert_eq!(st.status, OrderStatus::ParkedRefund);
    assert_eq!(st.owner.as_ref(), Some(&env.user));
    assert_eq!(st.side, Some(LimitOrderSide::Bid));
    assert_eq!(st.price, None);
    assert_eq!(st.remaining, Some(refund.remaining));
}

/// T8 — claim parked → Unknown; ExpiredLimitRefund is None.
#[test]
fn order_status_unknown_after_claim_parked() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let exp = app.block_info().time.seconds() + 60;
    let id = place_bid_expiring(&mut app, &env, Uint128::new(15_000), exp);
    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });
    app.execute_contract(
        Addr::unchecked("keeper"),
        env.pair.clone(),
        &ExecuteMsg::CleanLimitBook {
            side: LimitOrderSide::Bid,
            max_orders: 10,
            start_hint: None,
            max_steps: None,
        },
        &[],
    )
    .unwrap();

    assert_eq!(
        query_order_status(&app, &env.pair, id).status,
        OrderStatus::ParkedRefund
    );

    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::ClaimExpiredLimitOrder { order_id: id },
        &[],
    )
    .unwrap();

    assert_unknown(&query_order_status(&app, &env.pair, id), id);
    let refund: Option<ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ExpiredLimitRefund { order_id: id },
        )
        .unwrap();
    assert!(refund.is_none());
}

/// T9 — batch claim → each id Unknown.
#[test]
fn order_status_unknown_after_batch_claim() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let exp = app.block_info().time.seconds() + 60;
    let id1 = place_bid_expiring(&mut app, &env, Uint128::new(12_000), exp);
    let id2 = place_bid_expiring(&mut app, &env, Uint128::new(13_000), exp);
    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });
    app.execute_contract(
        Addr::unchecked("keeper"),
        env.pair.clone(),
        &ExecuteMsg::CleanLimitBook {
            side: LimitOrderSide::Bid,
            max_orders: 10,
            start_hint: None,
            max_steps: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::ClaimExpiredLimitOrders {
            order_ids: vec![id1, id2],
        },
        &[],
    )
    .unwrap();

    assert_unknown(&query_order_status(&app, &env.pair, id1), id1);
    assert_unknown(&query_order_status(&app, &env.pair, id2), id2);
}

/// T10 — unused order_id → Unknown (success).
#[test]
fn order_status_unknown_for_unused_id() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    assert_unknown(&query_order_status(&app, &env.pair, 9_999_999), 9_999_999);
}

/// T11 — order_id == 0 → Err (not Unknown).
#[test]
fn order_status_rejects_zero_id() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let err = app
        .wrap()
        .query_wasm_smart::<OrderStatusResponse>(
            env.pair.to_string(),
            &QueryMsg::OrderStatus { order_id: 0 },
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("non-zero") || err.to_string().contains("order_id"),
        "{err}"
    );
}

/// T12 — price update while active reflects new price.
#[test]
fn order_status_active_reflects_price_update() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(40_000),
        Decimal::one(),
    );
    let new_price = Decimal::from_ratio(12u128, 10u128);
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::UpdateLimitOrderPrice {
            order_id: id,
            price: new_price,
            hint_after_order_id: None,
            max_adjust_steps: 32,
        },
        &[],
    )
    .unwrap();

    let st = query_order_status(&app, &env.pair, id);
    assert_eq!(st.status, OrderStatus::Active);
    assert_eq!(st.price, Some(new_price));
}

/// Existing LimitOrder still errors; ExpiredLimitRefund still None when absent.
#[test]
fn legacy_queries_unchanged_for_absent_id() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let missing = 123_456u64;
    assert!(app
        .wrap()
        .query_wasm_smart::<LimitOrderResponse>(
            env.pair.to_string(),
            &QueryMsg::LimitOrder { order_id: missing },
        )
        .is_err());
    let refund: Option<ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ExpiredLimitRefund { order_id: missing },
        )
        .unwrap();
    assert!(refund.is_none());
    assert_unknown(&query_order_status(&app, &env.pair, missing), missing);
}

/// A9 — status query remains available while paused.
#[test]
fn order_status_available_while_paused() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(8_000),
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

    let paused: PausedResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::IsPaused {})
        .unwrap();
    assert!(paused.paused);

    let st = query_order_status(&app, &env.pair, id);
    assert_eq!(st.status, OrderStatus::Active);
    assert_eq!(st.owner.as_ref(), Some(&env.user));
}
