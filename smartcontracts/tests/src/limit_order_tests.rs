//! Integration tests for FIFO limit orders and Pattern C hybrid swaps.

use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
use cw_multi_test::{App, AppResponse, Executor};

use super::helpers::*;

use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::limit_placement::{
    LimitLadderDistribution, LimitOrderLadderSpec, LimitOrderPlacementItem,
};
use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, ExpiredLimitRefundResponse, HybridReverseSimulationResponse,
    HybridSimulationResponse, HybridSwapParams, LimitOrderConfigResponse, LimitOrderResponse,
    LimitOrderSide, PausedResponse, QueryMsg, MAX_EXPIRED_PARKS_PER_SWAP, MAX_MAKER_FILLS_HARD_CAP,
};
use dex_common::types::Asset;

fn batch_place_msg(
    side: LimitOrderSide,
    price: Decimal,
    amount: Uint128,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
) -> cosmwasm_std::Binary {
    to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side,
        orders: vec![LimitOrderPlacementItem {
            price,
            amount,
            max_adjust_steps,
            expires_at,
        }],
    })
    .unwrap()
}

fn place_bid_with_steps(
    app: &mut App,
    pair: &cosmwasm_std::Addr,
    from: &cosmwasm_std::Addr,
    token_b: &cosmwasm_std::Addr,
    amount: Uint128,
    price: Decimal,
    max_adjust_steps: u32,
) -> u64 {
    let msg = batch_place_msg(LimitOrderSide::Bid, price, amount, max_adjust_steps, None);
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

fn count_limit_order_expired_parked_events(events: &[cosmwasm_std::Event]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.attributes
                .iter()
                .any(|a| a.key == "action" && a.value == "limit_order_expired_parked")
        })
        .count()
}

fn place_expired_bids(
    app: &mut App,
    env: &TestEnv,
    count: usize,
    escrow_each: Uint128,
    exp: u64,
) -> Vec<u64> {
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        // Large stacks need incrementing prices so each insert stays within max_adjust_steps.
        let price = if count > 32 {
            Decimal::from_ratio(100u128 + i as u128, 100u128)
        } else {
            Decimal::one()
        };
        let msg = batch_place_msg(LimitOrderSide::Bid, price, escrow_each, 32, Some(exp));
        let res = app
            .execute_contract(
                env.user.clone(),
                env.token_b.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: escrow_each,
                    msg,
                },
                &[],
            )
            .unwrap();
        ids.push(parse_limit_order_placed(&res.events));
    }
    ids
}

fn place_expired_asks(
    app: &mut App,
    env: &TestEnv,
    count: usize,
    escrow_each: Uint128,
    exp: u64,
) -> Vec<u64> {
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        let msg = batch_place_msg(
            LimitOrderSide::Ask,
            Decimal::one(),
            escrow_each,
            32,
            Some(exp),
        );
        let res = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: escrow_each,
                    msg,
                },
                &[],
            )
            .unwrap();
        ids.push(parse_limit_order_placed(&res.events));
    }
    ids
}

fn hybrid_swap_a_to_b(
    app: &mut App,
    env: &TestEnv,
    taker: &Addr,
    book_input: Uint128,
    max_maker_fills: u32,
) -> AppResponse {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input,
            max_maker_fills,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        taker.clone(),
        env.token_a.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: book_input,
            msg: swap_msg,
        },
        &[],
    )
    .unwrap()
}

fn hybrid_swap_b_to_a(
    app: &mut App,
    env: &TestEnv,
    taker: &Addr,
    book_input: Uint128,
    max_maker_fills: u32,
) -> AppResponse {
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input,
            max_maker_fills,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    app.execute_contract(
        taker.clone(),
        env.token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount: book_input,
            msg: swap_msg,
        },
        &[],
    )
    .unwrap()
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
        .rfind(|a| a.key == key)
        .map(|a| a.value.clone())
}

