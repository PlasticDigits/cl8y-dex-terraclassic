//! FIFO doubly-linked limit orders per pair side (bids / asks).
//!
//! Hybrid match walks validate `book_start_hint` and each stepped order for side (GitLab #272 / L17).
//!
//! ## Ordering (composite key, strict total order)
//!
//! Price is always **token1 per token0** (same basis as pool pricing).
//!
//! - **Bids** (makers escrow token1; matched on taker **token0 → token1**):
//!   Walk best-first: **descending** `price`, then **ascending** `order_id` (FIFO at same price).
//! - **Asks** (makers escrow token0; matched on taker **token1 → token0**):
//!   Walk best-first: **ascending** `price`, then **ascending** `order_id` (FIFO at same price).
//!
//! ## Zero-cost fill skip (GitLab #470 / L18)
//!
//! `floor(fill × price)` can be **0** while `fill > 0` when price &lt; 1 (common across mismatched
//! token decimals). Such a fill would debit maker escrow without crediting the counter leg — skip
//! the order (`continue`) in `match_bids` / `match_asks` and `simulate_match_*` (symmetric on both sides).
//!
//! ## Blacklisted maker skip (GitLab #468 / L19)
//!
//! During execute walks, resting orders whose `owner` is on the factory trading blacklist are
//! never filled; when park budget allows, they are unlinked into `EXPIRED_LIMIT_CLAIMS` (same
//! escrow path as expiry — maker claims after `UnblacklistWallet`). Read-only sim walks skip them
//! without mutating storage.
//!
//! ## Match-time price math overflow skip (GitLab #467 / L20)
//!
//! Dust or extreme resting prices can make `checked_mul_floor` overflow on `1/price` or `fill × price`.
//! Placement rejects out-of-band prices; legacy rows skip instead of aborting the whole swap.

use std::collections::BTreeMap;

use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, Event, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use dex_common::pair::{LimitOrderResponse, LimitOrderSide};

use crate::blacklist_guard::TradeBlacklistGate;
use crate::error::ContractError;
use crate::state::{
    ExpiredLimitRefund, LimitOrder, EXPIRED_LIMIT_CLAIMS, HEAD_ASK, HEAD_BID, ORDERS,
    ORDER_NEXT_ID, PENDING_ESCROW_TOKEN0, PENDING_ESCROW_TOKEN1,
};

use dex_common::pair::{
    validate_limit_order_price, LIMIT_ORDER_DUST_FLUSH_THRESHOLD, MAX_ADJUST_STEPS_HARD_CAP,
    MAX_EXPIRED_PARKS_PER_SWAP, MAX_MAKER_FILLS_HARD_CAP, MAX_SCAN_STEPS,
};

fn try_price_inverse(price: Decimal) -> Option<Decimal> {
    Decimal::one().checked_div(price).ok()
}

fn try_mul_floor(amount: Uint128, factor: Decimal) -> Option<Uint128> {
    amount.checked_mul_floor(factor).ok()
}

/// True when post-fill `remaining` is sub-threshold dust (GitLab #264).
#[inline]
pub fn should_flush_dust(remaining: Uint128) -> bool {
    !remaining.is_zero() && remaining < LIMIT_ORDER_DUST_FLUSH_THRESHOLD
}

enum PostFillOutcome {
    Unlinked,
    FlushedDust { event: Event },
    PartialSaved,
}

/// Unlink, park dust, or save partial after a successful fill.
fn finalize_order_after_fill(
    storage: &mut dyn Storage,
    oid: u64,
    order: &LimitOrder,
    pair_contract: &str,
) -> Result<PostFillOutcome, ContractError> {
    if order.remaining.is_zero() {
        unlink_order(storage, oid)?;
        return Ok(PostFillOutcome::Unlinked);
    }
    if should_flush_dust(order.remaining) {
        ORDERS.save(storage, oid, order)?;
        let event = park_limit_order_for_clean(storage, oid, pair_contract, true, None)?;
        return Ok(PostFillOutcome::FlushedDust { event });
    }
    ORDERS.save(storage, oid, order)?;
    Ok(PostFillOutcome::PartialSaved)
}

#[inline]
fn apply_sim_dust_flush(order: &mut LimitOrder) {
    if should_flush_dust(order.remaining) {
        order.remaining = Uint128::zero();
    }
}

/// Increment scan step count; return `false` when [`MAX_SCAN_STEPS`] is reached (caller should break).
#[inline]
fn book_walk_step(scan_steps: &mut u32) -> bool {
    if *scan_steps >= MAX_SCAN_STEPS {
        return false;
    }
    *scan_steps += 1;
    true
}

/// Resolve `book_start_hint` for a match walk: accept only when the order exists on `expected_side`;
/// otherwise fall back to the book head (GitLab #272).
fn resolve_match_start_hint(
    storage: &dyn Storage,
    book_start_hint: Option<u64>,
    expected_side: LimitOrderSide,
) -> StdResult<Option<u64>> {
    if let Some(h) = book_start_hint {
        if let Some(order) = ORDERS.may_load(storage, h)? {
            if order.side == expected_side {
                return Ok(Some(h));
            }
        }
    }
    match expected_side {
        LimitOrderSide::Bid => Ok(HEAD_BID.may_load(storage)?.flatten()),
        LimitOrderSide::Ask => Ok(HEAD_ASK.may_load(storage)?.flatten()),
    }
}

/// Defense in depth during match walks: skip orders on the wrong side (GitLab #272).
#[inline]
fn order_on_match_side(order: &LimitOrder, expected_side: LimitOrderSide) -> bool {
    order.side == expected_side
}

/// Maker-side basis points charged once at order placement (`floor(effective/2)`).
#[inline]
pub fn maker_fee_bps(effective_fee_bps: u16) -> u16 {
    effective_fee_bps / 2
}

/// Taker-side basis points charged on each book fill (remainder so maker+taker = effective).
#[inline]
pub fn taker_fee_bps(effective_fee_bps: u16) -> u16 {
    effective_fee_bps - maker_fee_bps(effective_fee_bps)
}

/// Result of matching the limit book against a taker budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookMatchResult {
    /// Net ask-side output to the taker (after taker commission on each fill).
    pub return_net: Uint128,
    /// Offer-side amount consumed from the taker budget.
    pub offer_consumed: Uint128,
    /// Distinct maker orders filled (excluding expired parks).
    pub makers_used: u32,
    /// Expired orders parked into `EXPIRED_LIMIT_CLAIMS` during this walk.
    pub expired_parks: u32,
    /// Expired orders left on book because [`MAX_EXPIRED_PARKS_PER_SWAP`] was reached.
    pub expired_parks_skipped: u32,
    /// Book walk stopped because [`MAX_SCAN_STEPS`] was reached before `max_maker_fills` or list end.
    pub scan_steps_capped: bool,
    /// Sum of taker-side commission sent to treasury (denominated in the ask asset).
    pub commission_total: Uint128,
    /// Offer-token payouts per maker (deduped by owner; aggregated in `execute_swap`).
    pub maker_payouts: BTreeMap<Addr, Uint128>,
    pub fill_events: Vec<Event>,
}

/// CW20 transfers for aggregated maker payouts (offer token), one per distinct owner.
pub fn maker_payout_transfer_messages(
    maker_payouts: &BTreeMap<Addr, Uint128>,
    offer_token_addr: &str,
) -> Result<Vec<CosmosMsg>, StdError> {
    let mut msgs = Vec::with_capacity(maker_payouts.len());
    for (owner, amount) in maker_payouts {
        if amount.is_zero() {
            continue;
        }
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: offer_token_addr.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: *amount,
            })?,
            funds: vec![],
        }));
    }
    Ok(msgs)
}

/// Read-only book match for hybrid simulation (no storage mutation, no CW20 msgs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSimulateResult {
    pub return_net: Uint128,
    pub offer_consumed: Uint128,
    pub makers_used: u32,
    /// Same flag as [`BookMatchResult::scan_steps_capped`] for quote/execute parity.
    pub scan_steps_capped: bool,
    /// Taker commission total in the ask asset (same basis as [`BookMatchResult::commission_total`]).
    pub commission_total: Uint128,
}

/// Single load / checked subtract / save for bid-side escrow (token1).
fn escrow_sub_pending_token1(
    storage: &mut dyn Storage,
    amount: Uint128,
) -> Result<(), ContractError> {
    if amount.is_zero() {
        return Ok(());
    }
    let mut esc = PENDING_ESCROW_TOKEN1.may_load(storage)?.unwrap_or_default();
    esc = esc
        .checked_sub(amount)
        .map_err(|_| ContractError::InvariantViolation {
            reason: "pending escrow token1 underflow".into(),
        })?;
    PENDING_ESCROW_TOKEN1.save(storage, &esc)?;
    Ok(())
}

/// Single load / checked subtract / save for ask-side escrow (token0).
fn escrow_sub_pending_token0(
    storage: &mut dyn Storage,
    amount: Uint128,
) -> Result<(), ContractError> {
    if amount.is_zero() {
        return Ok(());
    }
    let mut esc = PENDING_ESCROW_TOKEN0.may_load(storage)?.unwrap_or_default();
    esc = esc
        .checked_sub(amount)
        .map_err(|_| ContractError::InvariantViolation {
            reason: "pending escrow token0 underflow".into(),
        })?;
    PENDING_ESCROW_TOKEN0.save(storage, &esc)?;
    Ok(())
}

/// `true` if order `a` should be closer to the bid head than `b` (better bid first).
pub fn bid_before(a_price: Decimal, a_id: u64, b_price: Decimal, b_id: u64) -> bool {
    a_price > b_price || (a_price == b_price && a_id < b_id)
}

/// `true` if order `a` should be closer to the ask head than `b` (better ask first).
pub fn ask_before(a_price: Decimal, a_id: u64, b_price: Decimal, b_id: u64) -> bool {
    a_price < b_price || (a_price == b_price && a_id < b_id)
}

/// Doubly-linked insert position: `(prev, next)` neighbor order ids.
type InsertNeighbors = (Option<u64>, Option<u64>);

/// Resolved insert position plus optional next-neighbor price when it was loaded (GitLab #266).
struct InsertFindResult {
    neighbors: InsertNeighbors,
    next_price: Option<Decimal>,
}

/// Thread cursor from the previous batch insert — O(0) verify for the next rung in book order (#266).
#[derive(Clone, Copy, Debug)]
pub struct InsertThreadCursor {
    pub after_id: u64,
    pub after_price: Decimal,
    pub next_id: Option<u64>,
    pub next_price: Option<Decimal>,
}

/// Outcome of a batch-path insert including cursor for the next traversal step (#266).
pub struct BatchInsertOutcome {
    pub id: u64,
    pub thread_cursor: InsertThreadCursor,
}

fn insert_find_result(
    prev: Option<u64>,
    next: Option<u64>,
    next_price: Option<Decimal>,
) -> InsertFindResult {
    InsertFindResult {
        neighbors: (prev, next),
        next_price: if next.is_some() { next_price } else { None },
    }
}

