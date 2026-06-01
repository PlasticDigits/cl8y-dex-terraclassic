//! Permissionless limit book clean — park expired/dust orders (GitLab #263).

use cosmwasm_std::{Event, Storage, Uint128};
use dex_common::pair::{clamp_max_clean_orders, LimitOrderSide, MAX_LIMIT_CLEAN_ORDERS_HARD_CAP};

use crate::error::ContractError;
use crate::orderbook::park_limit_order_for_clean;
use crate::state::{HEAD_ASK, HEAD_BID, LIMIT_CLEAN_CONFIG, ORDERS};

pub struct CleanLimitBookResult {
    pub cleaned_count: u32,
    pub time_expired_count: u32,
    pub force_expired_count: u32,
    pub cap_hit: bool,
    pub events: Vec<Event>,
}

pub fn load_limit_clean_config(storage: &dyn Storage) -> Result<(Uint128, Uint128), ContractError> {
    let cfg = LIMIT_CLEAN_CONFIG
        .may_load(storage)?
        .unwrap_or(LimitCleanConfigDefaults::ZERO);
    Ok((cfg.min_remaining_token0, cfg.min_remaining_token1))
}

struct LimitCleanConfigDefaults;

impl LimitCleanConfigDefaults {
    const ZERO: crate::state::LimitCleanConfig = crate::state::LimitCleanConfig {
        min_remaining_token0: Uint128::zero(),
        min_remaining_token1: Uint128::zero(),
    };
}

pub fn save_limit_clean_config(
    storage: &mut dyn Storage,
    min_remaining_token0: Uint128,
    min_remaining_token1: Uint128,
) -> Result<(), ContractError> {
    LIMIT_CLEAN_CONFIG.save(
        storage,
        &crate::state::LimitCleanConfig {
            min_remaining_token0,
            min_remaining_token1,
        },
    )?;
    Ok(())
}

fn min_remaining_for_side(
    side: &LimitOrderSide,
    min_token0: Uint128,
    min_token1: Uint128,
) -> Uint128 {
    match side {
        LimitOrderSide::Ask => min_token0,
        LimitOrderSide::Bid => min_token1,
    }
}

fn is_time_expired(expires_at: Option<u64>, now: u64) -> bool {
    expires_at.is_some_and(|e| now >= e)
}

fn is_force_dust(remaining: Uint128, min_remaining: Uint128) -> bool {
    !min_remaining.is_zero() && remaining < min_remaining
}

fn book_head(storage: &dyn Storage, side: &LimitOrderSide) -> Result<Option<u64>, ContractError> {
    Ok(match side {
        LimitOrderSide::Bid => HEAD_BID.may_load(storage)?.flatten(),
        LimitOrderSide::Ask => HEAD_ASK.may_load(storage)?.flatten(),
    })
}

fn resolve_start(
    storage: &dyn Storage,
    side: &LimitOrderSide,
    start_hint: Option<u64>,
) -> Result<Option<u64>, ContractError> {
    if let Some(hint) = start_hint {
        if let Some(order) = ORDERS.may_load(storage, hint)? {
            if &order.side == side {
                return Ok(Some(hint));
            }
        }
    }
    book_head(storage, side)
}

/// Walk the limit book from `start_hint` or head; park eligible orders up to `max_orders`.
pub fn clean_limit_book(
    storage: &mut dyn Storage,
    now: u64,
    side: LimitOrderSide,
    max_orders: u32,
    start_hint: Option<u64>,
    pair_contract: &str,
) -> Result<CleanLimitBookResult, ContractError> {
    if max_orders == 0 {
        return Err(ContractError::InvalidHybridParams {
            reason: "max_orders must be at least 1".into(),
        });
    }
    if max_orders > MAX_LIMIT_CLEAN_ORDERS_HARD_CAP {
        return Err(ContractError::LimitCleanMaxOrdersExceeded {
            max: MAX_LIMIT_CLEAN_ORDERS_HARD_CAP,
            actual: max_orders,
        });
    }
    let cap = clamp_max_clean_orders(max_orders);
    let (min_t0, min_t1) = load_limit_clean_config(storage)?;
    let min_remaining = min_remaining_for_side(&side, min_t0, min_t1);

    let mut cur = resolve_start(storage, &side, start_hint)?;
    let mut cleaned_count = 0u32;
    let mut time_expired_count = 0u32;
    let mut force_expired_count = 0u32;
    let mut cap_hit = false;
    let mut events = Vec::new();

    while let Some(oid) = cur {
        if cleaned_count >= cap {
            cap_hit = true;
            break;
        }

        let order = ORDERS.load(storage, oid)?;
        let next_ptr = order.next;

        if order.remaining.is_zero() {
            cur = next_ptr;
            continue;
        }

        let time_exp = is_time_expired(order.expires_at, now);
        let force = !time_exp && is_force_dust(order.remaining, min_remaining);

        if time_exp || force {
            let ev = park_limit_order_for_clean(
                storage,
                oid,
                pair_contract,
                force,
                if force { None } else { order.expires_at },
            )?;
            events.push(ev);
            cleaned_count += 1;
            if time_exp {
                time_expired_count += 1;
            } else {
                force_expired_count += 1;
            }
        }

        cur = next_ptr;
    }

    Ok(CleanLimitBookResult {
        cleaned_count,
        time_expired_count,
        force_expired_count,
        cap_hit,
        events,
    })
}