fn wasm_attr_in_action_event(
    events: &[cosmwasm_std::Event],
    action: &str,
    key: &str,
) -> Option<String> {
    for e in events {
        let attrs = &e.attributes;
        for (i, a) in attrs.iter().enumerate() {
            if a.key == "action" && a.value == action {
                return attrs[i..]
                    .iter()
                    .find(|x| x.key == key)
                    .map(|x| x.value.clone());
            }
        }
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
    let msg = batch_place_msg(LimitOrderSide::Ask, price, amount, 32, None);
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

/// GitLab #85 — `max_maker_fills` may be set to [`MAX_MAKER_FILLS_HARD_CAP`] (bounded match uses min with cap).
#[test]
fn hybrid_swap_accepts_max_maker_fills_at_hard_cap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_max_fill_cap");
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

    let _order_id = place_bid(
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
            max_maker_fills: MAX_MAKER_FILLS_HARD_CAP,
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

/// GitLab #238 — `HybridSimulation` applies the same CL8Y fee-tier discount as execute.
#[test]
fn hybrid_simulation_matches_execute_with_fee_discount() {
    use dex_common::pair::{
        hybrid_reverse_simulation_undiscounted, hybrid_reverse_simulation_with_trader,
        hybrid_simulation_with_trader, pool_only_hybrid_params, pool_only_hybrid_template,
        HybridReverseSimulationResponse,
    };

    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = Addr::unchecked("taker_sim_disc");

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
            "fd_sim_disc",
            None,
        )
        .unwrap();

    app.execute_contract(
        env.governance.clone(),
        fd.clone(),
        &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
            tier_id: 1,
            min_cl8y_balance: Uint128::zero(),
            discount_bps: 2500,
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
        Uint128::new(300_000),
        Decimal::one(),
    );

    let trader_str = taker.to_string();

    let assert_sim_execute_parity = |app: &mut App, hybrid: HybridSwapParams, total_in: Uint128| {
        let sim: HybridSimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &hybrid_simulation_with_trader(
                    Asset {
                        info: asset_info_token(&env.token_a),
                        amount: total_in,
                    },
                    hybrid.clone(),
                    trader_str.clone(),
                    None,
                ),
            )
            .unwrap();

        let sim_no_trader: HybridSimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &QueryMsg::HybridSimulation {
                    offer_asset: Asset {
                        info: asset_info_token(&env.token_a),
                        amount: total_in,
                    },
                    hybrid: hybrid.clone(),
                    trader: None,
                    sender: None,
                },
            )
            .unwrap();
        assert!(
            sim.return_amount > sim_no_trader.return_amount,
            "discounted sim should quote more output than undiscounted"
        );

        let taker_b_before = query_cw20_balance(app, &env.token_b, &taker);
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
        .unwrap();
        let taker_b_after = query_cw20_balance(app, &env.token_b, &taker);
        assert_eq!(
            taker_b_after.checked_sub(taker_b_before).unwrap(),
            sim.return_amount,
            "discounted HybridSimulation must match execute output"
        );
    };

    let pool_in = Uint128::new(50_000);
    assert_sim_execute_parity(&mut app, pool_only_hybrid_params(pool_in), pool_in);

    let book_in = Uint128::new(40_000);
    assert_sim_execute_parity(
        &mut app,
        HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: book_in,
            max_maker_fills: 8,
            book_start_hint: None,
        },
        book_in,
    );

    let total_in = Uint128::new(80_000);
    assert_sim_execute_parity(
        &mut app,
        HybridSwapParams {
            pool_input: Uint128::new(30_000),
            book_input: Uint128::new(50_000),
            max_maker_fills: 8,
            book_start_hint: None,
        },
        total_in,
    );

    // Reverse sim must also honor the discount: for a fixed ask, a discounted
    // trader needs strictly LESS offer than an undiscounted one (lower fee).
    // The discount is resolved once and reused across the search loop so query
    // gas stays bounded (GitLab #238 guardrail); here we assert the direction.
    let ask = Asset {
        info: asset_info_token(&env.token_b),
        amount: Uint128::new(40_000),
    };
    let rev_disc: HybridReverseSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &hybrid_reverse_simulation_with_trader(
                ask.clone(),
                pool_only_hybrid_template(),
                trader_str.clone(),
                None,
            ),
        )
        .unwrap();
    let rev_full: HybridReverseSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &hybrid_reverse_simulation_undiscounted(ask, pool_only_hybrid_template()),
        )
        .unwrap();
    assert!(
        rev_disc.offer_amount < rev_full.offer_amount,
        "discounted reverse sim should need less offer ({}) than undiscounted ({})",
        rev_disc.offer_amount,
        rev_full.offer_amount
    );
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
    let maker_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
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
    // Same owner on both bids: one aggregated token0 payout equal to total fill (GitLab #248).
    let maker_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
    assert_eq!(
        maker_a_after.checked_sub(maker_a_before).unwrap(),
        book_in,
        "aggregated maker payout should equal sum of per-fill token0 amounts"
    );
}