fn next_thread_cursor(id: u64, price: Decimal, found: &InsertFindResult) -> InsertThreadCursor {
    InsertThreadCursor {
        after_id: id,
        after_price: price,
        next_id: found.neighbors.1,
        next_price: found.next_price,
    }
}

fn next_order_id(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    reserve_order_id_block(storage, 1)
}

/// Reserve `count` consecutive order ids with a single `ORDER_NEXT_ID` write (GitLab #247).
/// Returns the first id in the block; callers consume ids sequentially (including on
/// `LimitInsertStepsExceeded` — same gap semantics as per-rung `next_order_id`).
pub fn reserve_order_id_block(storage: &mut dyn Storage, count: u32) -> Result<u64, ContractError> {
    if count == 0 {
        return Err(ContractError::InvariantViolation {
            reason: "cannot reserve zero order ids".into(),
        });
    }
    let start = ORDER_NEXT_ID.may_load(storage)?.unwrap_or(1u64);
    let end = start
        .checked_add(count as u64)
        .ok_or_else(|| ContractError::InvariantViolation {
            reason: "order id overflow".into(),
        })?;
    ORDER_NEXT_ID.save(storage, &end)?;
    Ok(start)
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
    expires_at: Option<u64>,
) -> Result<u64, ContractError> {
    let id = next_order_id(storage)?;
    insert_bid_with_id(
        storage,
        id,
        price,
        remaining_token1,
        owner,
        hint_after,
        max_adjust_steps,
        expires_at,
        true,
    )
}

