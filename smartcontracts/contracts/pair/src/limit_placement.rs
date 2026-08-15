//! Batch and ladder limit order placement (GitLab #206, book-order traversal #266).

use std::cmp::Ordering;

use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Storage, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::orderbook::{self, InsertThreadCursor};
use crate::state::{
    LimitOrderConfig, FEE_CONFIG, LIMIT_ORDER_CONFIG, PAIR_INFO, PENDING_ESCROW_TOKEN0,
    PENDING_ESCROW_TOKEN1,
};
use dex_common::limit_placement::{
    expand_limit_ladder, validate_limit_order_price, LimitOrderLadderSpec, LimitOrderPlacementItem,
};
use dex_common::pair::LimitOrderSide;
use dex_common::types::AssetInfo;

use super::contract::effective_fee_bps_with_discount_msgs;
use super::state::DISCOUNT_REGISTRY;

pub fn load_limit_order_config(storage: &dyn Storage) -> Result<LimitOrderConfig, ContractError> {
    LIMIT_ORDER_CONFIG.load(storage).map_err(ContractError::Std)
}

pub fn save_limit_order_config(
    storage: &mut dyn Storage,
    max_batch_rungs: u32,
) -> Result<(), ContractError> {
    let clamped = dex_common::pair::clamp_max_batch_rungs(max_batch_rungs);
    LIMIT_ORDER_CONFIG
        .save(
            storage,
            &LimitOrderConfig {
                max_batch_rungs: clamped,
            },
        )
        .map_err(ContractError::Std)
}

pub fn execute_place_limit_order_ladder(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Addr,
    send_amount: Uint128,
    ladder: LimitOrderLadderSpec,
) -> Result<Response, ContractError> {
    let config = load_limit_order_config(deps.storage)?;
    let orders = expand_limit_ladder(&ladder, config.max_batch_rungs).map_err(|e| {
        ContractError::LimitLadderInvalid {
            reason: e.to_string(),
        }
    })?;
    execute_place_limit_orders_batch(deps, env, info, owner, send_amount, ladder.side, orders)
}

struct BatchRungPlan {
    input_index: usize,
    id: u64,
    item: LimitOrderPlacementItem,
}