/// GitLab #248 — distinct makers each receive correct aggregated fill proceeds.
#[test]
fn hybrid_aggregated_maker_payouts_multi_maker() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_agg");
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

    let makers = [
        cosmwasm_std::Addr::unchecked("maker_a"),
        cosmwasm_std::Addr::unchecked("maker_b"),
        cosmwasm_std::Addr::unchecked("maker_c"),
    ];
    let mut maker_a_before = std::collections::HashMap::new();
    for m in &makers {
        transfer_tokens(&mut app, &env.token_b, &env.user, m, Uint128::new(100_000));
        maker_a_before.insert(m.clone(), query_cw20_balance(&app, &env.token_a, m));
        place_bid(
            &mut app,
            &env.pair,
            m,
            &env.token_b,
            Uint128::new(50_000),
            Decimal::one(),
        );
    }

    let book_in = Uint128::new(120_000);
    swap_a_to_b_hybrid(
        &mut app,
        &env.pair,
        &taker,
        &env.token_a,
        book_in,
        Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: book_in,
            max_maker_fills: 8,
            book_start_hint: None,
        }),
    );

    for m in &makers {
        let after = query_cw20_balance(&app, &env.token_a, m);
        assert!(
            after > *maker_a_before.get(m).unwrap(),
            "each maker should receive token0 fill proceeds"
        );
    }
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
    let taker_commission = cost_token1.multiply_ratio(15u128, 10_000u128);
    let net_to_taker = cost_token1.checked_sub(taker_commission).unwrap();

    let maker_fee_on_place = bid_escrow.multiply_ratio(15u128, 10_000u128);
    let initial_remaining = bid_escrow.checked_sub(maker_fee_on_place).unwrap();
    let lo = query_limit(&app, &env.pair, order_id);
    assert_eq!(
        lo.remaining,
        initial_remaining.checked_sub(cost_token1).unwrap()
    );

    let taker_b = query_cw20_balance(&app, &env.token_b, &taker);
    assert_eq!(taker_b, net_to_taker);

    let tre_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
    assert_eq!(
        tre_b_after.checked_sub(tre_b_before).unwrap(),
        taker_commission
    );
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
    let taker_commission = fill_t0.multiply_ratio(15u128, 10_000u128);
    let net_t0 = fill_t0.checked_sub(taker_commission).unwrap();

    let maker_fee_on_place = ask_escrow.multiply_ratio(15u128, 10_000u128);
    let initial_remaining = ask_escrow.checked_sub(maker_fee_on_place).unwrap();
    let lo = query_limit(&app, &env.pair, order_id);
    assert_eq!(
        lo.remaining,
        initial_remaining.checked_sub(fill_t0).unwrap()
    );

    let taker_a = query_cw20_balance(&app, &env.token_a, &taker);
    assert_eq!(taker_a, net_t0);

    let tre_a_after = query_cw20_balance(&app, &env.token_a, &env.treasury);
    assert_eq!(
        tre_a_after.checked_sub(tre_a_before).unwrap(),
        taker_commission
    );
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
    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        Uint128::new(10_000),
        32,
        Some(now),
    );

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
fn expired_bid_parked_on_hybrid_walk_claim_refunds_maker() {
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
    let escrow_sent = Uint128::new(10_000);
    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        escrow_sent,
        32,
        Some(exp),
    );
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: escrow_sent,
                msg,
            },
            &[],
        )
        .unwrap();
    let order_id = parse_limit_order_placed(&res.events);
    let maker_fee = escrow_sent.multiply_ratio(15u128, 10_000u128);
    let remaining_after_fee = escrow_sent.checked_sub(maker_fee).unwrap();

    let user_b_before_walk = query_cw20_balance(&app, &env.token_b, &env.user);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(HybridSwapParams {
            pool_input: Uint128::zero(),
            book_input: Uint128::new(5_000),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        trader: None,
    })
    .unwrap();
    let hybrid_res = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(5_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

    let parked_ev = hybrid_res.events.iter().find(|e| {
        e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "limit_order_expired_parked")
    });
    assert!(
        parked_ev.is_some(),
        "expired bid walk should emit limit_order_expired_parked"
    );

    let claim_row: Option<ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ExpiredLimitRefund { order_id },
        )
        .unwrap();
    let row = claim_row.expect("claim row");
    assert_eq!(row.order_id, order_id);
    assert_eq!(row.owner, env.user);
    assert_eq!(row.side, LimitOrderSide::Bid);
    assert_eq!(row.remaining, remaining_after_fee);
    assert_eq!(row.expires_at, Some(exp));

    assert!(
        app.wrap()
            .query_wasm_smart::<LimitOrderResponse>(
                env.pair.to_string(),
                &QueryMsg::LimitOrder { order_id },
            )
            .is_err(),
        "parked order should not appear in LimitOrder query"
    );

    let user_b_after_walk = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(
        user_b_after_walk, user_b_before_walk,
        "maker must not receive token1 until ClaimExpiredLimitOrder"
    );

    assert!(
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .is_err(),
        "cancel after park must fail — no active order row; prevents double CW20 return"
    );

    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::ClaimExpiredLimitOrder { order_id },
        &[],
    )
    .unwrap();

    let user_b_after_claim = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(
        user_b_after_claim.checked_sub(user_b_after_walk).unwrap(),
        remaining_after_fee
    );

    let empty: Option<ExpiredLimitRefundResponse> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::ExpiredLimitRefund { order_id },
        )
        .unwrap();
    assert!(empty.is_none());
}

// --- GitLab #250: cap expired limit parks during hybrid match walks ---

#[test]
fn hybrid_walk_parks_at_most_max_expired_bids_per_swap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp_cap");
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
    let escrow = Uint128::new(5_000);
    let expired_count = (MAX_EXPIRED_PARKS_PER_SWAP + 5) as usize;
    let order_ids = place_expired_bids(&mut app, &env, expired_count, escrow, exp);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let hybrid_res = hybrid_swap_a_to_b(&mut app, &env, &taker, Uint128::new(50_000), 8);

    assert_eq!(
        count_limit_order_expired_parked_events(&hybrid_res.events),
        MAX_EXPIRED_PARKS_PER_SWAP as usize
    );
    assert_eq!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "expired_parks_used"),
        Some(MAX_EXPIRED_PARKS_PER_SWAP.to_string())
    );
    assert_eq!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "expired_parks_capped"),
        Some("true".into())
    );
    assert_eq!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "expired_parks_skipped"),
        Some("5".into())
    );

    let mut parked = 0usize;
    for id in &order_ids[..MAX_EXPIRED_PARKS_PER_SWAP as usize] {
        let row: Option<ExpiredLimitRefundResponse> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &QueryMsg::ExpiredLimitRefund { order_id: *id },
            )
            .unwrap();
        assert!(row.is_some(), "order {id} should be parked");
        parked += 1;
    }
    assert_eq!(parked, MAX_EXPIRED_PARKS_PER_SWAP as usize);

    for id in &order_ids[MAX_EXPIRED_PARKS_PER_SWAP as usize..] {
        let row: Option<ExpiredLimitRefundResponse> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &QueryMsg::ExpiredLimitRefund { order_id: *id },
            )
            .unwrap();
        assert!(
            row.is_none(),
            "order {id} should remain on book (skipped park)"
        );
        app.wrap()
            .query_wasm_smart::<LimitOrderResponse>(
                env.pair.to_string(),
                &QueryMsg::LimitOrder { order_id: *id },
            )
            .expect("skipped expired order still queryable on book");
    }
}

#[test]
fn hybrid_walk_three_expired_bids_all_parked_then_fills_live_bid() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp3");
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
    place_expired_bids(&mut app, &env, 3, Uint128::new(5_000), exp);
    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(50_000),
        Decimal::one(),
    );

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let hybrid_res = hybrid_swap_a_to_b(&mut app, &env, &taker, Uint128::new(10_000), 8);

    assert_eq!(
        count_limit_order_expired_parked_events(&hybrid_res.events),
        3
    );
    assert_eq!(count_limit_order_fill_events(&hybrid_res.events), 1);
    assert!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "expired_parks_capped").is_none()
    );
}