/// Batch path: caller supplies a pre-reserved id; optionally skip per-insert escrow write.
#[allow(clippy::too_many_arguments)]
pub fn insert_bid_with_id(
    storage: &mut dyn Storage,
    id: u64,
    price: Decimal,
    remaining_token1: Uint128,
    owner: Addr,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
    update_escrow: bool,
) -> Result<u64, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;

    let head = HEAD_BID.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_bid(
            storage, head, None, hint_after, price, id, max_steps, &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token1,
        side: LimitOrderSide::Bid,
        expires_at,
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

    if update_escrow {
        let mut esc = PENDING_ESCROW_TOKEN1
            .may_load(storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_add(remaining_token1)
            .map_err(ContractError::Overflow)?;
        PENDING_ESCROW_TOKEN1.save(storage, &esc)?;
    }

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
    expires_at: Option<u64>,
) -> Result<u64, ContractError> {
    let id = next_order_id(storage)?;
    insert_ask_with_id(
        storage,
        id,
        price,
        remaining_token0,
        owner,
        hint_after,
        max_adjust_steps,
        expires_at,
        true,
    )
}

/// Batch path: caller supplies a pre-reserved id; optionally skip per-insert escrow write.
#[allow(clippy::too_many_arguments)]
pub fn insert_ask_with_id(
    storage: &mut dyn Storage,
    id: u64,
    price: Decimal,
    remaining_token0: Uint128,
    owner: Addr,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
    update_escrow: bool,
) -> Result<u64, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;

    let head = HEAD_ASK.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_ask(
            storage, head, None, hint_after, price, id, max_steps, &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token0,
        side: LimitOrderSide::Ask,
        expires_at,
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

    if update_escrow {
        let mut esc = PENDING_ESCROW_TOKEN0
            .may_load(storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_add(remaining_token0)
            .map_err(ContractError::Overflow)?;
        PENDING_ESCROW_TOKEN0.save(storage, &esc)?;
    }

    Ok(id)
}

/// Batch path: thread cursor + hint; returns cursor for the next book-order traversal step (#266).
#[allow(clippy::too_many_arguments)]
pub fn insert_bid_with_id_for_batch(
    storage: &mut dyn Storage,
    id: u64,
    price: Decimal,
    remaining_token1: Uint128,
    owner: Addr,
    thread_cursor: Option<InsertThreadCursor>,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
) -> Result<BatchInsertOutcome, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;

    let head = HEAD_BID.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_bid(
            storage,
            head,
            thread_cursor,
            hint_after,
            price,
            id,
            max_steps,
            &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token1,
        side: LimitOrderSide::Bid,
        expires_at,
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

    Ok(BatchInsertOutcome {
        id,
        thread_cursor: next_thread_cursor(id, price, &found),
    })
}

/// Batch path: thread cursor + hint; returns cursor for the next book-order traversal step (#266).
#[allow(clippy::too_many_arguments)]
pub fn insert_ask_with_id_for_batch(
    storage: &mut dyn Storage,
    id: u64,
    price: Decimal,
    remaining_token0: Uint128,
    owner: Addr,
    thread_cursor: Option<InsertThreadCursor>,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
    expires_at: Option<u64>,
) -> Result<BatchInsertOutcome, ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;

    let head = HEAD_ASK.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_ask(
            storage,
            head,
            thread_cursor,
            hint_after,
            price,
            id,
            max_steps,
            &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;

    let order = LimitOrder {
        owner: owner.clone(),
        price,
        remaining: remaining_token0,
        side: LimitOrderSide::Ask,
        expires_at,
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

    Ok(BatchInsertOutcome {
        id,
        thread_cursor: next_thread_cursor(id, price, &found),
    })
}

/// O(0)-load verify when batch traversal inserts the next rung immediately after `cursor` (#266).
fn try_insert_after_thread_cursor_bid(
    cursor: InsertThreadCursor,
    new_price: Decimal,
    new_id: u64,
) -> Option<InsertFindResult> {
    if !bid_before(cursor.after_price, cursor.after_id, new_price, new_id) {
        return None;
    }
    if let Some(next_id) = cursor.next_id {
        let np = cursor.next_price?;
        if !bid_before(new_price, new_id, np, next_id) {
            return None;
        }
    }
    Some(insert_find_result(
        Some(cursor.after_id),
        cursor.next_id,
        cursor.next_price,
    ))
}

fn try_insert_after_thread_cursor_ask(
    cursor: InsertThreadCursor,
    new_price: Decimal,
    new_id: u64,
) -> Option<InsertFindResult> {
    if !ask_before(cursor.after_price, cursor.after_id, new_price, new_id) {
        return None;
    }
    if let Some(next_id) = cursor.next_id {
        let np = cursor.next_price?;
        if !ask_before(new_price, new_id, np, next_id) {
            return None;
        }
    }
    Some(insert_find_result(
        Some(cursor.after_id),
        cursor.next_id,
        cursor.next_price,
    ))
}

/// Outcome of O(1) hint verification (GitLab #256) and directional fallback routing (#265).
enum HintInsertOutcome {
    /// Verified predecessor — insert in O(1).
    Ready {
        prev: Option<u64>,
        next: Option<u64>,
        next_price: Option<Decimal>,
    },
    /// Hint missing, wrong side, or dangling — fall back to head walk.
    HeadWalk,
    /// Valid anchor; new sorts before hint — walk `prev` toward head.
    WalkTowardHead { hint_id: u64 },
    /// Valid anchor; new sorts after hint's `next` — walk `next` toward tail from `start_id`.
    WalkTowardTail { start_id: u64 },
}

/// O(1) insert position when `hint_id` is a valid on-book predecessor (GitLab #256).
/// On verify failure with a valid anchor, returns a directional walk target (GitLab #265).
fn try_insert_after_hint_bid(
    storage: &dyn Storage,
    head: Option<u64>,
    hint_id: u64,
    new_price: Decimal,
    new_id: u64,
    steps: &mut u32,
) -> Result<HintInsertOutcome, ContractError> {
    *steps += 1;
    let hint = match ORDERS.may_load(storage, hint_id)? {
        Some(o) if o.side == LimitOrderSide::Bid => o,
        _ => return Ok(HintInsertOutcome::HeadWalk),
    };
    if hint.prev.is_none() && head != Some(hint_id) {
        return Ok(HintInsertOutcome::HeadWalk);
    }
    if !bid_before(hint.price, hint_id, new_price, new_id) {
        return Ok(HintInsertOutcome::WalkTowardHead { hint_id });
    }
    if let Some(next_id) = hint.next {
        *steps += 1;
        let next = match ORDERS.may_load(storage, next_id)? {
            Some(o) if o.side == LimitOrderSide::Bid => o,
            _ => {
                return Ok(HintInsertOutcome::WalkTowardTail { start_id: hint_id });
            }
        };
        if !bid_before(new_price, new_id, next.price, next_id) {
            return Ok(HintInsertOutcome::WalkTowardTail { start_id: next_id });
        }
        Ok(HintInsertOutcome::Ready {
            prev: Some(hint_id),
            next: Some(next_id),
            next_price: Some(next.price),
        })
    } else {
        Ok(HintInsertOutcome::Ready {
            prev: Some(hint_id),
            next: None,
            next_price: None,
        })
    }
}

fn try_insert_after_hint_ask(
    storage: &dyn Storage,
    head: Option<u64>,
    hint_id: u64,
    new_price: Decimal,
    new_id: u64,
    steps: &mut u32,
) -> Result<HintInsertOutcome, ContractError> {
    *steps += 1;
    let hint = match ORDERS.may_load(storage, hint_id)? {
        Some(o) if o.side == LimitOrderSide::Ask => o,
        _ => return Ok(HintInsertOutcome::HeadWalk),
    };
    if hint.prev.is_none() && head != Some(hint_id) {
        return Ok(HintInsertOutcome::HeadWalk);
    }
    if !ask_before(hint.price, hint_id, new_price, new_id) {
        return Ok(HintInsertOutcome::WalkTowardHead { hint_id });
    }
    if let Some(next_id) = hint.next {
        *steps += 1;
        let next = match ORDERS.may_load(storage, next_id)? {
            Some(o) if o.side == LimitOrderSide::Ask => o,
            _ => {
                return Ok(HintInsertOutcome::WalkTowardTail { start_id: hint_id });
            }
        };
        if !ask_before(new_price, new_id, next.price, next_id) {
            return Ok(HintInsertOutcome::WalkTowardTail { start_id: next_id });
        }
        Ok(HintInsertOutcome::Ready {
            prev: Some(hint_id),
            next: Some(next_id),
            next_price: Some(next.price),
        })
    } else {
        Ok(HintInsertOutcome::Ready {
            prev: Some(hint_id),
            next: None,
            next_price: None,
        })
    }
}

/// Forward book walk from `start_id` (same comparator loop as head walk).
fn walk_insert_bid_from(
    storage: &dyn Storage,
    start_id: u64,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut prev: Option<u64> = None;
    let mut cur = Some(start_id);

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if bid_before(new_price, new_id, ord.price, cid) {
            return Ok(insert_find_result(ord.prev, Some(cid), Some(ord.price)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok(insert_find_result(prev, None, None))
}

fn walk_insert_ask_from(
    storage: &dyn Storage,
    start_id: u64,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut prev: Option<u64> = None;
    let mut cur = Some(start_id);

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if ask_before(new_price, new_id, ord.price, cid) {
            return Ok(insert_find_result(ord.prev, Some(cid), Some(ord.price)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok(insert_find_result(prev, None, None))
}

/// Walk `prev` from `hint_id` toward head; new sorts before hint (GitLab #265).
/// At each node, verify the candidate `(prev, cur)` pair against price-time order before stopping.
fn walk_insert_bid_toward_head(
    storage: &dyn Storage,
    hint_id: u64,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut cur = Some(hint_id);

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if bid_before(new_price, new_id, ord.price, cid) {
            if let Some(prev_id) = ord.prev {
                let prev_ord = ORDERS.load(storage, prev_id)?;
                if bid_before(prev_ord.price, prev_id, new_price, new_id) {
                    return Ok(insert_find_result(
                        Some(prev_id),
                        Some(cid),
                        Some(ord.price),
                    ));
                }
                cur = Some(prev_id);
                continue;
            }
            return Ok(insert_find_result(None, Some(cid), Some(ord.price)));
        }
        cur = ord.prev;
    }
    Ok(insert_find_result(None, Some(hint_id), None))
}

fn walk_insert_ask_toward_head(
    storage: &dyn Storage,
    hint_id: u64,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut cur = Some(hint_id);

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if ask_before(new_price, new_id, ord.price, cid) {
            if let Some(prev_id) = ord.prev {
                let prev_ord = ORDERS.load(storage, prev_id)?;
                if ask_before(prev_ord.price, prev_id, new_price, new_id) {
                    return Ok(insert_find_result(
                        Some(prev_id),
                        Some(cid),
                        Some(ord.price),
                    ));
                }
                cur = Some(prev_id);
                continue;
            }
            return Ok(insert_find_result(None, Some(cid), Some(ord.price)));
        }
        cur = ord.prev;
    }
    Ok(insert_find_result(None, Some(hint_id), None))
}

/// Head walk — used when hint anchor is unusable (missing / wrong side / unlinked).
fn walk_insert_bid_from_head(
    storage: &dyn Storage,
    head: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut prev: Option<u64> = None;
    let mut cur = head;

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if bid_before(new_price, new_id, ord.price, cid) {
            return Ok(insert_find_result(ord.prev, Some(cid), Some(ord.price)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok(insert_find_result(prev, None, None))
}

fn walk_insert_ask_from_head(
    storage: &dyn Storage,
    head: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    let mut prev: Option<u64> = None;
    let mut cur = head;

    while let Some(cid) = cur {
        *steps += 1;
        if *steps > max_steps {
            return Err(ContractError::LimitInsertStepsExceeded { max: max_steps });
        }
        let ord = ORDERS.load(storage, cid)?;
        if ask_before(new_price, new_id, ord.price, cid) {
            return Ok(insert_find_result(ord.prev, Some(cid), Some(ord.price)));
        }
        prev = Some(cid);
        cur = ord.next;
    }
    Ok(insert_find_result(prev, None, None))
}

#[allow(clippy::too_many_arguments)]
fn find_insert_bid(
    storage: &dyn Storage,
    head: Option<u64>,
    thread_cursor: Option<InsertThreadCursor>,
    hint_after: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    if let Some(cursor) = thread_cursor {
        if let Some(found) = try_insert_after_thread_cursor_bid(cursor, new_price, new_id) {
            return Ok(found);
        }
    }
    if let Some(hint_id) = hint_after {
        match try_insert_after_hint_bid(storage, head, hint_id, new_price, new_id, steps)? {
            HintInsertOutcome::Ready {
                prev,
                next,
                next_price,
            } => return Ok(insert_find_result(prev, next, next_price)),
            HintInsertOutcome::WalkTowardHead { hint_id } => {
                return walk_insert_bid_toward_head(
                    storage, hint_id, new_price, new_id, max_steps, steps,
                );
            }
            HintInsertOutcome::WalkTowardTail { start_id } => {
                return walk_insert_bid_from(
                    storage, start_id, new_price, new_id, max_steps, steps,
                );
            }
            HintInsertOutcome::HeadWalk => {}
        }
    }
    walk_insert_bid_from_head(storage, head, new_price, new_id, max_steps, steps)
}

#[allow(clippy::too_many_arguments)]
fn find_insert_ask(
    storage: &dyn Storage,
    head: Option<u64>,
    thread_cursor: Option<InsertThreadCursor>,
    hint_after: Option<u64>,
    new_price: Decimal,
    new_id: u64,
    max_steps: u32,
    steps: &mut u32,
) -> Result<InsertFindResult, ContractError> {
    if let Some(cursor) = thread_cursor {
        if let Some(found) = try_insert_after_thread_cursor_ask(cursor, new_price, new_id) {
            return Ok(found);
        }
    }
    if let Some(hint_id) = hint_after {
        match try_insert_after_hint_ask(storage, head, hint_id, new_price, new_id, steps)? {
            HintInsertOutcome::Ready {
                prev,
                next,
                next_price,
            } => return Ok(insert_find_result(prev, next, next_price)),
            HintInsertOutcome::WalkTowardHead { hint_id } => {
                return walk_insert_ask_toward_head(
                    storage, hint_id, new_price, new_id, max_steps, steps,
                );
            }
            HintInsertOutcome::WalkTowardTail { start_id } => {
                return walk_insert_ask_from(
                    storage, start_id, new_price, new_id, max_steps, steps,
                );
            }
            HintInsertOutcome::HeadWalk => {}
        }
    }
    walk_insert_ask_from_head(storage, head, new_price, new_id, max_steps, steps)
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

/// Remove an order from the price-time list but keep it in `ORDERS` with `prev`/`next` cleared.
pub fn detach_limit_order_from_book(
    storage: &mut dyn Storage,
    id: u64,
) -> Result<LimitOrder, ContractError> {
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
        ORDERS
            .update(storage, p, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("prev"))?;
                x.next = order.next;
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    }
    if let Some(n) = order.next {
        ORDERS
            .update(storage, n, |o| -> StdResult<LimitOrder> {
                let mut x = o.ok_or_else(|| StdError::generic_err("next"))?;
                x.prev = order.prev;
                Ok(x)
            })
            .map_err(ContractError::Std)?;
    }

    let detached = LimitOrder {
        prev: None,
        next: None,
        ..order
    };
    ORDERS.save(storage, id, &detached)?;
    Ok(detached)
}

fn link_bid_order_at_id(
    storage: &mut dyn Storage,
    id: u64,
    mut order: LimitOrder,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
) -> Result<(), ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;
    let head = HEAD_BID.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_bid(
            storage,
            head,
            None,
            hint_after,
            order.price,
            id,
            max_steps,
            &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;
    order.prev = prev;
    order.next = next;
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
    Ok(())
}

fn link_ask_order_at_id(
    storage: &mut dyn Storage,
    id: u64,
    mut order: LimitOrder,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
) -> Result<(), ContractError> {
    let max_steps = max_adjust_steps.min(MAX_ADJUST_STEPS_HARD_CAP);
    let mut steps: u32 = 0;
    let head = HEAD_ASK.may_load(storage)?.flatten();
    let found = if head.is_none() {
        insert_find_result(None, None, None)
    } else {
        find_insert_ask(
            storage,
            head,
            None,
            hint_after,
            order.price,
            id,
            max_steps,
            &mut steps,
        )?
    };
    let (prev, next) = found.neighbors;
    order.prev = prev;
    order.next = next;
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
    Ok(())
}

/// Re-sort an existing order at `new_price` without changing its id or escrow.
pub fn relink_limit_order_price(
    storage: &mut dyn Storage,
    id: u64,
    new_price: Decimal,
    hint_after: Option<u64>,
    max_adjust_steps: u32,
) -> Result<(), ContractError> {
    validate_limit_order_price(new_price).map_err(|reason| ContractError::InvalidHybridParams {
        reason: reason.into(),
    })?;
    let mut order = detach_limit_order_from_book(storage, id)?;
    order.price = new_price;
    match order.side {
        LimitOrderSide::Bid => {
            link_bid_order_at_id(storage, id, order, hint_after, max_adjust_steps)
        }
        LimitOrderSide::Ask => {
            link_ask_order_at_id(storage, id, order, hint_after, max_adjust_steps)
        }
    }
}

pub fn side_str(side: &LimitOrderSide) -> &'static str {
    match side {
        LimitOrderSide::Bid => "bid",
        LimitOrderSide::Ask => "ask",
    }
}

/// One wasm event per maker fill for indexers (action: `limit_order_fill`).
///
/// `token0_amount` / `token1_amount` are raw amounts in the pair's token0 / token1 assets.
/// For bids: taker pays `token0_amount` token0; `token1_amount` is token1 taken from bid escrow
/// (`net_to_taker` + `commission_amount` to treasury).
/// `commission_amount` is the **taker** half of the pair fee for this fill (token1); the maker
/// half was charged at order placement.
#[allow(clippy::too_many_arguments)]
fn limit_order_fill_event(
    pair_contract: &str,
    order_id: u64,
    side: LimitOrderSide,
    maker: &Addr,
    price: Decimal,
    token0_amount: Uint128,
    token1_amount: Uint128,
    commission_amount: Uint128,
) -> Event {
    Event::new("wasm")
        .add_attribute("contract_address", pair_contract)
        .add_attribute("action", "limit_order_fill")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("side", side_str(&side))
        .add_attribute("maker", maker.as_str())
        .add_attribute("price", price.to_string())
        .add_attribute("token0_amount", token0_amount)
        .add_attribute("token1_amount", token1_amount)
        .add_attribute("commission_amount", commission_amount)
}

pub(crate) fn limit_order_expired_parked_event(
    pair_contract: &str,
    order_id: u64,
    maker: &Addr,
    side: LimitOrderSide,
    remaining: Uint128,
    force_expired: bool,
) -> Event {
    let mut ev = Event::new("wasm")
        .add_attribute("contract_address", pair_contract)
        .add_attribute("action", "limit_order_expired_parked")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("maker", maker.as_str())
        .add_attribute("side", side_str(&side))
        .add_attribute("remaining", remaining);
    if force_expired {
        ev = ev.add_attribute("force_expired", "true");
    }
    ev
}

/// Park an order into `EXPIRED_LIMIT_CLAIMS` without moving CW20 (GitLab #263 / #120).
pub fn park_limit_order_for_clean(
    storage: &mut dyn Storage,
    order_id: u64,
    pair_contract: &str,
    force_expired: bool,
    refund_expires_at: Option<u64>,
) -> Result<Event, ContractError> {
    if EXPIRED_LIMIT_CLAIMS.may_load(storage, order_id)?.is_some() {
        return Err(ContractError::InvariantViolation {
            reason: "expired limit claim already exists for order id".into(),
        });
    }
    let removed = unlink_order(storage, order_id)?;
    let row = ExpiredLimitRefund {
        owner: removed.owner.clone(),
        side: removed.side.clone(),
        remaining: removed.remaining,
        expires_at: refund_expires_at,
    };
    EXPIRED_LIMIT_CLAIMS.save(storage, order_id, &row)?;
    Ok(limit_order_expired_parked_event(
        pair_contract,
        order_id,
        &removed.owner,
        removed.side,
        removed.remaining,
        force_expired,
    ))
}

/// Remove a time-expired order from the book during a hybrid match walk.
pub fn park_expired_limit_order_for_claim(
    storage: &mut dyn Storage,
    order_id: u64,
    now: u64,
    pair_contract: &str,
) -> Result<Event, ContractError> {
    let order = ORDERS.load(storage, order_id)?;
    // `is_none_or` needs Rust 1.82+; `workspace-optimizer:0.16.1` uses 1.81.
    #[allow(clippy::unnecessary_map_or)]
    if order.expires_at.map_or(true, |e| now < e) {
        return Err(ContractError::InvariantViolation {
            reason: "park_expired_limit_order_for_claim on non-expired order".into(),
        });
    }
    park_limit_order_for_clean(storage, order_id, pair_contract, false, order.expires_at)
}

/// Skip (and optionally park) a resting order when its owner is trading-blacklisted (GitLab #468).
fn maker_owner_is_trade_blacklisted(
    blacklist_gate: Option<&TradeBlacklistGate>,
    owner: &Addr,
) -> Result<bool, ContractError> {
    match blacklist_gate {
        Some(gate) => gate.is_wallet_blacklisted(owner),
        None => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
fn skip_blacklisted_maker_order(
    storage: &mut dyn Storage,
    oid: u64,
    owner: &Addr,
    pair_contract: &str,
    blacklist_gate: Option<&TradeBlacklistGate>,
    park_off_book: bool,
    expired_parks: &mut u32,
    expired_parks_skipped: &mut u32,
    fill_events: &mut Vec<Event>,
) -> Result<bool, ContractError> {
    if !maker_owner_is_trade_blacklisted(blacklist_gate, owner)? {
        return Ok(false);
    }
    if park_off_book {
        if *expired_parks < MAX_EXPIRED_PARKS_PER_SWAP {
            let ev = park_limit_order_for_clean(storage, oid, pair_contract, true, None)?;
            fill_events.push(ev);
            *expired_parks += 1;
        } else {
            *expired_parks_skipped += 1;
        }
    }
    Ok(true)
}

/// Match bids while taker sells token0 for token1. `token0_budget` is filled from the taker.
#[allow(clippy::too_many_arguments)]
pub fn match_bids(
    storage: &mut dyn Storage,
    now: u64,
    token0_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    pair_contract: &str,
    _token0_addr: &str,
    _token1_addr: &str,
    _receiver: &Addr,
    _treasury: &Addr,
    effective_fee_bps: u16,
    blacklist_gate: Option<&TradeBlacklistGate>,
) -> Result<BookMatchResult, ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token0_left = token0_budget;
    let mut token1_out_total = Uint128::zero();
    let mut commission_total = Uint128::zero();
    let mut makers_used = 0u32;
    let mut expired_parks = 0u32;
    let mut expired_parks_skipped = 0u32;
    let mut scan_steps = 0u32;
    let mut scan_steps_capped = false;
    let mut maker_payouts: BTreeMap<Addr, Uint128> = BTreeMap::new();
    let mut fill_events: Vec<Event> = Vec::new();
    let mut token1_escrow_sub_total = Uint128::zero();

    let mut cur = resolve_match_start_hint(storage, book_start_hint, LimitOrderSide::Bid)?;

    while makers_used < cap {
        if !book_walk_step(&mut scan_steps) {
            scan_steps_capped = true;
            break;
        }
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let order = ORDERS.load(storage, oid)?;
        let next_ptr = order.next;

        if !order_on_match_side(&order, LimitOrderSide::Bid) {
            cur = next_ptr;
            continue;
        }

        if order.expires_at.is_some_and(|e| now >= e) {
            if expired_parks < MAX_EXPIRED_PARKS_PER_SWAP {
                let ev = park_expired_limit_order_for_claim(storage, oid, now, pair_contract)?;
                fill_events.push(ev);
                expired_parks += 1;
            } else {
                expired_parks_skipped += 1;
            }
            cur = next_ptr;
            continue;
        }

        if skip_blacklisted_maker_order(
            storage,
            oid,
            &order.owner,
            pair_contract,
            blacklist_gate,
            true,
            &mut expired_parks,
            &mut expired_parks_skipped,
            &mut fill_events,
        )? {
            cur = next_ptr;
            continue;
        }

        if order.remaining.is_zero() {
            cur = next_ptr;
            continue;
        }

        if token0_left.is_zero() {
            break;
        }

        let mut order = order;

        if order.price.is_zero() {
            return Err(ContractError::InvariantViolation {
                reason: "zero bid price".into(),
            });
        }
        let Some(inv) = try_price_inverse(order.price) else {
            cur = next_ptr;
            continue;
        };
        // Max token0 purchasable with this bid's remaining token1 budget at price (token1 per token0).
        let Some(max_fill_from_bid) = try_mul_floor(order.remaining, inv) else {
            cur = next_ptr;
            continue;
        };
        let mut fill = token0_left.min(max_fill_from_bid);
        if fill.is_zero() {
            cur = order.next;
            continue;
        }

        let Some(mut cost) = try_mul_floor(fill, order.price) else {
            cur = next_ptr;
            continue;
        };
        let mut skip_order = false;
        while fill > Uint128::zero() && cost > order.remaining {
            fill = fill.saturating_sub(Uint128::one());
            if fill.is_zero() {
                break;
            }
            match try_mul_floor(fill, order.price) {
                Some(adjusted) => cost = adjusted,
                None => {
                    skip_order = true;
                    break;
                }
            }
        }
        if skip_order {
            cur = next_ptr;
            continue;
        }

        if fill.is_zero() {
            cur = order.next;
            continue;
        }
        if cost.is_zero() {
            cur = order.next;
            continue;
        }

        makers_used += 1;

        let commission = cost
            .checked_mul(Uint128::new(taker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = cost.checked_sub(commission)?;
        commission_total = commission_total.checked_add(commission)?;

        let entry = maker_payouts.entry(order.owner.clone()).or_default();
        *entry = entry.checked_add(fill)?;

        token1_escrow_sub_total = token1_escrow_sub_total.checked_add(cost)?;

        fill_events.push(limit_order_fill_event(
            pair_contract,
            oid,
            LimitOrderSide::Bid,
            &order.owner,
            order.price,
            fill,
            cost,
            commission,
        ));

        order.remaining = order.remaining.checked_sub(cost)?;
        token0_left = token0_left.checked_sub(fill)?;
        token1_out_total = token1_out_total.checked_add(net_to_taker)?;

        match finalize_order_after_fill(storage, oid, &order, pair_contract)? {
            PostFillOutcome::Unlinked => {}
            PostFillOutcome::FlushedDust { event, .. } => {
                fill_events.push(event);
            }
            PostFillOutcome::PartialSaved => {}
        }

        cur = order.next;
    }

    escrow_sub_pending_token1(storage, token1_escrow_sub_total)?;

    Ok(BookMatchResult {
        return_net: token1_out_total,
        offer_consumed: token0_budget.checked_sub(token0_left)?,
        makers_used,
        expired_parks,
        expired_parks_skipped,
        scan_steps_capped,
        commission_total,
        maker_payouts,
        fill_events,
    })
}

/// Match asks while taker sells token1 for token0. `token1_budget` is from the taker.
#[allow(clippy::too_many_arguments)]
pub fn match_asks(
    storage: &mut dyn Storage,
    now: u64,
    token1_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    pair_contract: &str,
    _token0_addr: &str,
    _token1_addr: &str,
    _receiver: &Addr,
    _treasury: &Addr,
    effective_fee_bps: u16,
    blacklist_gate: Option<&TradeBlacklistGate>,
) -> Result<BookMatchResult, ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token1_left = token1_budget;
    let mut token0_out_total = Uint128::zero();
    let mut commission_total = Uint128::zero();
    let mut makers_used = 0u32;
    let mut expired_parks = 0u32;
    let mut expired_parks_skipped = 0u32;
    let mut scan_steps = 0u32;
    let mut scan_steps_capped = false;
    let mut maker_payouts: BTreeMap<Addr, Uint128> = BTreeMap::new();
    let mut fill_events: Vec<Event> = Vec::new();
    let mut token0_escrow_sub_total = Uint128::zero();

    let mut cur = resolve_match_start_hint(storage, book_start_hint, LimitOrderSide::Ask)?;

    while makers_used < cap {
        if !book_walk_step(&mut scan_steps) {
            scan_steps_capped = true;
            break;
        }
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let order = ORDERS.load(storage, oid)?;
        let next_ptr = order.next;

        if !order_on_match_side(&order, LimitOrderSide::Ask) {
            cur = next_ptr;
            continue;
        }

        if order.expires_at.is_some_and(|e| now >= e) {
            if expired_parks < MAX_EXPIRED_PARKS_PER_SWAP {
                let ev = park_expired_limit_order_for_claim(storage, oid, now, pair_contract)?;
                fill_events.push(ev);
                expired_parks += 1;
            } else {
                expired_parks_skipped += 1;
            }
            cur = next_ptr;
            continue;
        }

        if skip_blacklisted_maker_order(
            storage,
            oid,
            &order.owner,
            pair_contract,
            blacklist_gate,
            true,
            &mut expired_parks,
            &mut expired_parks_skipped,
            &mut fill_events,
        )? {
            cur = next_ptr;
            continue;
        }

        if order.remaining.is_zero() {
            cur = next_ptr;
            continue;
        }

        if token1_left.is_zero() {
            break;
        }

        let mut order = order;

        let max_fill_token0_from_ask = order.remaining;
        let max_fill_token0_from_budget = if !order.price.is_zero() {
            let Some(inv) = try_price_inverse(order.price) else {
                cur = next_ptr;
                continue;
            };
            let Some(budget_fill) = try_mul_floor(token1_left, inv) else {
                cur = next_ptr;
                continue;
            };
            budget_fill
        } else {
            Uint128::zero()
        };

        let mut fill_t0 = max_fill_token0_from_ask.min(max_fill_token0_from_budget);
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }

        let Some(mut cost) = try_mul_floor(fill_t0, order.price) else {
            cur = next_ptr;
            continue;
        };
        let mut skip_order = false;
        while fill_t0 > Uint128::zero() && cost > token1_left {
            fill_t0 = fill_t0.saturating_sub(Uint128::one());
            if fill_t0.is_zero() {
                break;
            }
            match try_mul_floor(fill_t0, order.price) {
                Some(adjusted) => cost = adjusted,
                None => {
                    skip_order = true;
                    break;
                }
            }
        }
        if skip_order {
            cur = next_ptr;
            continue;
        }
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }
        if cost.is_zero() {
            cur = order.next;
            continue;
        }

        makers_used += 1;

        // Fee on token0 output to taker (same denomination as net and treasury).
        let commission = fill_t0
            .checked_mul(Uint128::new(taker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = fill_t0.checked_sub(commission)?;
        commission_total = commission_total.checked_add(commission)?;

        let entry = maker_payouts.entry(order.owner.clone()).or_default();
        *entry = entry.checked_add(cost)?;

        token0_escrow_sub_total = token0_escrow_sub_total.checked_add(fill_t0)?;

        fill_events.push(limit_order_fill_event(
            pair_contract,
            oid,
            LimitOrderSide::Ask,
            &order.owner,
            order.price,
            fill_t0,
            cost,
            commission,
        ));

        order.remaining = order.remaining.checked_sub(fill_t0)?;
        token1_left = token1_left.checked_sub(cost)?;
        token0_out_total = token0_out_total.checked_add(net_to_taker)?;

        match finalize_order_after_fill(storage, oid, &order, pair_contract)? {
            PostFillOutcome::Unlinked => {}
            PostFillOutcome::FlushedDust { event, .. } => {
                fill_events.push(event);
            }
            PostFillOutcome::PartialSaved => {}
        }

        cur = order.next;
    }

    escrow_sub_pending_token0(storage, token0_escrow_sub_total)?;

    Ok(BookMatchResult {
        return_net: token0_out_total,
        offer_consumed: token1_budget.checked_sub(token1_left)?,
        makers_used,
        expired_parks,
        expired_parks_skipped,
        scan_steps_capped,
        commission_total,
        maker_payouts,
        fill_events,
    })
}

/// Read-only bid match for hybrid quotes.
pub fn simulate_match_bids(
    storage: &dyn Storage,
    now: u64,
    token0_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    effective_fee_bps: u16,
    blacklist_gate: Option<&TradeBlacklistGate>,
) -> Result<BookSimulateResult, ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token0_left = token0_budget;
    let mut token1_out_total = Uint128::zero();
    let mut commission_total = Uint128::zero();
    let mut makers_used: u32 = 0;
    let mut scan_steps = 0u32;
    let mut scan_steps_capped = false;
    let mut cur = resolve_match_start_hint(storage, book_start_hint, LimitOrderSide::Bid)?;
    while makers_used < cap {
        if !book_walk_step(&mut scan_steps) {
            scan_steps_capped = true;
            break;
        }
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let order = ORDERS.load(storage, oid)?;
        let next_ptr = order.next;
        if !order_on_match_side(&order, LimitOrderSide::Bid) {
            cur = next_ptr;
            continue;
        }
        if order.expires_at.is_some_and(|e| now >= e) {
            cur = next_ptr;
            continue;
        }
        if maker_owner_is_trade_blacklisted(blacklist_gate, &order.owner)? {
            cur = next_ptr;
            continue;
        }
        if order.remaining.is_zero() {
            cur = next_ptr;
            continue;
        }
        if token0_left.is_zero() {
            break;
        }
        let mut order = order;
        if order.price.is_zero() {
            return Err(ContractError::InvariantViolation {
                reason: "zero bid price".into(),
            });
        }
        let Some(inv) = try_price_inverse(order.price) else {
            cur = next_ptr;
            continue;
        };
        let Some(max_fill_from_bid) = try_mul_floor(order.remaining, inv) else {
            cur = next_ptr;
            continue;
        };
        let mut fill = token0_left.min(max_fill_from_bid);
        if fill.is_zero() {
            cur = order.next;
            continue;
        }
        let Some(mut cost) = try_mul_floor(fill, order.price) else {
            cur = next_ptr;
            continue;
        };
        let mut skip_order = false;
        while fill > Uint128::zero() && cost > order.remaining {
            fill = fill.saturating_sub(Uint128::one());
            if fill.is_zero() {
                break;
            }
            match try_mul_floor(fill, order.price) {
                Some(adjusted) => cost = adjusted,
                None => {
                    skip_order = true;
                    break;
                }
            }
        }
        if skip_order {
            cur = next_ptr;
            continue;
        }
        if fill.is_zero() {
            cur = order.next;
            continue;
        }
        if cost.is_zero() {
            cur = order.next;
            continue;
        }
        makers_used += 1;
        let commission = cost
            .checked_mul(Uint128::new(taker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = cost.checked_sub(commission)?;
        commission_total = commission_total.checked_add(commission)?;
        order.remaining = order.remaining.checked_sub(cost)?;
        apply_sim_dust_flush(&mut order);
        token0_left = token0_left.checked_sub(fill)?;
        token1_out_total = token1_out_total.checked_add(net_to_taker)?;
        cur = order.next;
    }
    Ok(BookSimulateResult {
        return_net: token1_out_total,
        offer_consumed: token0_budget.checked_sub(token0_left)?,
        makers_used,
        scan_steps_capped,
        commission_total,
    })
}

/// Read-only ask match for hybrid quotes.
pub fn simulate_match_asks(
    storage: &dyn Storage,
    now: u64,
    token1_budget: Uint128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    effective_fee_bps: u16,
    blacklist_gate: Option<&TradeBlacklistGate>,
) -> Result<BookSimulateResult, ContractError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token1_left = token1_budget;
    let mut token0_out_total = Uint128::zero();
    let mut commission_total = Uint128::zero();
    let mut makers_used: u32 = 0;
    let mut scan_steps = 0u32;
    let mut scan_steps_capped = false;
    let mut cur = resolve_match_start_hint(storage, book_start_hint, LimitOrderSide::Ask)?;
    while makers_used < cap {
        if !book_walk_step(&mut scan_steps) {
            scan_steps_capped = true;
            break;
        }
        let oid = match cur {
            Some(id) => id,
            None => break,
        };
        let order = ORDERS.load(storage, oid)?;
        let next_ptr = order.next;
        if !order_on_match_side(&order, LimitOrderSide::Ask) {
            cur = next_ptr;
            continue;
        }
        if order.expires_at.is_some_and(|e| now >= e) {
            cur = next_ptr;
            continue;
        }
        if maker_owner_is_trade_blacklisted(blacklist_gate, &order.owner)? {
            cur = next_ptr;
            continue;
        }
        if order.remaining.is_zero() {
            cur = next_ptr;
            continue;
        }
        if token1_left.is_zero() {
            break;
        }
        let mut order = order;
        let max_fill_token0_from_ask = order.remaining;
        let max_fill_token0_from_budget = if !order.price.is_zero() {
            let Some(inv) = try_price_inverse(order.price) else {
                cur = next_ptr;
                continue;
            };
            let Some(budget_fill) = try_mul_floor(token1_left, inv) else {
                cur = next_ptr;
                continue;
            };
            budget_fill
        } else {
            Uint128::zero()
        };
        let mut fill_t0 = max_fill_token0_from_ask.min(max_fill_token0_from_budget);
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }
        let Some(mut cost) = try_mul_floor(fill_t0, order.price) else {
            cur = next_ptr;
            continue;
        };
        let mut skip_order = false;
        while fill_t0 > Uint128::zero() && cost > token1_left {
            fill_t0 = fill_t0.saturating_sub(Uint128::one());
            if fill_t0.is_zero() {
                break;
            }
            match try_mul_floor(fill_t0, order.price) {
                Some(adjusted) => cost = adjusted,
                None => {
                    skip_order = true;
                    break;
                }
            }
        }
        if skip_order {
            cur = next_ptr;
            continue;
        }
        if fill_t0.is_zero() {
            cur = order.next;
            continue;
        }
        if cost.is_zero() {
            cur = order.next;
            continue;
        }
        makers_used += 1;
        let commission = fill_t0
            .checked_mul(Uint128::new(taker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        let net_to_taker = fill_t0.checked_sub(commission)?;
        commission_total = commission_total.checked_add(commission)?;
        order.remaining = order.remaining.checked_sub(fill_t0)?;
        apply_sim_dust_flush(&mut order);
        token1_left = token1_left.checked_sub(cost)?;
        token0_out_total = token0_out_total.checked_add(net_to_taker)?;
        cur = order.next;
    }
    Ok(BookSimulateResult {
        return_net: token0_out_total,
        offer_consumed: token1_budget.checked_sub(token1_left)?,
        makers_used,
        scan_steps_capped,
        commission_total,
    })
}

pub fn load_order_response(storage: &dyn Storage, id: u64) -> StdResult<LimitOrderResponse> {
    let o = ORDERS.load(storage, id)?;
    Ok(LimitOrderResponse {
        order_id: id,
        owner: o.owner,
        side: o.side,
        price: o.price,
        remaining: o.remaining,
        expires_at: o.expires_at,
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
            None,
        )
        .unwrap();
        let id_better = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            b,
            None,
            32,
            None,
        )
        .unwrap();
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
            None,
        )
        .unwrap();
        let id_better = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(100),
            b,
            None,
            32,
            None,
        )
        .unwrap();
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
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_atomics(99u128, 2).unwrap(),
            Uint128::new(50),
            o.clone(),
            None,
            32,
            None,
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

    /// GitLab #256 — valid `hint_after` inserts after predecessor without head walk.
    #[test]
    fn insert_bid_with_valid_hint_after_is_o1() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_bid(
            storage,
            Decimal::from_ratio(98u128, 100u128),
            Uint128::new(100),
            o,
            Some(id2),
            256,
            None,
        )
        .unwrap();
        let lo2 = load_order_response(storage, id2).unwrap();
        assert_eq!(lo2.prev, Some(id1));
        assert_eq!(lo2.next, Some(id3));
        let lo3 = load_order_response(storage, id3).unwrap();
        assert_eq!(lo3.prev, Some(id2));
    }

    #[test]
    fn insert_bid_stale_hint_falls_back_to_head_walk() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        unlink_order(storage, id1).unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o,
            Some(id1),
            256,
            None,
        )
        .unwrap();
        assert_eq!(
            query_head(storage, LimitOrderSide::Bid).unwrap().unwrap(),
            id2
        );
    }

    #[test]
    fn insert_bid_wrong_side_hint_falls_back() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let ask_id = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let bid_id = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o,
            Some(ask_id),
            256,
            None,
        )
        .unwrap();
        let lo = load_order_response(storage, bid_id).unwrap();
        assert!(lo.prev.is_none());
    }

    #[test]
    fn insert_bid_hint_at_tail() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_bid(
            storage,
            Decimal::from_ratio(98u128, 100u128),
            Uint128::new(100),
            o,
            Some(id2),
            256,
            None,
        )
        .unwrap();
        let lo3 = load_order_response(storage, id3).unwrap();
        assert_eq!(lo3.prev, Some(id2));
        assert!(lo3.next.is_none());
        let lo2 = load_order_response(storage, id2).unwrap();
        assert_eq!(lo2.next, Some(id3));
        assert_eq!(lo2.prev, Some(id1));
    }

    #[test]
    fn insert_bid_bad_hint_with_max_steps_one_errors() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        for i in 0..5 {
            insert_bid(
                storage,
                Decimal::from_ratio(100u128 - i, 100u128),
                Uint128::new(100),
                o.clone(),
                None,
                256,
                None,
            )
            .unwrap();
        }
        let err = insert_bid(
            storage,
            Decimal::from_ratio(50u128, 100u128),
            Uint128::new(100),
            o,
            Some(999_999),
            1,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ContractError::LimitInsertStepsExceeded { max: 1 }
        ));
    }

    #[test]
    fn insert_ask_with_valid_hint_after() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_ask(
            storage,
            Decimal::from_ratio(101u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_ask(
            storage,
            Decimal::from_ratio(102u128, 100u128),
            Uint128::new(100),
            o,
            Some(id2),
            256,
            None,
        )
        .unwrap();
        let lo3 = load_order_response(storage, id3).unwrap();
        assert_eq!(lo3.prev, Some(id2));
        assert!(lo3.next.is_none());
        let lo1 = load_order_response(storage, id1).unwrap();
        assert_eq!(lo1.next, Some(id2));
        let _ = id3;
    }

    #[test]
    fn relink_limit_order_price_uses_hint() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        relink_limit_order_price(
            storage,
            id2,
            Decimal::from_ratio(98u128, 100u128),
            Some(id1),
            256,
        )
        .unwrap();
        let lo = load_order_response(storage, id2).unwrap();
        assert_eq!(lo.price, Decimal::from_ratio(98u128, 100u128));
        assert_eq!(lo.prev, Some(id1));
        assert!(lo.next.is_none());
    }

    /// GitLab #265 — near-miss hint (new better than hint): prev walk finds slot under tight cap.
    #[test]
    fn insert_bid_near_miss_hint_toward_head_succeeds_under_cap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_bid(
            storage,
            Decimal::from_ratio(98u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        // Hint id3 (too late); new belongs after id2 — head walk needs 3 steps, directional needs 1.
        let id4 = insert_bid(
            storage,
            Decimal::from_ratio(985u128, 1000u128),
            Uint128::new(100),
            o,
            Some(id3),
            3,
            None,
        )
        .unwrap();
        let lo = load_order_response(storage, id4).unwrap();
        assert_eq!(lo.prev, Some(id2));
        assert_eq!(lo.next, Some(id3));
        let lo2 = load_order_response(storage, id2).unwrap();
        assert_eq!(lo2.next, Some(id4));
        let _ = id1;
    }

    /// GitLab #265 — near-miss hint (new worse than hint.next): next walk finds slot under tight cap.
    #[test]
    fn insert_bid_near_miss_hint_toward_tail_succeeds_under_cap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_bid(
            storage,
            Decimal::from_ratio(98u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        // Hint id1 (too early); new belongs after id2 — directional tail walk from id2.
        let id4 = insert_bid(
            storage,
            Decimal::from_ratio(985u128, 1000u128),
            Uint128::new(100),
            o,
            Some(id1),
            4,
            None,
        )
        .unwrap();
        let lo = load_order_response(storage, id4).unwrap();
        assert_eq!(lo.prev, Some(id2));
        assert_eq!(lo.next, Some(id3));
        let lo2 = load_order_response(storage, id2).unwrap();
        assert_eq!(lo2.next, Some(id4));
    }

    /// GitLab #265 — deep book: near-miss hint succeeds where head walk would revert.
    #[test]
    fn insert_bid_deep_book_near_miss_hint_beats_head_walk_cap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let mut ids = Vec::new();
        for i in 0..20u128 {
            ids.push(
                insert_bid(
                    storage,
                    Decimal::from_ratio(100u128 - i, 100u128),
                    Uint128::new(100),
                    o.clone(),
                    None,
                    256,
                    None,
                )
                .unwrap(),
            );
        }
        let target_prev = ids[17];
        let stale_hint = ids[19];
        let id_new = insert_bid(
            storage,
            Decimal::from_ratio(825u128, 1000u128),
            Uint128::new(100),
            o,
            Some(stale_hint),
            3,
            None,
        )
        .unwrap();
        let lo = load_order_response(storage, id_new).unwrap();
        assert_eq!(lo.prev, Some(target_prev));
        assert_eq!(lo.next, Some(ids[18]));
    }

    #[test]
    fn insert_ask_near_miss_hint_toward_tail_succeeds_under_cap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_ask(
            storage,
            Decimal::from_ratio(101u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_ask(
            storage,
            Decimal::from_ratio(102u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id4 = insert_ask(
            storage,
            Decimal::from_ratio(1015u128, 1000u128),
            Uint128::new(100),
            o,
            Some(id1),
            4,
            None,
        )
        .unwrap();
        let lo = load_order_response(storage, id4).unwrap();
        assert_eq!(lo.prev, Some(id2));
        assert_eq!(lo.next, Some(id3));
        let _ = (id1, id2, id3);
    }

    #[test]
    fn relink_limit_order_price_near_miss_hint_succeeds() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let id1 = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id2 = insert_bid(
            storage,
            Decimal::from_ratio(99u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let id3 = insert_bid(
            storage,
            Decimal::from_ratio(98u128, 100u128),
            Uint128::new(100),
            o.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        relink_limit_order_price(
            storage,
            id1,
            Decimal::from_ratio(985u128, 1000u128),
            Some(id3),
            3,
        )
        .unwrap();
        let lo = load_order_response(storage, id1).unwrap();
        assert_eq!(lo.price, Decimal::from_ratio(985u128, 1000u128));
        assert_eq!(lo.prev, Some(id2));
        assert_eq!(lo.next, Some(id3));
    }

    #[test]
    fn park_expired_bid_unlinks_and_records_claim_without_pending_delta() {
        use crate::state::EXPIRED_LIMIT_CLAIMS;

        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let o = Addr::unchecked("owner");
        let exp = 100u64;
        let id = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(1000),
            o.clone(),
            None,
            32,
            Some(exp),
        )
        .unwrap();
        let pending_before = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();
        let ev = park_expired_limit_order_for_claim(storage, id, exp, "pair").unwrap();
        assert!(ev
            .attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "limit_order_expired_parked"));
        assert!(ORDERS.may_load(storage, id).unwrap().is_none());
        let row = EXPIRED_LIMIT_CLAIMS.load(storage, id).unwrap();
        assert_eq!(row.remaining, Uint128::new(1000));
        assert_eq!(row.owner, o);
        let pending_after = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();
        assert_eq!(pending_before, pending_after);
    }
}

/// GitLab #467 — legacy out-of-band resting prices skip instead of reverting the match walk.
#[cfg(test)]
mod limit_price_band_tests {
    use super::*;
    use crate::state::{LimitOrder, HEAD_ASK, ORDER_NEXT_ID, PENDING_ESCROW_TOKEN0};
    use cosmwasm_std::testing::mock_dependencies;

    fn seed_ask_chain(storage: &mut dyn Storage, asks: &[(u64, Decimal, Uint128)]) {
        for (i, (id, price, remaining)) in asks.iter().enumerate() {
            let prev = if i == 0 { None } else { Some(asks[i - 1].0) };
            let next = if i + 1 < asks.len() {
                Some(asks[i + 1].0)
            } else {
                None
            };
            ORDERS
                .save(
                    storage,
                    *id,
                    &LimitOrder {
                        owner: Addr::unchecked("maker"),
                        price: *price,
                        remaining: *remaining,
                        side: LimitOrderSide::Ask,
                        expires_at: None,
                        prev,
                        next,
                    },
                )
                .unwrap();
        }
        HEAD_ASK.save(storage, &Some(asks[0].0)).unwrap();
        let total: Uint128 = asks.iter().map(|(_, _, r)| *r).sum();
        PENDING_ESCROW_TOKEN0.save(storage, &total).unwrap();
        let max_id = asks.iter().map(|(id, _, _)| *id).max().unwrap();
        ORDER_NEXT_ID.save(storage, &(max_id + 1)).unwrap();
    }

    #[test]
    fn match_asks_skips_legacy_dust_price_without_reverting() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        seed_ask_chain(
            storage,
            &[
                (1, Decimal::raw(1), Uint128::one()),
                (2, Decimal::from_ratio(1u128, 2u128), Uint128::new(100_000)),
            ],
        );

        let token1_budget = Uint128::new(400_000_000_000_000_000_000_000u128);
        let result = match_asks(
            storage,
            1,
            token1_budget,
            8,
            None,
            "pair",
            "t0",
            "t1",
            &Addr::unchecked("taker"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert!(result.makers_used >= 1, "must fill past dust head");
        assert!(
            !result.return_net.is_zero(),
            "taker must receive token0 from valid ask"
        );
        assert_eq!(
            ORDERS.load(storage, 1).unwrap().remaining,
            Uint128::one(),
            "dust ask must be skipped, not filled"
        );
        let remaining_2 = ORDERS
            .may_load(storage, 2)
            .unwrap()
            .map(|o| o.remaining)
            .unwrap_or(Uint128::zero());
        assert!(
            remaining_2 < Uint128::new(100_000),
            "valid ask behind dust must fill (remaining={remaining_2})"
        );
    }
}

/// GitLab #272 — `book_start_hint` must match the active matcher side.
#[cfg(test)]
mod book_start_hint_side_tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn match_bids_wrong_side_hint_falls_back_to_bid_head() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let maker = Addr::unchecked("maker");
        let bid_id = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            maker.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let ask_id = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(40_000),
            maker,
            None,
            256,
            None,
        )
        .unwrap();
        let ask_before = ORDERS.load(storage, ask_id).unwrap().remaining;
        let bid_before = ORDERS.load(storage, bid_id).unwrap().remaining;

        let result = match_bids(
            storage,
            1,
            Uint128::new(10_000),
            8,
            Some(ask_id),
            "pair",
            "t0",
            "t1",
            &Addr::unchecked("taker"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert!(result.makers_used >= 1);
        assert_eq!(
            ORDERS.load(storage, ask_id).unwrap().remaining,
            ask_before,
            "ask must not be consumed via wrong-side hint"
        );
        assert!(
            ORDERS.load(storage, bid_id).unwrap().remaining < bid_before,
            "bid head should be matched"
        );
    }

    #[test]
    fn match_asks_wrong_side_hint_falls_back_to_ask_head() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let maker = Addr::unchecked("maker");
        let bid_id = insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            maker.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let ask_id = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(40_000),
            maker,
            None,
            256,
            None,
        )
        .unwrap();
        let bid_before = ORDERS.load(storage, bid_id).unwrap().remaining;
        let ask_before = ORDERS.load(storage, ask_id).unwrap().remaining;

        let result = match_asks(
            storage,
            1,
            Uint128::new(10_000),
            8,
            Some(bid_id),
            "pair",
            "t0",
            "t1",
            &Addr::unchecked("taker"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert!(result.makers_used >= 1);
        assert_eq!(
            ORDERS.load(storage, bid_id).unwrap().remaining,
            bid_before,
            "bid must not be consumed via wrong-side hint"
        );
        assert!(
            ORDERS.load(storage, ask_id).unwrap().remaining < ask_before,
            "ask head should be matched"
        );
    }

    #[test]
    fn simulate_match_bids_wrong_side_hint_matches_execute_start() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let maker = Addr::unchecked("maker");
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            maker.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let ask_id = insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(40_000),
            maker,
            None,
            256,
            None,
        )
        .unwrap();
        let sim = simulate_match_bids(storage, 1, Uint128::new(10_000), 8, Some(ask_id), 30, None)
            .unwrap();
        assert!(sim.makers_used >= 1);
        assert!(sim.return_net > Uint128::zero());
    }
}

/// GitLab #248 — book match accumulates maker payouts and defers CW20 msgs to `execute_swap`.
#[cfg(test)]
mod aggregation_tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn match_bids_dedupes_maker_payouts_by_owner() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();

        let result = match_bids(
            storage,
            1,
            Uint128::new(80_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.makers_used, 2);
        assert_eq!(result.maker_payouts.len(), 1);
        assert_eq!(
            result.maker_payouts.get(&owner).copied().unwrap(),
            Uint128::new(80_000)
        );
        let msgs = maker_payout_transfer_messages(&result.maker_payouts, "token0").unwrap();
        assert_eq!(msgs.len(), 1, "one CW20 transfer per distinct maker");
    }

    #[test]
    fn match_bids_commission_and_return_net_sum_per_fill() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(100_000),
            owner,
            None,
            256,
            None,
        )
        .unwrap();

        let fill = Uint128::new(10_000);
        let result = match_bids(
            storage,
            1,
            fill,
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        let taker_bps = taker_fee_bps(30);
        let cost = fill;
        let commission = cost
            .checked_mul(Uint128::new(taker_bps as u128))
            .unwrap()
            .checked_div(Uint128::new(10000))
            .unwrap();
        let net = cost.checked_sub(commission).unwrap();
        assert_eq!(result.commission_total, commission);
        assert_eq!(result.return_net, net);
    }

    /// GitLab #255 — one pending-escrow write per token side per `match_bids` invocation.
    #[test]
    fn match_bids_batches_pending_escrow_token1_subtract() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(8_000),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(8_000),
            owner,
            None,
            256,
            None,
        )
        .unwrap();
        let pending_before = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();

        let result = match_bids(
            storage,
            1,
            Uint128::new(12_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.makers_used, 2);
        let pending_after = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();
        let consumed = pending_before.checked_sub(pending_after).unwrap();
        assert_eq!(consumed, result.offer_consumed);
        assert_eq!(consumed, Uint128::new(12_000));
    }

    /// GitLab #255 — batched subtract uses `checked_sub` and the same underflow error as singles.
    #[test]
    fn match_bids_pending_escrow_underflow_on_excessive_batch_subtract() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            owner,
            None,
            256,
            None,
        )
        .unwrap();
        PENDING_ESCROW_TOKEN1
            .save(storage, &Uint128::new(1_000))
            .unwrap();

        let err = match_bids(
            storage,
            1,
            Uint128::new(10_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ContractError::InvariantViolation { reason } if reason == "pending escrow token1 underflow"
        ));
    }

    /// GitLab #255 — one pending-escrow write per token side per `match_asks` invocation.
    #[test]
    fn match_asks_batches_pending_escrow_token0_subtract() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(15_000),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        insert_ask(
            storage,
            Decimal::one(),
            Uint128::new(15_000),
            owner,
            None,
            256,
            None,
        )
        .unwrap();
        let pending_before = PENDING_ESCROW_TOKEN0.may_load(storage).unwrap().unwrap();

        let result = match_asks(
            storage,
            1,
            Uint128::new(20_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.makers_used, 2);
        let pending_after = PENDING_ESCROW_TOKEN0.may_load(storage).unwrap().unwrap();
        let consumed = pending_before.checked_sub(pending_after).unwrap();
        assert_eq!(consumed, Uint128::new(20_000));
    }

    /// GitLab #264 — bid fill leaving `remaining = 1` parks dust; escrow includes cost + dust.
    #[test]
    fn match_bid_dust_remainder_one_flushes_to_expired_claim() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let price = Decimal::from_ratio(105u128, 100u128);
        let fill = Uint128::new(94_380_952);
        let cost = fill.checked_mul_floor(price).unwrap();
        let escrow = cost.checked_add(Uint128::one()).unwrap();
        let order_id = insert_bid(storage, price, escrow, owner.clone(), None, 256, None).unwrap();
        let pending_before = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();

        let result = match_bids(
            storage,
            1,
            fill,
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.makers_used, 1);
        assert!(ORDERS.may_load(storage, order_id).unwrap().is_none());
        let claim = EXPIRED_LIMIT_CLAIMS.load(storage, order_id).unwrap();
        assert_eq!(claim.remaining, Uint128::one());
        assert_eq!(claim.owner, owner);
        assert!(claim.expires_at.is_none());
        let pending_after = PENDING_ESCROW_TOKEN1.may_load(storage).unwrap().unwrap();
        assert_eq!(
            pending_before.checked_sub(pending_after).unwrap(),
            cost,
            "fill batched subtract releases cost; dust stays in pending until claim (L1)"
        );
        assert_eq!(pending_after, Uint128::one());
        assert!(
            result.fill_events.iter().any(|e| e
                .attributes
                .iter()
                .any(|a| { a.key == "action" && a.value == "limit_order_expired_parked" })),
            "dust flush emits park event"
        );
    }

    /// GitLab #264 — ask partial leaving `remaining = 5` flushes on token0 side.
    #[test]
    fn match_ask_dust_remainder_five_flushes_to_expired_claim() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let price = Decimal::one();
        let order_id = insert_ask(
            storage,
            price,
            Uint128::new(15),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();
        let pending_before = PENDING_ESCROW_TOKEN0.may_load(storage).unwrap().unwrap();

        let result = match_asks(
            storage,
            1,
            Uint128::new(10),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.makers_used, 1);
        assert!(ORDERS.may_load(storage, order_id).unwrap().is_none());
        let claim = EXPIRED_LIMIT_CLAIMS.load(storage, order_id).unwrap();
        assert_eq!(claim.remaining, Uint128::new(5));
        let pending_after = PENDING_ESCROW_TOKEN0.may_load(storage).unwrap().unwrap();
        assert_eq!(
            pending_before.checked_sub(pending_after).unwrap(),
            Uint128::new(10),
            "fill subtracts consumed token0; dust pending until claim"
        );
        assert_eq!(pending_after, Uint128::new(5));
    }

    /// GitLab #264 — `remaining = 10` stays on book (threshold is exclusive upper bound).
    #[test]
    fn match_bid_remaining_ten_stays_on_book() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let price = Decimal::one();
        let order_id =
            insert_bid(storage, price, Uint128::new(20), owner, None, 256, None).unwrap();

        match_bids(
            storage,
            1,
            Uint128::new(10),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        let order = ORDERS.load(storage, order_id).unwrap();
        assert_eq!(order.remaining, Uint128::new(10));
        assert!(EXPIRED_LIMIT_CLAIMS
            .may_load(storage, order_id)
            .unwrap()
            .is_none());
    }

    /// GitLab #264 / #255 — multi-fill ladder: each near-complete fill parks dust; sim = execute.
    #[test]
    fn match_bids_multi_maker_dust_flush_sim_matches_execute() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let price = Decimal::from_ratio(105u128, 100u128);
        let fill_each = Uint128::new(94_380_952);
        let cost = fill_each.checked_mul_floor(price).unwrap();
        let escrow = cost.checked_add(Uint128::one()).unwrap();
        let mut order_ids = Vec::new();
        for _ in 0..5 {
            order_ids
                .push(insert_bid(storage, price, escrow, owner.clone(), None, 256, None).unwrap());
        }

        let budget = fill_each.checked_mul(Uint128::new(5)).unwrap();
        let exec = match_bids(
            storage,
            1,
            budget,
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();
        assert_eq!(exec.makers_used, 5);
        for id in &order_ids {
            assert!(ORDERS.may_load(storage, *id).unwrap().is_none());
            assert_eq!(
                EXPIRED_LIMIT_CLAIMS.load(storage, *id).unwrap().remaining,
                Uint128::one()
            );
        }

        let mut deps2 = mock_dependencies();
        let storage2 = deps2.as_mut().storage;
        for _ in 0..5 {
            insert_bid(storage2, price, escrow, owner.clone(), None, 256, None).unwrap();
        }
        let sim = simulate_match_bids(storage2, 1, budget, 8, None, 30, None).unwrap();
        assert_eq!(sim.makers_used, exec.makers_used);
        assert_eq!(sim.offer_consumed, exec.offer_consumed);
        assert_eq!(sim.return_net, exec.return_net);
        assert_eq!(sim.commission_total, exec.commission_total);
    }

    #[test]
    fn should_flush_dust_boundary_nine_yes_ten_no() {
        assert!(should_flush_dust(Uint128::new(9)));
        assert!(!should_flush_dust(Uint128::new(10)));
        assert!(!should_flush_dust(Uint128::zero()));
    }
}

