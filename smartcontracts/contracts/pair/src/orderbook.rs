//! FIFO doubly-linked limit orders per pair side (bids / asks).
//!
//! ## Ordering (composite key, strict total order)
//!
//! Price is always **token1 per token0** (same basis as pool pricing).
//!
//! - **Bids** (makers escrow token1; matched on taker **token0 → token1**):
//!   Walk best-first: **descending** `price`, then **ascending** `order_id` (FIFO at same price).
//! - **Asks** (makers escrow token0; matched on taker **token1 → token0**):
//!   Walk best-first: **ascending** `price`, then **ascending** `order_id` (FIFO at same price).

use cosmwasm_std::{Addr, CosmosMsg, Decimal, StdError, StdResult, Storage, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use dex_common::pair::{LimitOrderResponse, LimitOrderSide};

use crate::error::ContractError;
use crate::state::{
    LimitOrder, HEAD_ASK, HEAD_BID, ORDERS, ORDER_NEXT_ID, PENDING_ESCROW_TOKEN0,
    PENDING_ESCROW_TOKEN1,
};

use dex_common::pair::{MAX_ADJUST_STEPS_HARD_CAP, MAX_MAKER_FILLS_HARD_CAP};

/// `true` if order `a` should be closer to the bid head than `b` (better bid first).
pub fn bid_before(a_price: Decimal, a_id: u64, b_price: Decimal, b_id: u64) -> bool {
    a_price > b_price || (a_price == b_price && a_id < b_id)
}

/// `true` if order `a` should be closer to the ask head than `b` (better ask first).
pub fn ask_before(a_price: Decimal, a_id: u64, b_price: Decimal, b_id: u64) -> bool {
    a_price < b_price || (a_price == b_price && a_id < b_id)
}

fn next_order_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let id = ORDER_NEXT_ID.may_load(storage)?.unwrap_or(1u64);
    ORDER_NEXT_ID.save(
        storage,
        &(id.checked_add(1)
            .ok_or_else(|| ContractError::InvariantViolation {
                reason: "order id overflow".into(),
            })?),
    )?;
    Ok(id)
}

/// Insert a bid (descending price, then ascending order_id). Returns new order id.
#[allow(clippy::too_many_arguments)]
pub fn insert_bid(
    storage: &mut dyn Storage,
    price: Decimal,
    remaining_token1: Uint128,
    owner: Addr,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
) -> Result<u64, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let id = next_order_id(storage)?;
    let mut steps: u32 = 0;

    let head = HEAD_BID.may_load(storage)?.flatten();
    let (prev, next) = if head.is_none() {
        (None, None)
    } else {
        find_insert_bid(storage, head, hint_after, price, id, max_steps, &mut steps)?
    };

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token1,
        side: LimitOrderSide::Bid,
        prev,
        next,
    };
    ORDERS.save(storage, id, &order)?;

    // link neighbors
    if let Some(p) = prev {
        ORDERS
            .update(storage, p, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("prev neighbor"))?;
                x.next = Some(id);
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    } else {
        HEAD_BID.save(storage, &Some(id))?;
    }
    if let Some(n) = next {
        ORDERS
            .update(storage, n, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("next neighbor"))?;
                x.prev = Some(id);
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    }

    let mut esc = PENDING_ESCROW_TOKEN1
        .may_load(storage)?
        .unwrap_or(Uint128::zero());
    esc = esc
        .checked_add(remaining_token1)
        .map_err(ContractError::Overflow)?;
    PENDING_ESCROW_TOKEN1.save(storage, &esc)?;

    Ok(id)
}

