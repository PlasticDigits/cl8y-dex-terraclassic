//! Batch and ladder limit order placement (GitLab #206).

use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Storage, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::orderbook;
use crate::state::{LimitOrderConfig, FEE_CONFIG, LIMIT_ORDER_CONFIG, PAIR_INFO};
use dex_common::limit_placement::{
    expand_limit_ladder, LimitOrderLadderSpec, LimitOrderPlacementItem,
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

pub fn execute_place_limit_orders_batch(
    deps: DepsMut,
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

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let fee_config = FEE_CONFIG.load(deps.storage)?;
    let discount_registry = DISCOUNT_REGISTRY.load(deps.storage)?;
    let (effective_fee_bps, deregister_msgs) = effective_fee_bps_with_discount_msgs(
        deps.as_ref(),
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

    let now = env.block.time.seconds();
    let side_label = orderbook::side_str(&side);
    let owner_str = owner.to_string();
    let maker_bps = orderbook::maker_fee_bps(effective_fee_bps);

    let mut resp = Response::new();
    for m in deregister_msgs {
        resp = resp.add_message(m);
    }

    let mut total_maker_fee = Uint128::zero();
    let mut placed: Vec<(u64, Uint128, Decimal)> = Vec::with_capacity(orders.len());

    for item in &orders {
        validate_placement_item(item, now)?;
        let maker_fee = item
            .amount
            .checked_mul(Uint128::new(maker_bps as u128))?
            .checked_div(Uint128::new(10000))?;
        if maker_fee >= item.amount {
            return Err(ContractError::LimitOrderMakerFeeExceedsAmount {});
        }
        let remaining_for_book = item.amount.checked_sub(maker_fee)?;
        total_maker_fee = total_maker_fee.checked_add(maker_fee)?;

        let id = match side {
            LimitOrderSide::Bid => orderbook::insert_bid(
                deps.storage,
                item.price,
                remaining_for_book,
                owner.clone(),
                None,
                item.max_adjust_steps,
                item.expires_at,
            )?,
            LimitOrderSide::Ask => orderbook::insert_ask(
                deps.storage,
                item.price,
                remaining_for_book,
                owner.clone(),
                None,
                item.max_adjust_steps,
                item.expires_at,
            )?,
        };
        placed.push((id, maker_fee, item.price));
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
        .add_attribute("batch_count", placed.len().to_string())
        .add_attribute("side", side_label)
        .add_attribute("total_maker_fee_amount", total_maker_fee)
        .add_attribute("effective_fee_bps", effective_fee_bps.to_string());

    for (id, maker_fee, price) in placed {
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
    if item.price.is_zero() {
        return Err(ContractError::InvalidHybridParams {
            reason: "limit price must be positive".into(),
        });
    }
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