/// Property tests for limit-book invariants (L1 / L5 in `docs/contracts-security-audit.md`).
#[cfg(test)]
mod proptest_limits {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use proptest::prelude::*;

    fn walk_bid_sum(storage: &dyn Storage) -> Uint128 {
        let mut sum = Uint128::zero();
        let mut cur = HEAD_BID.may_load(storage).unwrap().flatten();
        let mut prev: Option<u64> = None;
        while let Some(id) = cur {
            let o = ORDERS.load(storage, id).unwrap();
            assert_eq!(o.side, LimitOrderSide::Bid);
            assert_eq!(o.prev, prev, "DLL prev link at {}", id);
            sum = sum.checked_add(o.remaining).unwrap();
            prev = Some(id);
            cur = o.next;
        }
        sum
    }

    fn walk_ask_sum(storage: &dyn Storage) -> Uint128 {
        let mut sum = Uint128::zero();
        let mut cur = HEAD_ASK.may_load(storage).unwrap().flatten();
        let mut prev: Option<u64> = None;
        while let Some(id) = cur {
            let o = ORDERS.load(storage, id).unwrap();
            assert_eq!(o.side, LimitOrderSide::Ask);
            assert_eq!(o.prev, prev, "DLL prev link at {}", id);
            sum = sum.checked_add(o.remaining).unwrap();
            prev = Some(id);
            cur = o.next;
        }
        sum
    }