#[test]
fn hybrid_walk_twenty_expired_asks_parks_cap_skips_remainder() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp_ask");
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

    let exp = app.block_info().time.seconds() + 120;
    let expired_count = (MAX_EXPIRED_PARKS_PER_SWAP + 5) as usize;
    place_expired_asks(&mut app, &env, expired_count, Uint128::new(5_000), exp);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let hybrid_res = hybrid_swap_b_to_a(&mut app, &env, &taker, Uint128::new(50_000), 8);

    assert_eq!(
        count_limit_order_expired_parked_events(&hybrid_res.events),
        MAX_EXPIRED_PARKS_PER_SWAP as usize
    );
    assert_eq!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "expired_parks_skipped"),
        Some("5".into())
    );
}

#[test]
fn skipped_expired_bid_cancelable_by_maker() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp_cancel");
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
    let expired_count = (MAX_EXPIRED_PARKS_PER_SWAP + 5) as usize;
    let order_ids = place_expired_bids(&mut app, &env, expired_count, Uint128::new(5_000), exp);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    hybrid_swap_a_to_b(&mut app, &env, &taker, Uint128::new(50_000), 8);

    let skipped_id = order_ids[MAX_EXPIRED_PARKS_PER_SWAP as usize];
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::CancelLimitOrder {
            order_id: skipped_id,
        },
        &[],
    )
    .expect("maker can cancel skipped expired order still on book");
}

#[test]
fn hybrid_simulation_matches_execute_with_expired_park_cap() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp_sim");
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
    place_expired_bids(&mut app, &env, 10, Uint128::new(5_000), exp);
    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(50_000),
        Decimal::one(),
    );

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let hybrid = HybridSwapParams {
        pool_input: Uint128::zero(),
        book_input: Uint128::new(10_000),
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
                    amount: hybrid.book_input,
                },
                hybrid: hybrid.clone(),
                trader: None,
                sender: None,
            },
        )
        .unwrap();

    let taker_b_before = query_cw20_balance(&app, &env.token_b, &taker);
    hybrid_swap_a_to_b(
        &mut app,
        &env,
        &taker,
        hybrid.book_input,
        hybrid.max_maker_fills,
    );
    let taker_b_after = query_cw20_balance(&app, &env.token_b, &taker);

    assert_eq!(
        taker_b_after.checked_sub(taker_b_before).unwrap(),
        sim.return_amount,
        "sim skips all expired; execute parks ≤ cap then skips — fill behind stack matches"
    );
}

/// GitLab #254 — scan step budget bounds expired-prefix reads; pool spillover completes swap.
#[test]
fn hybrid_walk_scan_steps_cap_bounds_expired_prefix_and_spills_to_pool() {
    use dex_common::pair::MAX_SCAN_STEPS;

    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_scan_cap");
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
    let expired_count = (MAX_SCAN_STEPS + 50) as usize;
    place_expired_bids(&mut app, &env, expired_count, Uint128::new(1_000), exp);

    app.update_block(|b| {
        b.time = b.time.plus_seconds(10_000);
    });

    let book_input = Uint128::new(50_000);
    let hybrid_res = hybrid_swap_a_to_b(&mut app, &env, &taker, book_input, 8);

    assert_eq!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "scan_steps_capped"),
        Some("true".into())
    );
    assert!(
        wasm_attr_in_action_event(&hybrid_res.events, "swap", "pool_return_amount").is_some(),
        "unfilled book budget should spill to pool"
    );
}

/// Invariant L6: `ClaimExpiredLimitOrder` is blocked while paused (same gate as cancel); succeeds after unpause.
#[test]
fn claim_expired_limit_order_blocked_while_pair_paused_then_succeeds_after_unpause() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker_exp2");
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

    let exp = app.block_info().time.seconds() + 60;
    let escrow_sent = Uint128::new(10_000);
    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        escrow_sent,
        32,
        Some(exp),
    );
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: escrow_sent,
                msg,
            },
            &[],
        )
        .unwrap();
    let order_id = parse_limit_order_placed(&res.events);
    let maker_fee = escrow_sent.multiply_ratio(15u128, 10_000u128);
    let remaining_after_fee = escrow_sent.checked_sub(maker_fee).unwrap();

    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
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
            msg: swap_msg,
        },
        &[],
    )
    .unwrap();

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

    assert!(app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrder { order_id },
            &[],
        )
        .is_err());

    let claim_while_paused = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::ClaimExpiredLimitOrder { order_id },
            &[],
        )
        .unwrap_err();
    let pause_msg = format!("{:?}", claim_while_paused.root_cause());
    assert!(
        pause_msg.contains("Paused"),
        "expected Paused error, got {}",
        pause_msg
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

    let bal_before = query_cw20_balance(&app, &env.token_b, &env.user);
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::ClaimExpiredLimitOrder { order_id },
        &[],
    )
    .unwrap();
    let bal_after = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(
        bal_after.checked_sub(bal_before).unwrap(),
        remaining_after_fee
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
    // Maker half (15 bps of 30) charged at placement; cancel refunds post-fee escrow only.
    let maker_fee = escrow.multiply_ratio(15u128, 10_000u128);
    assert_eq!(
        after.checked_sub(before).unwrap(),
        escrow.checked_sub(maker_fee).unwrap()
    );
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

    let place_msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        Uint128::new(10_000),
        32,
        None,
    );
    let place_res = app
        .execute_contract(
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

    assert_eq!(
        wasm_attr_in_action_event(&place_res.events, "place_limit_order", "action").as_deref(),
        Some("place_limit_order")
    );
    assert_eq!(
        wasm_attr_in_action_event(&place_res.events, "place_limit_order", "side").as_deref(),
        Some("bid")
    );
    assert_eq!(
        wasm_attr_in_action_event(&place_res.events, "place_limit_order", "price").as_deref(),
        Some("1")
    );
    assert_eq!(
        wasm_attr_in_action_event(&place_res.events, "place_limit_order", "owner").as_deref(),
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
    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        Uint128::new(10_000),
        32,
        None,
    );
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
/// reflects paused state; after unpause, cancel refunds bid escrow. (Parked-expiry **`ClaimExpiredLimitOrder`**
/// is pause-gated too — GitLab #120 — see `claim_expired_limit_order_blocked_while_pair_paused_then_succeeds_after_unpause`.)
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

    let place_msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::one(),
        Uint128::new(10_000),
        32,
        None,
    );
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
    let escrow = Uint128::new(50_000);
    let maker_fee = escrow.multiply_ratio(15u128, 10_000u128);
    assert_eq!(
        bal_after.checked_sub(bal_before).unwrap(),
        escrow.checked_sub(maker_fee).unwrap()
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
    let per_order_maker_fee = Uint128::new(100_000).multiply_ratio(15u128, 10_000u128);
    let rem_after_place = Uint128::new(100_000)
        .checked_sub(per_order_maker_fee)
        .unwrap();
    assert_eq!(
        lo_a.remaining,
        rem_after_place.checked_sub(Uint128::new(50_000)).unwrap()
    );
    assert_eq!(lo_b.remaining, rem_after_place);
}

