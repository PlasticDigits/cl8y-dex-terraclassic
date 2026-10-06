use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, StdError, StdResult, Uint128};

use crate::types::Asset;

#[cw_serde]
pub enum HookExecuteMsg {
    AfterSwap {
        pair: Addr,
        sender: Addr,
        offer_asset: Asset,
        return_asset: Asset,
        /// Total protocol commission in the ask asset: pool leg + book taker fees
        /// (treasury transfers). Matches [`HybridSimulationResponse::commission_amount`].
        /// Invariant **L7** — [GitLab #196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196).
        commission_amount: Uint128,
        /// Constant-product spread metric from the pool leg only.
        spread_amount: Uint128,
    },
}

/// Wrapper used by the pair contract when calling hooks.
/// Serializes identically to each hook's `ExecuteMsg::Hook(...)` variant.
#[cw_serde]
pub enum HookCallMsg {
    Hook(HookExecuteMsg),
}

/// Maximum number of post-swap hooks on one pair (#1398).
/// Eight supports the standard policy hooks while bounding config queries to
/// eight and callback dispatch to eight. This is a hard execution bound,
/// not a claim that arbitrary hook code has bounded gas.
pub const MAX_PAIR_HOOKS: usize = 8;

fn validate_hook_count(count: usize) -> StdResult<()> {
    if count > MAX_PAIR_HOOKS {
        return Err(StdError::generic_err(format!(
            "Too many pair hooks: {count}; maximum {MAX_PAIR_HOOKS}"
        )));
    }
    Ok(())
}

/// Validate a replacement list completely before saving or emitting messages.
/// Preserve caller order; never silently truncate or deduplicate policy hooks.
pub fn validate_pair_hooks(api: &dyn Api, hooks: &[String]) -> StdResult<Vec<Addr>> {
    // Reject oversize input before address validation or allocation.
    validate_hook_count(hooks.len())?;
    let validated = hooks
        .iter()
        .map(|hook| api.addr_validate(hook))
        .collect::<StdResult<Vec<_>>>()?;
    validate_stored_hooks(&validated)?;
    Ok(validated)
}

/// Fail closed on legacy invalid state before any hook query or callback.
/// Governance can repair it with a complete bounded UpdateHooks replacement;
/// execution must never drop a suffix or a repeated policy hook implicitly.
pub fn validate_stored_hooks(hooks: &[Addr]) -> StdResult<()> {
    validate_hook_count(hooks.len())?;
    for (index, hook) in hooks.iter().enumerate() {
        if hooks[..index].contains(hook) {
            return Err(StdError::generic_err(format!(
                "Duplicate pair hook: {hook}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "hook_bounds_tests.rs"]
mod hook_bounds_tests;