    fn assert_escrow_matches_lists(storage: &dyn Storage) {
        let p1 = PENDING_ESCROW_TOKEN1
            .may_load(storage)
            .unwrap()
            .unwrap_or_default();
        let p0 = PENDING_ESCROW_TOKEN0
            .may_load(storage)
            .unwrap()
            .unwrap_or_default();
        // GitLab #264: sub-threshold dust is parked off-book into EXPIRED_LIMIT_CLAIMS while
        // escrow keeps backing it until the maker claims (L1). Escrow therefore equals on-book
        // remaining PLUS parked dust on each side, not on-book remaining alone.
        let mut parked_bid = Uint128::zero();
        let mut parked_ask = Uint128::zero();
        for item in EXPIRED_LIMIT_CLAIMS.range(storage, None, None, cosmwasm_std::Order::Ascending)
        {
            let (_id, refund) = item.unwrap();
            match refund.side {
                LimitOrderSide::Bid => {
                    parked_bid = parked_bid.checked_add(refund.remaining).unwrap()
                }
                LimitOrderSide::Ask => {
                    parked_ask = parked_ask.checked_add(refund.remaining).unwrap()
                }
            }
        }
        assert_eq!(walk_bid_sum(storage).checked_add(parked_bid).unwrap(), p1);
        assert_eq!(walk_ask_sum(storage).checked_add(parked_ask).unwrap(), p0);
    }