#[test]
fn update_limit_order_price_changes_price_not_remaining() {
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
    let before = query_limit(&app, &env.pair, order_id);
    let new_price = Decimal::from_ratio(11u128, 10u128);
    app.execute_contract(
        env.user.clone(),
        env.pair.clone(),
        &ExecuteMsg::UpdateLimitOrderPrice {
            order_id,
            price: new_price,
            hint_after_order_id: None,
            max_adjust_steps: 32,
        },
        &[],
    )
    .unwrap();
    let after = query_limit(&app, &env.pair, order_id);
    assert_eq!(after.order_id, order_id);
    assert_eq!(after.remaining, before.remaining);
    assert_eq!(after.price, new_price);
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
                trader: None,
                sender: None,
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

/// GitLab #197 — hybrid execute enforces unified `max_spread` (pool spread / total gross out).
#[test]
fn hybrid_max_spread_exact_tolerance_succeeds() {
    use dex_common::max_spread::{self, MaxSpreadInputs};

    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(200_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_max_spread_ok");
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
                trader: None,
                sender: None,
            },
        )
        .unwrap();
    assert!(sim.book_return_amount > Uint128::zero());

    let inputs = MaxSpreadInputs::from_hybrid_simulation(
        total_in,
        sim.pool_return_amount,
        sim.pool_commission_amount,
        sim.spread_amount,
        sim.book_return_amount,
    );
    let pool_gross = sim
        .pool_return_amount
        .checked_add(sim.pool_commission_amount)
        .unwrap();
    let spread_cmp = sim.spread_amount.min(pool_gross);
    let total_gross = pool_gross.checked_add(sim.book_return_amount).unwrap();
    let exact_max = Decimal::from_ratio(spread_cmp, total_gross);
    max_spread::check_max_spread(None, Some(exact_max), &inputs).unwrap();

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(exact_max),
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
    .unwrap();
}

#[test]
fn hybrid_max_spread_tighter_than_simulation_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(200_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_max_spread_fail");
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

    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::zero()),
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
        trader: None,
    })
    .unwrap();
    let err = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Max spread assertion"),
        "hybrid swap should reject when pool spread / total gross exceeds max_spread"
    );
}

#[test]
fn hybrid_belief_price_max_spread_rejects_shortfall_on_total_output() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(200_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_belief_hybrid");
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
    // Belief implies 2:1 ask output vs offer — actual hybrid output is far lower.
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: Some(Decimal::from_ratio(Uint128::one(), Uint128::new(2))),
        max_spread: Some(Decimal::permille(1)),
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
        trader: None,
    })
    .unwrap();
    let err = app
        .execute_contract(
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total_in,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Max spread assertion"),
        "belief_price path must count book + pool output on hybrid swaps"
    );
}

/// GitLab #196 — `AfterSwap` / `HybridSimulation` commission is pool + book taker fees (ask asset).
#[test]
fn hybrid_hook_commission_includes_pool_and_book() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(200_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_l7_hybrid");
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
                trader: None,
                sender: None,
            },
        )
        .unwrap();

    let tre_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
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

    let pool_commission = wasm_attr_in_action_event(&res.events, "swap", "commission_amount")
        .expect("pool commission_amount on swap")
        .parse::<u128>()
        .unwrap();
    let book_commission = wasm_attr_in_action_event(&res.events, "swap", "book_commission_amount")
        .expect("book_commission_amount on hybrid swap")
        .parse::<u128>()
        .unwrap();

    assert_eq!(
        sim.commission_amount.u128(),
        pool_commission + book_commission,
        "L7: sim commission should equal pool + book wasm attrs"
    );
    assert!(
        book_commission > 0,
        "book leg should charge taker commission"
    );

    let tre_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
    assert_eq!(
        tre_b_after.checked_sub(tre_b_before).unwrap(),
        sim.commission_amount,
        "L7: treasury ask-token delta should match total commission"
    );
}

