//! Per-transaction swap ordinal for `limit_order_fill` wasm attrs (GitLab #331).
//!
//! Within one Cosmos tx, multiple `execute_swap` calls on the same pair each get a
//! 0-based `swap_index` matching indexer `swap_events.swap_index` / parser walk order.

use cosmwasm_std::{Env, Event, StdResult, Storage};

use crate::state::{TxSwapScope, TX_SWAP_COUNTER, TX_SWAP_SCOPE};

fn tx_swap_scope(env: &Env) -> TxSwapScope {
    TxSwapScope {
        height: env.block.height,
        tx_index: env.transaction.as_ref().map(|t| t.index).unwrap_or(0),
    }
}

/// Current swap ordinal for this tx (before incrementing for the swap being executed).
pub fn current_tx_swap_index(storage: &mut dyn Storage, env: &Env) -> StdResult<u32> {
    let scope = tx_swap_scope(env);
    if TX_SWAP_SCOPE.may_load(storage)? != Some(scope.clone()) {
        TX_SWAP_SCOPE.save(storage, &scope)?;
        TX_SWAP_COUNTER.save(storage, &0)?;
    }
    TX_SWAP_COUNTER.load(storage)
}

/// Call once per completed swap so the next swap in the same tx gets the next ordinal.
pub fn advance_tx_swap_index(storage: &mut dyn Storage, env: &Env) -> StdResult<()> {
    let scope = tx_swap_scope(env);
    if TX_SWAP_SCOPE.may_load(storage)? == Some(scope) {
        let n = TX_SWAP_COUNTER.load(storage)?;
        TX_SWAP_COUNTER.save(storage, &(n + 1))?;
    }
    Ok(())
}

/// Attach `swap_index` to each `limit_order_fill` wasm event (not lifecycle parks).
pub fn stamp_swap_index_on_fill_events(events: Vec<Event>, swap_index: u32) -> Vec<Event> {
    events
        .into_iter()
        .map(|ev| {
            if ev
                .attributes
                .iter()
                .any(|a| a.key == "action" && a.value == "limit_order_fill")
            {
                ev.add_attribute("swap_index", swap_index.to_string())
            } else {
                ev
            }
        })
        .collect()
}