    #[derive(Clone, Debug)]
    enum Op {
        Bid {
            price_num: u128,
            price_den: u128,
            amt: u128,
        },
        Ask {
            price_num: u128,
            price_den: u128,
            amt: u128,
        },
    }

    fn op_strategy() -> impl Strategy<Value = Op> {
        prop_oneof![
            (1u128..=500u128, 1u128..=100u128, 1u128..=50_000u128).prop_map(|(n, d, a)| Op::Bid {
                price_num: n,
                price_den: d,
                amt: a,
            }),
            (1u128..=500u128, 1u128..=100u128, 1u128..=50_000u128).prop_map(|(n, d, a)| Op::Ask {
                price_num: n,
                price_den: d,
                amt: a,
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_escrow_dll_after_random_inserts(ops in proptest::collection::vec(op_strategy(), 0..24)) {
            let mut deps = mock_dependencies();
            let storage = deps.as_mut().storage;
            let owner = Addr::unchecked("maker");
            for op in ops {
                match op {
                    Op::Bid { price_num, price_den, amt } => {
                        let price = Decimal::from_ratio(price_num, price_den);
                        insert_bid(
                            storage,
                            price,
                            Uint128::new(amt),
                            owner.clone(),
                            None,
                            256,
                            None,
                        ).unwrap();
                    }
                    Op::Ask { price_num, price_den, amt } => {
                        let price = Decimal::from_ratio(price_num, price_den);
                        insert_ask(
                            storage,
                            price,
                            Uint128::new(amt),
                            owner.clone(),
                            None,
                            256,
                            None,
                        ).unwrap();
                    }
                }
                assert_escrow_matches_lists(storage);
            }
        }

        #[test]
        fn prop_match_bids_maker_cap(
            n_bids in 1usize..8usize,
            budget in 1u128..200_000u128,
            max_makers in 0u32..20u32,
        ) {
            let mut deps = mock_dependencies();
            let storage = deps.as_mut().storage;
            let owner = Addr::unchecked("maker");
            for i in 0..n_bids {
                let price = Decimal::from_ratio(100u128 + i as u128, 100u128);
                insert_bid(
                    storage,
                    price,
                    Uint128::new(10_000),
                    owner.clone(),
                    None,
                    256,
                    None,
                )
                .unwrap();
            }
            let cap = max_makers.min(MAX_MAKER_FILLS_HARD_CAP);
            let result = match_bids(
                storage,
                1,
                Uint128::new(budget),
                max_makers,
                None,
                "pair_addr",
                "token0_addr",
                "token1_addr",
                &Addr::unchecked("recv"),
                &Addr::unchecked("treasury"),
                30,
                None,
            )
            .unwrap();
            prop_assert!(result.makers_used <= cap);
            assert_escrow_matches_lists(storage);
        }

        #[test]
        fn prop_match_asks_maker_cap(
            n_asks in 1usize..8usize,
            budget in 1u128..200_000u128,
            max_makers in 0u32..20u32,
        ) {
            let mut deps = mock_dependencies();
            let storage = deps.as_mut().storage;
            let owner = Addr::unchecked("maker");
            for i in 0..n_asks {
                let price = Decimal::from_ratio(100u128 + i as u128, 100u128);
                insert_ask(
                    storage,
                    price,
                    Uint128::new(10_000),
                    owner.clone(),
                    None,
                    256,
                    None,
                )
                .unwrap();
            }
            let cap = max_makers.min(MAX_MAKER_FILLS_HARD_CAP);
            let result = match_asks(
                storage,
                1,
                Uint128::new(budget),
                max_makers,
                None,
                "pair_addr",
                "token0_addr",
                "token1_addr",
                &Addr::unchecked("recv"),
                &Addr::unchecked("treasury"),
                30,
                None,
            )
            .unwrap();
            prop_assert!(result.makers_used <= cap);
            assert_escrow_matches_lists(storage);
        }

        #[test]
        fn prop_match_bids_adversarial_wrong_side_hint_preserves_escrow(
            n_bids in 1usize..5usize,
            budget in 1u128..100_000u128,
            ask_amt in 1u128..50_000u128,
        ) {
            let mut deps = mock_dependencies();
            let storage = deps.as_mut().storage;
            let owner = Addr::unchecked("maker");
            for i in 0..n_bids {
                let price = Decimal::from_ratio(100u128 + i as u128, 100u128);
                insert_bid(
                    storage,
                    price,
                    Uint128::new(10_000),
                    owner.clone(),
                    None,
                    256,
                    None,
                ).unwrap();
            }
            let ask_id = insert_ask(
                storage,
                Decimal::one(),
                Uint128::new(ask_amt),
                owner,
                None,
                256,
                None,
            ).unwrap();
            let _ = match_bids(
                storage,
                1,
                Uint128::new(budget),
                8,
                Some(ask_id),
                "pair",
                "t0",
                "t1",
                &Addr::unchecked("recv"),
                &Addr::unchecked("treasury"),
                30,
                None,
            ).unwrap();
            assert_escrow_matches_lists(storage);
        }
    }
}

/// GitLab #250 — cap expired limit parks during hybrid match walks.
#[cfg(test)]
mod expired_park_cap_tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    fn insert_expired_bid(
        storage: &mut dyn Storage,
        owner: Addr,
        exp: u64,
        escrow: Uint128,
        price_offset: u64,
    ) -> u64 {
        let price = Decimal::from_ratio(100u128 + price_offset as u128, 100u128);
        insert_bid(storage, price, escrow, owner, None, 256, Some(exp)).unwrap()
    }

    #[test]
    fn match_bids_parks_at_most_max_expired_parks_per_swap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let exp = 100u64;
        for i in 0..20 {
            insert_expired_bid(storage, owner.clone(), exp, Uint128::new(1_000), i);
        }

        let result = match_bids(
            storage,
            exp,
            Uint128::new(50_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.expired_parks, MAX_EXPIRED_PARKS_PER_SWAP);
        assert_eq!(
            result.expired_parks_skipped,
            20 - MAX_EXPIRED_PARKS_PER_SWAP
        );
        assert_eq!(result.makers_used, 0);
        assert_eq!(
            result
                .fill_events
                .iter()
                .filter(|e| {
                    e.attributes
                        .iter()
                        .any(|a| a.key == "action" && a.value == "limit_order_expired_parked")
                })
                .count(),
            MAX_EXPIRED_PARKS_PER_SWAP as usize
        );
        let parked = EXPIRED_LIMIT_CLAIMS
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .count();
        assert_eq!(parked, MAX_EXPIRED_PARKS_PER_SWAP as usize);
    }

    #[test]
    fn match_bids_parks_three_expired_then_fills_behind() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let exp = 100u64;
        for _ in 0..3 {
            insert_expired_bid(storage, owner.clone(), exp, Uint128::new(1_000), 0);
        }
        insert_bid(
            storage,
            Decimal::one(),
            Uint128::new(50_000),
            owner.clone(),
            None,
            256,
            None,
        )
        .unwrap();

        let result = match_bids(
            storage,
            exp,
            Uint128::new(10_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.expired_parks, 3);
        assert_eq!(result.expired_parks_skipped, 0);
        assert_eq!(result.makers_used, 1);
    }

    #[test]
    fn match_asks_parks_at_most_max_expired_parks_per_swap() {
        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let exp = 100u64;
        for _ in 0..20 {
            insert_ask(
                storage,
                Decimal::one(),
                Uint128::new(1_000),
                owner.clone(),
                None,
                256,
                Some(exp),
            )
            .unwrap();
        }

        let result = match_asks(
            storage,
            exp,
            Uint128::new(50_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert_eq!(result.expired_parks, MAX_EXPIRED_PARKS_PER_SWAP);
        assert_eq!(
            result.expired_parks_skipped,
            20 - MAX_EXPIRED_PARKS_PER_SWAP
        );
    }

    #[test]
    fn match_bids_scan_steps_cap_bounds_expired_prefix_walk() {
        use dex_common::pair::MAX_SCAN_STEPS;

        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let exp = 100u64;
        let order_count = (MAX_SCAN_STEPS + 50) as usize;
        for i in 0..order_count as u64 {
            insert_expired_bid(storage, owner.clone(), exp, Uint128::new(1_000), i);
        }

        let result = match_bids(
            storage,
            exp,
            Uint128::new(50_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();

        assert!(result.scan_steps_capped);
        assert_eq!(result.expired_parks, MAX_EXPIRED_PARKS_PER_SWAP);
        assert_eq!(
            result.expired_parks + result.expired_parks_skipped,
            MAX_SCAN_STEPS,
            "each scan step should park or skip one expired head order"
        );
        let on_book = ORDERS
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .count();
        assert_eq!(
            on_book,
            order_count - MAX_EXPIRED_PARKS_PER_SWAP as usize,
            "only parked orders leave ORDERS; scan cap leaves skipped backlog"
        );
    }

    #[test]
    fn simulate_match_bids_scan_steps_cap_matches_execute() {
        use dex_common::pair::MAX_SCAN_STEPS;

        let mut deps = mock_dependencies();
        let storage = deps.as_mut().storage;
        let owner = Addr::unchecked("maker");
        let exp = 100u64;
        for i in 0..(MAX_SCAN_STEPS + 50) {
            insert_expired_bid(storage, owner.clone(), exp, Uint128::new(1_000), i as u64);
        }

        let exec = match_bids(
            storage,
            exp,
            Uint128::new(50_000),
            8,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("recv"),
            &Addr::unchecked("treasury"),
            30,
            None,
        )
        .unwrap();
        let sim =
            simulate_match_bids(storage, exp, Uint128::new(50_000), 8, None, 30, None).unwrap();

        assert!(exec.scan_steps_capped);
        assert_eq!(sim.scan_steps_capped, exec.scan_steps_capped);
        assert_eq!(sim.makers_used, exec.makers_used);
        assert_eq!(sim.offer_consumed, exec.offer_consumed);
        assert_eq!(sim.return_net, exec.return_net);
    }
}