/// Sort key for book-order traversal: composite `(price, order_id)` (#266).
fn book_order_cmp(
    side: &LimitOrderSide,
    a_price: Decimal,
    a_id: u64,
    b_price: Decimal,
    b_id: u64,
) -> Ordering {
    let a_before_b = match side {
        LimitOrderSide::Bid => orderbook::bid_before(a_price, a_id, b_price, b_id),
        LimitOrderSide::Ask => orderbook::ask_before(a_price, a_id, b_price, b_id),
    };
    if a_before_b {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub fn execute_place_limit_orders_batch(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Addr,
    send_amount: Uint128,
    side: LimitOrderSide,
    orders: Vec<LimitOrderPlacementItem>,
) -> Result<Response, ContractError> {
    if orders.is_empty() {
        return Err(ContractError::LimitBatchEmpty {});
    }

    let config = load_limit_order_config(deps.storage)?;
    let max = config.max_batch_rungs;
    let actual = orders.len() as u32;
    if actual > max {
        return Err(ContractError::LimitBatchTooLarge { max, actual });
    }

    let expected_send: Uint128 = orders
        .iter()
        .try_fold(Uint128::zero(), |acc, o| acc.checked_add(o.amount))?;
    if send_amount != expected_send {
        return Err(ContractError::LimitBatchAmountMismatch {
            expected: expected_send.to_string(),
            actual: send_amount.to_string(),
        });
    }

    let now = env.block.time.seconds();
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    let discount_registry = DISCOUNT_REGISTRY.load(deps.storage)?;
    let (_swap_effective_fee_bps, effective_fee_bps, deregister_msgs) =
        effective_fee_bps_with_discount_msgs(
            deps.branch(),
            now,
            fee_config.fee_bps,
            discount_registry,
            owner.clone(),
        )?;

    let token_a = token_addr(&pair_info.asset_infos[0]);
    let token_b = token_addr(&pair_info.asset_infos[1]);
    let cw20_addr = info.sender.as_str();
    match side {
        LimitOrderSide::Bid => {
            if cw20_addr != token_b {
                return Err(ContractError::InvalidToken {});
            }
        }
        LimitOrderSide::Ask => {
            if cw20_addr != token_a {
                return Err(ContractError::InvalidToken {});
            }
        }
    }

    let side_label = orderbook::side_str(&side);
    let owner_str = owner.to_string();
    let maker_bps = orderbook::maker_fee_bps(effective_fee_bps);

    let mut resp = Response::new();
    for m in deregister_msgs {
        resp = resp.add_message(m);
    }

    let rung_count = orders.len();
    let mut next_id = orderbook::reserve_order_id_block(deps.storage, rung_count as u32)?;

    let mut plans: Vec<BatchRungPlan> = Vec::with_capacity(rung_count);
    for (input_index, item) in orders.into_iter().enumerate() {
        let id = next_id;
        next_id = next_id
            .checked_add(1)
            .ok_or_else(|| ContractError::InvariantViolation {
                reason: "order id overflow".into(),
            })?;
        plans.push(BatchRungPlan {
            input_index,
            id,
            item,
        });
    }

    plans.sort_by(|a, b| book_order_cmp(&side, a.item.price, a.id, b.item.price, b.id));

    let mut placed_by_input: Vec<Option<(u64, Uint128, Decimal)>> = vec![None; rung_count];
    let mut skipped_count: u32 = 0;
    let mut placed_amount_sum = Uint128::zero();
    let mut placed_escrow_remaining = Uint128::zero();
    let mut total_maker_fee = Uint128::zero();
    let mut thread_cursor: Option<InsertThreadCursor> = None;
    let mut last_placed_hint: Option<u64> = None;

    for plan in &plans {
        validate_placement_item(&plan.item, now)?;
        let maker_fee = plan
            .item
            .amount
            .checked_mul(Uint128::new(maker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        if maker_fee >= plan.item.amount {
            return Err(ContractError::LimitOrderMakerFeeExceedsAmount {});
        }
        let remaining_for_book = plan.item.amount.checked_sub(maker_fee)?;

        let hint_after = plan.item.hint_after_order_id.or(last_placed_hint);

        let insert_result = match side {
            LimitOrderSide::Bid => orderbook::insert_bid_with_id_for_batch(
                deps.storage,
                plan.id,
                plan.item.price,
                remaining_for_book,
                owner.clone(),
                thread_cursor,
                hint_after,
                plan.item.max_adjust_steps,
                plan.item.expires_at,
            ),
            LimitOrderSide::Ask => orderbook::insert_ask_with_id_for_batch(
                deps.storage,
                plan.id,
                plan.item.price,
                remaining_for_book,
                owner.clone(),
                thread_cursor,
                hint_after,
                plan.item.max_adjust_steps,
                plan.item.expires_at,
            ),
        };

        match insert_result {
            Ok(outcome) => {
                total_maker_fee = total_maker_fee.checked_add(maker_fee)?;
                placed_amount_sum = placed_amount_sum.checked_add(plan.item.amount)?;
                placed_escrow_remaining =
                    placed_escrow_remaining.checked_add(remaining_for_book)?;
                placed_by_input[plan.input_index] = Some((outcome.id, maker_fee, plan.item.price));
                thread_cursor = Some(outcome.thread_cursor);
                last_placed_hint = Some(outcome.id);
            }
            Err(ContractError::LimitInsertStepsExceeded { .. }) => {
                skipped_count += 1;
            }
            Err(e) => return Err(e),
        }
    }

    if !placed_escrow_remaining.is_zero() {
        match side {
            LimitOrderSide::Bid => {
                let mut esc = PENDING_ESCROW_TOKEN1
                    .may_load(deps.storage)?
                    .unwrap_or(Uint128::zero());
                esc = esc
                    .checked_add(placed_escrow_remaining)
                    .map_err(ContractError::Overflow)?;
                PENDING_ESCROW_TOKEN1.save(deps.storage, &esc)?;
            }
            LimitOrderSide::Ask => {
                let mut esc = PENDING_ESCROW_TOKEN0
                    .may_load(deps.storage)?
                    .unwrap_or(Uint128::zero());
                esc = esc
                    .checked_add(placed_escrow_remaining)
                    .map_err(ContractError::Overflow)?;
                PENDING_ESCROW_TOKEN0.save(deps.storage, &esc)?;
            }
        }
    }

    let placed_count = placed_by_input.iter().filter(|p| p.is_some()).count();
    if placed_count == 0 {
        return Err(ContractError::LimitBatchNoRungsPlaced {
            skipped: skipped_count,
        });
    }

    let refund = send_amount.checked_sub(placed_amount_sum)?;
    if !refund.is_zero() {
        resp = resp.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: refund,
            })?,
            funds: vec![],
        }));
    }

    if !total_maker_fee.is_zero() {
        resp = resp.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: fee_config.treasury.to_string(),
                amount: total_maker_fee,
            })?,
            funds: vec![],
        }));
    }

    resp = resp
        .add_attribute("action", "place_limit_order_batch")
        .add_attribute("batch_count", placed_count.to_string())
        .add_attribute("batch_skipped_count", skipped_count.to_string())
        .add_attribute("batch_refund_amount", refund)
        .add_attribute("side", side_label)
        .add_attribute("total_maker_fee_amount", total_maker_fee)
        .add_attribute("effective_fee_bps", effective_fee_bps.to_string());

    // Emit per-rung attrs in input/id order for indexer columnar zip (#266).
    for slot in placed_by_input.into_iter().flatten() {
        let (id, maker_fee, price) = slot;
        resp = resp
            .add_attribute("action", "place_limit_order")
            .add_attribute("limit_order_placed", id.to_string())
            .add_attribute("order_id", id.to_string())
            .add_attribute("side", side_label)
            .add_attribute("price", price.to_string())
            .add_attribute("owner", owner_str.as_str())
            .add_attribute("maker_fee_amount", maker_fee)
            .add_attribute("effective_fee_bps", effective_fee_bps.to_string());
    }

    Ok(resp)
}

fn validate_placement_item(item: &LimitOrderPlacementItem, now: u64) -> Result<(), ContractError> {
    if item.amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    validate_limit_order_price(item.price).map_err(|reason| {
        ContractError::InvalidHybridParams {
            reason: reason.into(),
        }
    })?;
    if let Some(t) = item.expires_at {
        if t <= now {
            return Err(ContractError::InvalidHybridParams {
                reason: "expires_at must be in the future".into(),
            });
        }
    }
    Ok(())
}

fn token_addr(info: &AssetInfo) -> &str {
    match info {
        AssetInfo::Token { contract_addr } => contract_addr.as_str(),
        AssetInfo::NativeToken { .. } => "",
    }
}