/// Insert an ask (ascending price, then ascending order_id).
#[allow(clippy::too_many_arguments)]
pub fn insert_ask(
    storage: &mut dyn Storage,
    price: Decimal,
    remaining_token0: Uint128,
    owner: Addr,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
) -> Result<u64, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let id = next_order_id(storage)?;
    let mut steps: u32 = 0;

    let head = HEAD_ASK.may_load(storage)?.flatten();
    let (prev, next) = if head.is_none() {
        (None, None)
    } else {
        find_insert_ask(storage, head, hint_after, price, id, max_steps, &mut steps)?
    };

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token0,
        side: LimitOrderSide::Ask,
        prev,
        next,
    };
    ORDERS.save(storage, id, &order)?;

    if let Some(p) = prev {
        ORDERS
            .update(storage, p, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("prev neighbor"))?;
                x.next = Some(id);
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    } else {
        HEAD_ASK.save(storage, &Some(id))?;
    }
    if let Some(n) = next {
        ORDERS
            .update(storage, n, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("next neighbor"))?;
                x.prev = Some(id);
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    }

    let mut esc = PENDING_ESCROW_TOKEN0
        .may_load(storage)?
        .unwrap_or(Uint128::zero());
    esc = esc
        .checked_add(remaining_token0)
        .map_err(ContractError::Overflow)?;
    PENDING_ESCROW_TOKEN0.save(storage, &esc)?;

    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn find_insert_bid(
    storage: &dyn Storage,
    head: Option<u64>,
    _hint_after: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<(Option<u64>, Option<u64>), ContractError> {
    // Linear walk from head (indexer hints are advisory; full verify is head-only within max_steps).
    let mut prev: Option<u64> = None;
    let mut cur = head;

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if bid_before(new_price, new_id, ord.price, cid) {
            // insert before cid
            return Ok((ord.prev, Some(cid)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok((prev, None))
}

#[allow(clippy::too_many_arguments)]
fn find_insert_ask(
    storage: &dyn Storage,
    head: Option<u64>,
    _hint_after: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<(Option<u64>, Option<u64>), ContractError> {
    let mut prev: Option<u64> = None;
    let mut cur = head;

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if ask_before(new_price, new_id, ord.price, cid) {
            return Ok((ord.prev, Some(cid)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok((prev, None))
}

/// Remove an order from its list and delete storage (refund handled by caller).
pub fn unlink_order(storage: &mut dyn Storage, id: u64) -> StdResult<LimitOrder> {
    let order = ORDERS.load(storage, id)?;
    let head = match order.side {
        LimitOrderSide::Bid => HEAD_BID.may_load(storage)?.flatten(),
        LimitOrderSide::Ask => HEAD_ASK.may_load(storage)?.flatten(),
    };

    if head == Some(id) {
        match order.side {
            LimitOrderSide::Bid => HEAD_BID.save(storage, &order.next)?,
            LimitOrderSide::Ask => HEAD_ASK.save(storage, &order.next)?,
        }
    }

    if let Some(p) = order.prev {
        ORDERS.update(storage, p, |o| -> StdResult<LimitOrder> {
            let mut x = o.ok_or_else(|| StdError::generic_err("prev"))?;
            x.next = order.next;
            Ok(x)
        })?;
    }
    if let Some(n) = order.next {
        ORDERS.update(storage, n, |o| -> StdResult<LimitOrder> {
            let mut x = o.ok_or_else(|| StdError::generic_err("next"))?;
            x.prev = order.prev;
            Ok(x)
        })?;
    }

    ORDERS.remove(storage, id);
    Ok(order)
}

/// Match bids while taker sells token0 for token1. `token0_budget` is filled from the taker.
#[allow(clippy::too_many_arguments)]
pub fn match_bids(
    storage: &mut dyn Storage,
    token0_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    token0_addr: &str,
    token1_addr: &str,
    receiver: &Addr,
    treasury: &Addr,
    effective_fee_bps: u16,
) -> Result<(Uint128, Uint128, u32, Vec<CosmosMsg>), ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let mut token0_left = token0_budget;
    let mut token1_out_total = Uint128::zero();
    let mut makers_used = 0u32;
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    let mut cur = if let Some(h) = book_start_hint {
        if ORDERS.may_load(storage, h)?.is_some() {
            Some(h)
        } else {
            HEAD_BID.may_load(storage)?.flatten()
        }
    } else {
        HEAD_BID.may_load(storage)?.flatten()
    };

    while token0_left > Uint128::zero() && makers_used < cap {
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let mut order = ORDERS.load(storage, oid)?;
        if order.remaining.is_zero() {
            cur = order.next;
            continue;
        }

        if order.price.is_zero() {
            return Err(ContractError::InvariantViolation {
                reason: "zero bid price".into(),
            });
        }
        let inv = Decimal::one().checked_div(order.price).map_err(|_| {
            ContractError::InvariantViolation {
                reason: "bid price div".into(),
            }
        })?;
        // Max token0 purchasable with this bid's remaining token1 budget at price (token1 per token0).
        let max_fill_from_bid = order.remaining.checked_mul_floor(inv).map_err(|_| {
            ContractError::InvariantViolation {
                reason: "bid max fill".into(),
            }
        })?;
        let mut fill = token0_left.min(max_fill_from_bid);
        if fill.is_zero() {
            cur = order.next;
            continue;
        }

        let mut cost =
            fill.checked_mul_floor(order.price)
                .map_err(|_| ContractError::InvariantViolation {
                    reason: "cost mul_floor".into(),
                })?;
        while fill > Uint128::zero() && cost > order.remaining {
            fill = fill.saturating_sub(Uint128::one());
            if fill.is_zero() {
                break;
            }
            cost = fill.checked_mul_floor(order.price).map_err(|_| {
                ContractError::InvariantViolation {
                    reason: "cost mul_floor adjust".into(),
                }
            })?;
        }

        if fill.is_zero() {
            cur = order.next;
            continue;
        }

        makers_used += 1;

        let commission = cost
            .checked_mul(Uint128::new(effective_fee_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = cost.checked_sub(commission)?;

        // Maker receives fill token0 from contract (taker funds already received by pair)
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token0_addr.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: order.owner.to_string(),
                amount: fill,
            })?,
            funds: vec![],
        }));

        if !net_to_taker.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token1_addr.to_string(),
                msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: receiver.to_string(),
                    amount: net_to_taker,
                })?,
                funds: vec![],
            }));
        }
        if !commission.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token1_addr.to_string(),
                msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: treasury.to_string(),
                    amount: commission,
                })?,
                funds: vec![],
            }));
        }

        let mut esc = PENDING_ESCROW_TOKEN1
            .may_load(storage)?
            .unwrap_or(Uint128::zero());
        esc = esc.saturating_sub(cost);
        PENDING_ESCROW_TOKEN1.save(storage, &esc)?;

        order.remaining = order.remaining.checked_sub(cost)?;
        token0_left = token0_left.checked_sub(fill)?;
        token1_out_total = token1_out_total.checked_add(net_to_taker)?;

        if order.remaining.is_zero() {
            unlink_order(storage, oid)?;
        } else {
            ORDERS.save(storage, oid, &order)?;
        }

        cur = order.next;
    }

    Ok((
        token1_out_total,
        token0_budget.checked_sub(token0_left)?,
        makers_used,
        msgs,
    ))
}