/// GitLab #196 — pool-only swaps: hook/sim commission unchanged (book component zero).
#[test]
fn pool_only_hook_commission_unchanged() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let offer = Uint128::new(50_000);
    let hybrid = dex_common::pair::pool_only_hybrid_params(offer);
    let sim: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: offer,
                },
                hybrid: hybrid.clone(),
                trader: None,
                sender: None,
            },
        )
        .unwrap();

    let tre_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
    let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
        belief_price: None,
        max_spread: Some(Decimal::one()),
        to: None,
        deadline: None,
        hybrid: Some(hybrid),
        trader: None,
    })
    .unwrap();
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: offer,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

    let pool_commission = wasm_attr_in_action_event(&res.events, "swap", "commission_amount")
        .expect("commission_amount")
        .parse::<u128>()
        .unwrap();
    assert_eq!(sim.commission_amount.u128(), pool_commission);
    assert!(
        wasm_attr_in_action_event(&res.events, "swap", "book_commission_amount").is_none(),
        "pool-only swap should not emit book_commission_amount"
    );

    let tre_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
    assert_eq!(
        tre_b_after.checked_sub(tre_b_before).unwrap(),
        sim.commission_amount
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

    let msg = batch_place_msg(
        LimitOrderSide::Bid,
        Decimal::from_ratio(5u128, 10u128),
        Uint128::new(10_000),
        5,
        None,
    );

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
    let s = err.root_cause().to_string();
    assert!(
        s.contains("max adjust steps")
            || s.contains("Limit order insert")
            || s.contains("book-walk cap"),
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
                trader: None,
                sender: None,
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
                trader: None,
                sender: None,
            },
        )
        .unwrap();

    assert_eq!(sim_none.amount, sim_hybrid.amount);

    let direct: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: offer,
                },
                hybrid: dex_common::pair::pool_only_hybrid_params(offer),
                trader: None,
                sender: None,
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
                trader: None,
                sender: None,
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

/// GitLab #192 — 3-hop router path with hybrid on ≥2 legs; sim vs execute parity (L8).
fn hybrid_params_split(total: Uint128, pool_num: u128, book_num: u128) -> HybridSwapParams {
    let denom = pool_num + book_num;
    let pool_input = total.multiply_ratio(pool_num, denom);
    let book_input = total.checked_sub(pool_input).unwrap();
    HybridSwapParams {
        pool_input,
        book_input,
        max_maker_fills: 8,
        book_start_hint: None,
    }
}

fn router_simulate_amount(
    app: &App,
    router: &Addr,
    offer_amount: Uint128,
    operations: &[cl8y_dex_router::msg::SwapOperation],
) -> Uint128 {
    let sim: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
        .wrap()
        .query_wasm_smart(
            router.to_string(),
            &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                offer_amount,
                operations: operations.to_vec(),
                trader: None,
                sender: None,
            },
        )
        .unwrap();
    sim.amount
}

#[test]
fn router_three_hop_two_legs_hybrid_matches_simulate() {
    let mut app = App::default();
    let abcd = setup_router_abcd_env(&mut app);
    let abc = &abcd.abc;
    let env = &abc.env;

    place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(500_000),
        Decimal::one(),
    );
    place_bid(
        &mut app,
        &abc.pair_bc,
        &env.user,
        &abc.token_c,
        Uint128::new(500_000),
        Decimal::one(),
    );

    let taker = cosmwasm_std::Addr::unchecked("taker_3hop_multi_hybrid");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &taker,
        Uint128::new(500_000),
    );

    let offer_a = Uint128::new(100_000);
    let hop1_hybrid = hybrid_params_split(offer_a, 1, 3);

    let hop1_only = vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
        offer_asset_info: asset_info_token(&env.token_a),
        ask_asset_info: asset_info_token(&env.token_b),
        hybrid: Some(hop1_hybrid.clone()),
    }];
    let offer_b = router_simulate_amount(&app, &env.router, offer_a, &hop1_only);

    let hop2_hybrid = hybrid_params_split(offer_b, 2, 3);
    let hops_ab = vec![
        cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_a),
            ask_asset_info: asset_info_token(&env.token_b),
            hybrid: Some(hop1_hybrid),
        },
        cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&env.token_b),
            ask_asset_info: asset_info_token(&abc.token_c),
            hybrid: Some(hop2_hybrid),
        },
    ];
    let _offer_c = router_simulate_amount(&app, &env.router, offer_a, &hops_ab);

    let operations = vec![
        hops_ab[0].clone(),
        hops_ab[1].clone(),
        cl8y_dex_router::msg::SwapOperation::TerraSwap {
            offer_asset_info: asset_info_token(&abc.token_c),
            ask_asset_info: asset_info_token(&abcd.token_d),
            hybrid: None,
        },
    ];

    let sim_amount = router_simulate_amount(&app, &env.router, offer_a, &operations);

    let d_before = query_cw20_balance(&app, &abcd.token_d, &taker);
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
    assert!(
        count_limit_order_fill_events(&res.events) >= 2,
        "expected book fills on both hybrid legs"
    );
    let d_after = query_cw20_balance(&app, &abcd.token_d, &taker);
    let got_d = d_after.checked_sub(d_before).unwrap();
    assert_eq!(
        got_d, sim_amount,
        "L8: 3-hop router output should match SimulateSwapOperations (hybrid on hops 1 and 2)"
    );
}

#[test]
fn hybrid_reverse_pool_only_template_is_stable() {
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
    let hrev: HybridReverseSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridReverseSimulation {
                ask_asset: Asset {
                    info: asset_info_token(&env.token_b),
                    amount: ask_amt,
                },
                hybrid: dex_common::pair::pool_only_hybrid_template(),
                trader: None,
                sender: None,
            },
        )
        .unwrap();

    assert!(hrev.offer_amount > Uint128::zero());
    assert_eq!(hrev.book_return_amount, Uint128::zero());
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
                trader: None,
                sender: None,
            },
        )
        .unwrap();
    let p: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: Asset {
                    info: asset_info_token(&env.token_a),
                    amount: offer,
                },
                hybrid: dex_common::pair::pool_only_hybrid_params(offer),
                trader: None,
                sender: None,
            },
        )
        .unwrap();
    assert_eq!(h.return_amount, p.return_amount);
    assert_eq!(h.book_return_amount, Uint128::zero());
}

