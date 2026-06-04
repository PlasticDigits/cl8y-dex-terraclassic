//! Short-lived CL8Y fee-discount cache on the pair ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)).
//!
//! Invariant ([GitLab #275](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/275), accepted design):
//! the cached `(effective_fee_bps, discount)` is a **`DISCOUNT_CACHE_TTL_SECONDS` (300s) snapshot**,
//! not a live per-trade balance check. Within the window the pair serves the cached discount
//! without re-querying the registry, so a tier holder must transact **at least once per 300s** to
//! keep it fresh, and a wallet that drops below its tier can still receive the cached discount
//! until the entry expires (fee leakage bounded to one 300s window). This is a deliberate gas
//! optimization — re-querying the registry on every swap is the cost it avoids (#251) — and the
//! once-per-window refresh requirement is considered sufficient demand on the CL8Y sink. The
//! registry itself (`fee_discount::query_discount`) always reads live balance; only the on-pair
//! cache window is a snapshot.

use cosmwasm_std::{Addr, Deps, DepsMut, Storage};
use dex_common::fee_discount;
use dex_common::pair::DISCOUNT_CACHE_TTL_SECONDS;

use crate::state::{DiscountCacheEntry, DISCOUNT_CACHE};

pub(crate) fn effective_fee_bps_from_discount(
    fee_bps: u16,
    discount: &fee_discount::DiscountResponse,
) -> u16 {
    let discounted =
        (fee_bps as u32) * (10000u32.saturating_sub(discount.discount_bps as u32)) / 10000u32;
    discounted as u16
}

fn cache_key(
    deps: Deps,
    trader: &str,
    sender: &str,
) -> Result<(Addr, Addr), cosmwasm_std::StdError> {
    Ok((
        deps.api.addr_validate(trader)?,
        deps.api.addr_validate(sender)?,
    ))
}

fn is_fresh(entry: &DiscountCacheEntry, block_time: u64) -> bool {
    block_time.saturating_sub(entry.cached_at) < DISCOUNT_CACHE_TTL_SECONDS
}

/// Read-only cache hit for simulation and execute paths.
pub(crate) fn try_discount_cache_hit(
    deps: Deps,
    storage: &dyn Storage,
    block_time: u64,
    trader: &str,
    sender: &str,
) -> Result<Option<(u16, fee_discount::DiscountResponse)>, cosmwasm_std::StdError> {
    let key = cache_key(deps, trader, sender)?;
    let Some(entry) = DISCOUNT_CACHE.may_load(storage, (&key.0, &key.1))? else {
        return Ok(None);
    };
    if !is_fresh(&entry, block_time) {
        return Ok(None);
    }
    if entry.discount.needs_deregister {
        return Ok(None);
    }
    Ok(Some((entry.effective_fee_bps, entry.discount)))
}

fn query_registry_discount(
    deps: Deps,
    registry: &Addr,
    trader: &str,
    sender: &str,
) -> Result<fee_discount::DiscountResponse, cosmwasm_std::StdError> {
    deps.querier.query_wasm_smart(
        registry.to_string(),
        &fee_discount::QueryMsg::GetDiscount {
            trader: trader.to_string(),
            sender: sender.to_string(),
        },
    )
}

pub(crate) fn save_discount_cache(
    storage: &mut dyn Storage,
    trader: &Addr,
    sender: &Addr,
    block_time: u64,
    effective_fee_bps: u16,
    discount: &fee_discount::DiscountResponse,
) -> Result<(), cosmwasm_std::StdError> {
    if discount.needs_deregister {
        return Ok(());
    }
    DISCOUNT_CACHE.save(
        storage,
        (trader, sender),
        &DiscountCacheEntry {
            effective_fee_bps,
            discount: discount.clone(),
            cached_at: block_time,
        },
    )
}

pub(crate) fn invalidate_discount_cache(
    storage: &mut dyn Storage,
    trader: &Addr,
    sender: &Addr,
) -> Result<(), cosmwasm_std::StdError> {
    DISCOUNT_CACHE.remove(storage, (trader, sender));
    Ok(())
}

/// Registry lookup with optional write-through cache (execute / limit placement).
pub(crate) fn lookup_effective_fee_bps_cached(
    deps: DepsMut,
    block_time: u64,
    fee_bps: u16,
    discount_registry: &Option<Addr>,
    trader: &str,
    sender: &str,
) -> Result<(u16, Option<fee_discount::DiscountResponse>), cosmwasm_std::StdError> {
    let Some(registry) = discount_registry else {
        return Ok((fee_bps, None));
    };

    if let Some((bps, discount)) =
        try_discount_cache_hit(deps.as_ref(), deps.storage, block_time, trader, sender)?
    {
        return Ok((bps, Some(discount)));
    }

    let discount_result = query_registry_discount(deps.as_ref(), registry, trader, sender);
    match discount_result {
        Ok(discount) => {
            let effective = effective_fee_bps_from_discount(fee_bps, &discount);
            let key = cache_key(deps.as_ref(), trader, sender)?;
            save_discount_cache(
                deps.storage,
                &key.0,
                &key.1,
                block_time,
                effective,
                &discount,
            )?;
            Ok((effective, Some(discount)))
        }
        Err(_) => Ok((fee_bps, None)),
    }
}

/// Read-only lookup for hybrid simulation (cache read, no write).
pub(crate) fn lookup_effective_fee_bps_readonly(
    deps: Deps,
    block_time: u64,
    fee_bps: u16,
    discount_registry: &Option<Addr>,
    trader: &str,
    sender: &str,
) -> (u16, Option<fee_discount::DiscountResponse>) {
    let Some(registry) = discount_registry else {
        return (fee_bps, None);
    };

    if let Ok(Some((bps, discount))) =
        try_discount_cache_hit(deps, deps.storage, block_time, trader, sender)
    {
        return (bps, Some(discount));
    }

    match query_registry_discount(deps, registry, trader, sender) {
        Ok(discount) => {
            let effective = effective_fee_bps_from_discount(fee_bps, &discount);
            (effective, Some(discount))
        }
        Err(_) => (fee_bps, None),
    }
}