/// Match asks while taker sells token1 for token0. `token1_budget` is from the taker.
#[allow(clippy::too_many_arguments)]
pub fn match_asks(
    storage: &mut dyn Storage,
    token1_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    token0_addr: &str,
    token1_addr: &str,
    receiver: &Addr,
    treasury: &Addr,
    effective_fee_bps: u16,
) -> Result<(Uint128, Uint128, u32, Vec<CosmosMsg>), ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let mut token1_left = token1_budget;
    let mut token0_out_total = Uint128::zero();
    let mut makers_used = 0u32;
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    let mut cur = if let Some(h) = book_start_hint {
        if ORDERS.may_load(storage, h)?.is_some() {
            Some(h)
        } else {
            HEAD_ASK.may_load(storage)?.flatten()
        }
    } else {
        HEAD_ASK.may_load(storage)?.flatten()
    };

    while token1_left > Uint128::zero() && makers_used < cap {
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let mut order = ORDERS.load(storage, oid)?;
        if order.remaining.is_zero() {
            cur = order.next;
            continue;
        }

        let max_fill_token0_from_ask = order.remaining;
        let max_fill_token0_from_budget = if !order.price.is_zero() {
            token1_left
                .checked_mul_floor(Decimal::one().checked_div(order.price).map_err(|_| {
                    ContractError::InvariantViolation {
                        reason: "ask price div".into(),
                    }
                })?)
                .map_err(|_| ContractError::InvariantViolation {
                    reason: "ask mul_floor".into(),
                })?
        } else {
            Uint128::zero()
        };

        let mut fill_t0 = max_fill_token0_from_ask.min(max_fill_token0_from_budget);
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }

        let mut cost = fill_t0.checked_mul_floor(order.price).map_err(|_| {
            ContractError::InvariantViolation {
                reason: "ask cost".into(),
            }
        })?;
        while fill_t0 > Uint128::zero() && cost > token1_left {
            fill_t0 = fill_t0.saturating_sub(Uint128::one());
            if fill_t0.is_zero() {
                break;
            }
            cost = fill_t0.checked_mul_floor(order.price).map_err(|_| {
                ContractError::InvariantViolation {
                    reason: "ask cost adjust".into(),
                }
            })?;
        }
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }

        makers_used += 1;

        let commission = cost
            .checked_mul(Uint128::new(effective_fee_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = fill_t0.checked_sub(commission)?;

        // Pay token1 cost from taker's funds held by contract to maker
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token1_addr.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: order.owner.to_string(),
                amount: cost,
            })?,
            funds: vec![],
        }));

        if !net_to_taker.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token0_addr.to_string(),
                msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: receiver.to_string(),
                    amount: net_to_taker,
                })?,
                funds: vec![],
            }));
        }
        if !commission.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token0_addr.to_string(),
                msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: treasury.to_string(),
                    amount: commission,
                })?,
                funds: vec![],
            }));
        }

        let mut esc = PENDING_ESCROW_TOKEN0
            .may_load(storage)?
            .unwrap_or(Uint128::zero());
        esc = esc.saturating_sub(fill_t0);
        PENDING_ESCROW_TOKEN0.save(storage, &esc)?;

        order.remaining = order.remaining.checked_sub(fill_t0)?;
        token1_left = token1_left.checked_sub(cost)?;
        token0_out_total = token0_out_total.checked_add(net_to_taker)?;

        if order.remaining.is_zero() {
            unlink_order(storage, oid)?;
        } else {
            ORDERS.save(storage, oid, &order)?;
        }

        cur = order.next;
    }

    Ok((
        token0_out_total,
        token1_budget.checked_sub(token1_left)?,
        makers_used,
        msgs,
    ))
}

