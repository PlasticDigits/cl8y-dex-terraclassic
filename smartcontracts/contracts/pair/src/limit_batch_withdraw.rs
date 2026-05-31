//! Batch cancel and batch claim for resting / parked-expiry limit orders (GitLab #246).

use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use std::collections::BTreeSet;

use crate::error::ContractError;
use crate::limit_placement::load_limit_order_config;
use crate::orderbook;
use crate::state::{
    EXPIRED_LIMIT_CLAIMS, ORDERS, PAIR_INFO, PENDING_ESCROW_TOKEN0, PENDING_ESCROW_TOKEN1,
};
use dex_common::pair::LimitOrderSide;
use dex_common::types::AssetInfo;

fn token_addr(info: &AssetInfo) -> &str {
    match info {
        AssetInfo::Token { contract_addr } => contract_addr.as_str(),
        AssetInfo::NativeToken { .. } => "",
    }
}

fn validate_batch_order_ids(
    storage: &dyn cosmwasm_std::Storage,
    order_ids: &[u64],
) -> Result<(), ContractError> {
    if order_ids.is_empty() {
        return Err(ContractError::LimitBatchEmpty {});
    }
    let config = load_limit_order_config(storage)?;
    let max = config.max_batch_rungs;
    let actual = order_ids.len() as u32;
    if actual > max {
        return Err(ContractError::LimitBatchTooLarge { max, actual });
    }
    let mut seen = BTreeSet::new();
    for id in order_ids {
        if !seen.insert(*id) {
            return Err(ContractError::LimitBatchDuplicateOrderId { order_id: *id });
        }
    }
    Ok(())
}

fn maybe_transfer(
    token_addr: &str,
    recipient: &str,
    amount: Uint128,
) -> Result<Option<CosmosMsg>, ContractError> {
    if amount.is_zero() {
        return Ok(None);
    }
    Ok(Some(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
        })?,
        funds: vec![],
    })))
}

/// Cancel up to `max_batch_rungs` resting orders in one tx; aggregate refunds into ≤ 2 CW20 transfers.
pub fn execute_cancel_limit_orders(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    order_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    validate_batch_order_ids(deps.storage, &order_ids)?;

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let token_a = token_addr(&pair_info.asset_infos[0]).to_string();
    let token_b = token_addr(&pair_info.asset_infos[1]).to_string();
    let owner = info.sender.clone();
    let owner_str = owner.as_str();

    let mut refund_token0 = Uint128::zero();
    let mut refund_token1 = Uint128::zero();
    let mut cancelled: Vec<u64> = Vec::with_capacity(order_ids.len());

    for order_id in order_ids {
        let o = ORDERS.load(deps.storage, order_id)?;
        if o.owner != owner {
            return Err(ContractError::Unauthorized {});
        }
        let removed = orderbook::unlink_order(deps.storage, order_id)?;
        match removed.side {
            LimitOrderSide::Bid => {
                refund_token1 = refund_token1.checked_add(removed.remaining)?;
            }
            LimitOrderSide::Ask => {
                refund_token0 = refund_token0.checked_add(removed.remaining)?;
            }
        }
        cancelled.push(order_id);
    }

    if !refund_token0.is_zero() {
        let mut esc = PENDING_ESCROW_TOKEN0
            .may_load(deps.storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_sub(refund_token0)
            .map_err(|_| ContractError::InvariantViolation {
                reason: "pending escrow token0 underflow on batch cancel".into(),
            })?;
        PENDING_ESCROW_TOKEN0.save(deps.storage, &esc)?;
    }
    if !refund_token1.is_zero() {
        let mut esc = PENDING_ESCROW_TOKEN1
            .may_load(deps.storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_sub(refund_token1)
            .map_err(|_| ContractError::InvariantViolation {
                reason: "pending escrow token1 underflow on batch cancel".into(),
            })?;
        PENDING_ESCROW_TOKEN1.save(deps.storage, &esc)?;
    }

    let mut resp = Response::new()
        .add_attribute("action", "cancel_limit_orders_batch")
        .add_attribute("batch_count", cancelled.len().to_string());

    if let Some(msg) = maybe_transfer(&token_a, owner_str, refund_token0)? {
        resp = resp.add_message(msg);
    }
    if let Some(msg) = maybe_transfer(&token_b, owner_str, refund_token1)? {
        resp = resp.add_message(msg);
    }

    for order_id in &cancelled {
        resp = resp
            .add_attribute("action", "cancel_limit_order")
            .add_attribute("limit_order_cancelled", order_id.to_string())
            .add_attribute("owner", owner_str);
    }

    Ok(resp)
}

/// Claim up to `max_batch_rungs` parked-expiry rows in one tx; aggregate refunds into ≤ 2 CW20 transfers.
pub fn execute_claim_expired_limit_orders(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    order_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    validate_batch_order_ids(deps.storage, &order_ids)?;

    let pair_info = PAIR_INFO.load(deps.storage)?;
    let token_a = token_addr(&pair_info.asset_infos[0]).to_string();
    let token_b = token_addr(&pair_info.asset_infos[1]).to_string();
    let owner = info.sender.clone();
    let owner_str = owner.as_str();

    let mut refund_token0 = Uint128::zero();
    let mut refund_token1 = Uint128::zero();
    let mut claimed: Vec<u64> = Vec::with_capacity(order_ids.len());

    for order_id in order_ids {
        let row = EXPIRED_LIMIT_CLAIMS
            .may_load(deps.storage, order_id)?
            .ok_or(ContractError::NoExpiredLimitClaim { order_id })?;
        if row.owner != owner {
            return Err(ContractError::Unauthorized {});
        }
        match row.side {
            LimitOrderSide::Bid => {
                refund_token1 = refund_token1.checked_add(row.remaining)?;
            }
            LimitOrderSide::Ask => {
                refund_token0 = refund_token0.checked_add(row.remaining)?;
            }
        }
        EXPIRED_LIMIT_CLAIMS.remove(deps.storage, order_id);
        claimed.push(order_id);
    }

    if !refund_token0.is_zero() {
        let mut esc = PENDING_ESCROW_TOKEN0
            .may_load(deps.storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_sub(refund_token0)
            .map_err(|_| ContractError::InvariantViolation {
                reason: "pending escrow token0 underflow on batch claim expired".into(),
            })?;
        PENDING_ESCROW_TOKEN0.save(deps.storage, &esc)?;
    }
    if !refund_token1.is_zero() {
        let mut esc = PENDING_ESCROW_TOKEN1
            .may_load(deps.storage)?
            .unwrap_or(Uint128::zero());
        esc = esc
            .checked_sub(refund_token1)
            .map_err(|_| ContractError::InvariantViolation {
                reason: "pending escrow token1 underflow on batch claim expired".into(),
            })?;
        PENDING_ESCROW_TOKEN1.save(deps.storage, &esc)?;
    }

    let mut resp = Response::new()
        .add_attribute("action", "claim_expired_limit_orders_batch")
        .add_attribute("batch_count", claimed.len().to_string());

    if let Some(msg) = maybe_transfer(&token_a, owner_str, refund_token0)? {
        resp = resp.add_message(msg);
    }
    if let Some(msg) = maybe_transfer(&token_b, owner_str, refund_token1)? {
        resp = resp.add_message(msg);
    }

    for order_id in &claimed {
        resp = resp
            .add_attribute("action", "claim_expired_limit_order")
            .add_attribute("order_id", order_id.to_string())
            .add_attribute("owner", owner_str);
    }

    Ok(resp)
}