#[test]
fn place_limit_order_ladder_five_rungs() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let total = Uint128::new(50_000);
    let ladder = LimitOrderLadderSpec {
        side: LimitOrderSide::Bid,
        start_price: Decimal::from_ratio(95u128, 100u128),
        end_price: Decimal::one(),
        count: 5,
        total_amount: total,
        distribution: LimitLadderDistribution::Equal,
        max_adjust_steps: 32,
        expires_at: None,
    };
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderLadder { ladder }).unwrap();
    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total,
                msg,
            },
            &[],
        )
        .unwrap();

    let placed: Vec<u64> = res
        .events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|a| a.key == "limit_order_placed")
        .map(|a| a.value.parse().unwrap())
        .collect();
    assert_eq!(placed.len(), 5);
    assert_eq!(
        wasm_attr_in_action_event(&res.events, "place_limit_order_batch", "action").as_deref(),
        Some("place_limit_order_batch")
    );
    assert_eq!(
        wasm_attr_in_action_event(&res.events, "place_limit_order_batch", "batch_count").as_deref(),
        Some("5")
    );
}

/// GitLab #247: batch reserves ids in one block; sequence must match sequential singles.
#[test]
fn batch_placement_order_ids_match_sequential_singles() {
    fn three_rung_ids(app: &mut App, env: &TestEnv, batch: bool) -> Vec<u64> {
        provide_liquidity(
            app,
            env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );
        let prices = [
            Decimal::from_ratio(95u128, 100u128),
            Decimal::from_ratio(96u128, 100u128),
            Decimal::from_ratio(97u128, 100u128),
        ];
        if batch {
            let orders: Vec<LimitOrderPlacementItem> = prices
                .iter()
                .map(|p| LimitOrderPlacementItem {
                    price: *p,
                    amount: Uint128::new(10_000),
                    max_adjust_steps: 32,
                    expires_at: None,
                })
                .collect();
            let total = Uint128::new(30_000);
            let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
                side: LimitOrderSide::Bid,
                orders,
            })
            .unwrap();
            let res = app
                .execute_contract(
                    env.user.clone(),
                    env.token_b.clone(),
                    &cw20::Cw20ExecuteMsg::Send {
                        contract: env.pair.to_string(),
                        amount: total,
                        msg,
                    },
                    &[],
                )
                .unwrap();
            res.events
                .iter()
                .flat_map(|e| e.attributes.iter())
                .filter(|a| a.key == "limit_order_placed")
                .map(|a| a.value.parse().unwrap())
                .collect()
        } else {
            prices
                .iter()
                .map(|p| {
                    place_bid(
                        app,
                        &env.pair,
                        &env.user,
                        &env.token_b,
                        Uint128::new(10_000),
                        *p,
                    )
                })
                .collect()
        }
    }

    let mut batch_app = App::default();
    let batch_env = setup_full_env(&mut batch_app);
    let batch_ids = three_rung_ids(&mut batch_app, &batch_env, true);

    let mut seq_app = App::default();
    let seq_env = setup_full_env(&mut seq_app);
    let seq_ids = three_rung_ids(&mut seq_app, &seq_env, false);

    assert_eq!(batch_ids, seq_ids);
    assert_eq!(batch_ids.len(), 3);
}

#[test]
fn limit_batch_partial_success_skips_book_walk_failures() {
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

    // Rung 2 uses a better price than rung 1 so batch hint-chaining cannot O(1)-insert;
    // head walk with max_adjust_steps=5 fails against the seeded deep book (GitLab #256).
    let orders = vec![
        LimitOrderPlacementItem {
            price: Decimal::from_ratio(99u128, 100u128),
            amount: Uint128::new(1_000),
            max_adjust_steps: 32,
            expires_at: None,
        },
        LimitOrderPlacementItem {
            price: Decimal::one(),
            amount: Uint128::new(1_000),
            max_adjust_steps: 5,
            expires_at: None,
        },
        LimitOrderPlacementItem {
            price: Decimal::from_ratio(98u128, 100u128),
            amount: Uint128::new(1_000),
            max_adjust_steps: 32,
            expires_at: None,
        },
    ];
    let total = Uint128::new(3_000);
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side: LimitOrderSide::Bid,
        orders,
    })
    .unwrap();

    let res = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total,
                msg,
            },
            &[],
        )
        .unwrap();

    let placed: Vec<u64> = res
        .events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|a| a.key == "limit_order_placed")
        .map(|a| a.value.parse().unwrap())
        .collect();
    assert_eq!(placed.len(), 2, "expected two rungs placed, one skipped");
    assert_eq!(
        wasm_attr_in_action_event(
            &res.events,
            "place_limit_order_batch",
            "batch_skipped_count"
        )
        .as_deref(),
        Some("1")
    );
    assert_eq!(
        wasm_attr_in_action_event(
            &res.events,
            "place_limit_order_batch",
            "batch_refund_amount"
        )
        .as_deref(),
        Some("1000")
    );
}

#[test]
fn limit_batch_too_large_rejected() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let cfg: LimitOrderConfigResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::LimitOrderConfig {})
        .unwrap();
    let max = cfg.max_batch_rungs;

    let mut orders = Vec::new();
    for i in 0..=max {
        orders.push(LimitOrderPlacementItem {
            price: Decimal::from_ratio(90u128 + i as u128, 100u128),
            amount: Uint128::new(1000),
            max_adjust_steps: 32,
            expires_at: None,
        });
    }
    let total: Uint128 = orders.iter().map(|o| o.amount).sum();
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side: LimitOrderSide::Bid,
        orders,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total,
                msg,
            },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(
        s.contains("max_batch_rungs") || s.contains("Limit batch"),
        "{}",
        s
    );
}