pub fn load_order_response(storage: &dyn Storage, id: u64) -> StdResult<LimitOrderResponse> {
    let o = ORDERS.load(storage, id)?;
    Ok(LimitOrderResponse {
        order_id: id,
        owner: o.owner,
        side: o.side,
        price: o.price,
        remaining: o.remaining,
        prev: o.prev,
        next: o.next,
    })
}

pub fn query_head(storage: &dyn Storage, side: LimitOrderSide) -> StdResult<Option<u64>> {
    match side {
        LimitOrderSide::Bid => Ok(HEAD_BID.may_load(storage)?.flatten()),
        LimitOrderSide::Ask => Ok(HEAD_ASK.may_load(storage)?.flatten()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn bid_head_is_best_price_then_fifo() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let a = Addr::unchecked("alice");
        let b = Addr::unchecked("bob");
        let id_worse = insert_bid(
            storage,
            Decimal::percent(50),
            Uint128::new(100),
            a.clone(),
            None,
            32,
        )
        .unwrap();
        let id_better =
            insert_bid(storage, Decimal::one(), Uint128::new(100), b, None, 32).unwrap();
        assert_ne!(id_worse, id_better);
        let head = query_head(storage, LimitOrderSide::Bid).unwrap().unwrap();
        assert_eq!(head, id_better);
        let lo = load_order_response(storage, head).unwrap();
        assert_eq!(lo.price, Decimal::one());
    }

    #[test]
    fn ask_head_is_best_price_then_fifo() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let a = Addr::unchecked("alice");
        let b = Addr::unchecked("bob");
        let _id_worse = insert_ask(
            storage,
            Decimal::from_atomics(2u128, 0).unwrap(),
            Uint128::new(100),
            a.clone(),
            None,
            32,
        )
        .unwrap();
        let id_better =
            insert_ask(storage, Decimal::one(), Uint128::new(100), b, None, 32).unwrap();
        let head = query_head(storage, LimitOrderSide::Ask).unwrap().unwrap();
        assert_eq!(head, id_better);
        let lo = load_order_response(storage, head).unwrap();
        assert_eq!(lo.price, Decimal::one());
    }

    #[test]
    fn unlink_order_repairs_doubly_linked_list() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50),
            o.clone(),
            None,
            32,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_atomics(99u128, 2).unwrap(),
            Uint128::new(50),
            o.clone(),
            None,
            32,
        )
        .unwrap();
        assert_eq!(
            query_head(storage, LimitOrderSide::Bid).unwrap().unwrap(),
            id1
        );
        unlink_order(storage, id1).unwrap();
        assert_eq!(
            query_head(storage, LimitOrderSide::Bid).unwrap().unwrap(),
            id2
        );
        let lo2 = load_order_response(storage, id2).unwrap();
        assert!(lo2.prev.is_none());
    }
}