#[test]
fn governance_set_pair_limit_batch_max_enforced() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    app.execute_contract(
        env.governance.clone(),
        env.factory.clone(),
        &FactoryExecuteMsg::SetPairLimitBatchMax {
            pair: env.pair.to_string(),
            max_rungs: 2,
        },
        &[],
    )
    .unwrap();

    let orders = vec![
        LimitOrderPlacementItem {
            price: Decimal::from_ratio(9u128, 10u128),
            amount: Uint128::new(1000),
            max_adjust_steps: 32,
            expires_at: None,
        },
        LimitOrderPlacementItem {
            price: Decimal::from_ratio(91u128, 100u128),
            amount: Uint128::new(1000),
            max_adjust_steps: 32,
            expires_at: None,
        },
        LimitOrderPlacementItem {
            price: Decimal::from_ratio(92u128, 100u128),
            amount: Uint128::new(1000),
            max_adjust_steps: 32,
            expires_at: None,
        },
    ];
    let total = Uint128::new(3000);
    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side: LimitOrderSide::Bid,
        orders,
    })
    .unwrap();

    let err = app
        .execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: total,
                msg,
            },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(
        s.contains("max_batch_rungs") || s.contains("Limit batch"),
        "{}",
        s
    );
}

// --- GitLab #246: batch cancel / batch claim expired ---

fn count_cancelled_ids(events: &[cosmwasm_std::Event]) -> Vec<u64> {
    events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|a| a.key == "limit_order_cancelled")
        .map(|a| a.value.parse().unwrap())
        .collect()
}

#[test]
fn batch_cancel_mixed_bid_ask_one_tx() {
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
    let ask_escrow = Uint128::new(50_000);
    let bid_id = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        bid_escrow,
        Decimal::one(),
    );
    let ask_id = place_ask(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_a,
        ask_escrow,
        Decimal::one(),
    );

    let bal_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
    let bal_a_before = query_cw20_balance(&app, &env.token_a, &env.user);

    let res = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrders {
                order_ids: vec![bid_id, ask_id],
            },
            &[],
        )
        .unwrap();

    assert_eq!(
        wasm_attr_in_action_event(&res.events, "cancel_limit_orders_batch", "batch_count")
            .as_deref(),
        Some("2")
    );
    assert_eq!(count_cancelled_ids(&res.events).len(), 2);

    let maker_fee_bid = bid_escrow.multiply_ratio(15u128, 10_000u128);
    let maker_fee_ask = ask_escrow.multiply_ratio(15u128, 10_000u128);
    let bal_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
    let bal_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
    assert_eq!(
        bal_b_after.checked_sub(bal_b_before).unwrap(),
        bid_escrow.checked_sub(maker_fee_bid).unwrap()
    );
    assert_eq!(
        bal_a_after.checked_sub(bal_a_before).unwrap(),
        ask_escrow.checked_sub(maker_fee_ask).unwrap()
    );
}

#[test]
fn batch_cancel_empty_reverts() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let err = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrders { order_ids: vec![] },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(s.contains("Limit batch") || s.contains("empty"), "{}", s);
}

#[test]
fn batch_cancel_duplicate_id_reverts() {
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
        Uint128::new(10_000),
        Decimal::one(),
    );
    let err = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrders {
                order_ids: vec![id, id],
            },
            &[],
        )
        .unwrap_err();
    let s = err.root_cause().to_string();
    assert!(s.contains("Duplicate") || s.contains("duplicate"), "{}", s);
}

#[test]
fn batch_cancel_foreign_owner_reverts_whole_tx() {
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
    let id2 = place_bid(
        &mut app,
        &env.pair,
        &env.user,
        &env.token_b,
        Uint128::new(10_000),
        Decimal::one(),
    );
    let attacker = cosmwasm_std::Addr::unchecked("attacker246");
    let err = app
        .execute_contract(
            attacker,
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrders {
                order_ids: vec![id1, id2],
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Unauthorized"),
        "{}",
        err
    );
    // Orders still on book
    let _: LimitOrderResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::LimitOrder { order_id: id1 },
        )
        .unwrap();
}

#[test]
fn batch_cancel_while_paused_reverts() {
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
        Uint128::new(10_000),
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
    let err = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::CancelLimitOrders {
                order_ids: vec![id],
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{:?}", err.root_cause()).contains("Paused"),
        "{}",
        err
    );
}

#[test]
fn batch_claim_expired_two_orders_one_tx() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let taker = cosmwasm_std::Addr::unchecked("taker246");
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

    let exp = app.block_info().time.seconds() + 60;
    let escrow = Uint128::new(10_000);
    let mut order_ids = Vec::new();
    for _ in 0..2 {
        let msg = batch_place_msg(LimitOrderSide::Bid, Decimal::one(), escrow, 32, Some(exp));
        let res = app
            .execute_contract(
                env.user.clone(),
                env.token_b.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: escrow,
                    msg,
                },
                &[],
            )
            .unwrap();
        order_ids.push(parse_limit_order_placed(&res.events));
    }

    app.update_block(|b| {
        b.time = b.time.plus_seconds(120);
    });

    for _ in 0..2 {
        let swap_msg = to_json_binary(&Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
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
            taker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    }

    let maker_fee = escrow.multiply_ratio(15u128, 10_000u128);
    let remaining = escrow.checked_sub(maker_fee).unwrap();
    let bal_before = query_cw20_balance(&app, &env.token_b, &env.user);

    let res = app
        .execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &ExecuteMsg::ClaimExpiredLimitOrders {
                order_ids: order_ids.clone(),
            },
            &[],
        )
        .unwrap();

    assert_eq!(
        wasm_attr_in_action_event(
            &res.events,
            "claim_expired_limit_orders_batch",
            "batch_count"
        )
        .as_deref(),
        Some("2")
    );
    let bal_after = query_cw20_balance(&app, &env.token_b, &env.user);
    assert_eq!(
        bal_after.checked_sub(bal_before).unwrap(),
        remaining.checked_mul(Uint128::new(2)).unwrap()
    );
}
