//! Request-local accounting for public reverse queries (#1389).
//! Opaque pair reverse queries reserve their maximum internal simulation cost.
//! Agent protocol: skills/AGENTS_REVERSE_QUERY_BUDGET.md.
//! Verification: docs/qa/issue-1389/README.md.
use std::collections::BTreeMap;

use cosmwasm_std::{Addr, Deps, StdError, StdResult, Uint128};

pub const MAX_REVERSE_SIMULATIONS_PER_HOP: u32 = 33;
pub const MAX_REVERSE_SIMULATIONS_PER_REQUEST: u32 = 96;
pub const MAX_PAIR_HYBRID_REVERSE_SIMULATIONS: u32 = dex_common::pair::MAX_HYBRID_REVERSE_SIM_CALLS;

/// Legacy pair pool reverse used the same 32-simulation search as declared
/// hybrids. Only reviewed exact cw2 versions may receive the optimized cost.
/// Missing, malformed, unknown or failed identity reads reserve the opaque max.
pub(crate) fn pool_reverse_cost(deps: Deps, pair: &Addr) -> u32 {
    let known = deps
        .querier
        .query_wasm_raw(pair, b"contract_info")
        .ok()
        .flatten()
        .and_then(|raw| cosmwasm_std::from_json::<cw2::ContractVersion>(raw).ok())
        .is_some_and(|version| {
            version.contract == "cl8y-dex-pair"
                && matches!(version.version.as_str(), "1.19.0" | "1.20.0")
        });
    if known {
        1
    } else {
        MAX_PAIR_HYBRID_REVERSE_SIMULATIONS
    }
}

#[derive(Default)]
pub(crate) struct ReverseQueryBudget {
    per_hop: [u32; 4],
    total: u32,
}

impl ReverseQueryBudget {
    pub(crate) fn charge(&mut self, hop: usize, cost: u32) -> StdResult<()> {
        if hop >= self.per_hop.len()
            || cost > MAX_REVERSE_SIMULATIONS_PER_HOP.saturating_sub(self.per_hop[hop])
            || cost > MAX_REVERSE_SIMULATIONS_PER_REQUEST.saturating_sub(self.total)
        {
            return Err(StdError::generic_err("REVERSE_QUERY_BUDGET_EXCEEDED"));
        }
        self.per_hop[hop] += cost;
        self.total += cost;
        Ok(())
    }
}

/// Invert the actual greedy forward engine. Cache responses by raw offer;
/// a checked pool inverse seeds the bracket, but never proves greedy sufficiency.
/// The caller meters seed and every cache miss before making a nested query.
pub(crate) fn greedy_reverse_search(
    target: Uint128,
    seed: Uint128,
    mut simulate: impl FnMut(Uint128) -> StdResult<Uint128>,
) -> StdResult<Uint128> {
    if target.is_zero() {
        return Ok(Uint128::zero());
    }
    let mut cache = BTreeMap::new();
    cache.insert(Uint128::zero(), Uint128::zero());
    let mut output = |offer| -> StdResult<Uint128> {
        if let Some(value) = cache.get(&offer) {
            return Ok(*value);
        }
        let value = simulate(offer)?;
        cache.insert(offer, value);
        Ok(value)
    };
    let mut hi = seed.max(Uint128::one());
    let mut lo = Uint128::zero();
    loop {
        if output(hi)? >= target {
            break;
        }
        lo = hi;
        hi = hi.checked_mul(Uint128::new(2)).unwrap_or(Uint128::MAX);
        if hi == lo {
            return Err(StdError::generic_err("REVERSE_TARGET_INFEASIBLE"));
        }
    }
    // Empty or exhausted books often have the exact pool inverse minimum even
    // for 18-decimal targets. Prove the adjacent lower offer instead of bisecting
    // the whole Uint128 interval unnecessarily.
    if hi > Uint128::one() && output(hi - Uint128::one())? < target {
        return Ok(hi);
    }
    while hi.u128() - lo.u128() > 1 {
        let mid = Uint128::new(lo.u128() + (hi.u128() - lo.u128()) / 2);
        if output(mid)? >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    // The sufficient response is already cached; no confirmation replay.
    Ok(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue1389_budget_reserves_opaque_pair_cost_and_fresh_validation() {
        let mut budget = ReverseQueryBudget::default();
        for hop in 0..2 {
            budget
                .charge(hop, MAX_PAIR_HYBRID_REVERSE_SIMULATIONS)
                .unwrap();
            budget.charge(hop, 1).unwrap();
        }
        assert_eq!(budget.total, 66);
        assert!(budget
            .charge(2, 32)
            .unwrap_err()
            .to_string()
            .contains("REVERSE_QUERY_BUDGET_EXCEEDED"));
        assert_eq!(budget.total, 66); // denial occurs before the nested call
        assert!(budget.charge(0, 1).is_err());
    }

    #[test]
    fn issue1389_greedy_large_empty_book_minimum_needs_two_calls() {
        for target in [1u128, 1_000_000, 100_000_000_000_000_000_000, u128::MAX] {
            let mut calls = 0;
            let offer =
                greedy_reverse_search(Uint128::new(target), Uint128::new(target), |amount| {
                    calls += 1;
                    Ok(amount)
                })
                .unwrap();
            assert_eq!(offer, Uint128::new(target));
            assert!(calls <= 2);
        }
    }

    #[test]
    fn issue1389_expansion_and_bisection_share_budget_and_never_replay() {
        let mut budget = ReverseQueryBudget::default();
        budget.charge(0, 1).unwrap(); // reserved fresh validation
        budget.charge(0, 1).unwrap(); // checked pool seed
        let mut calls = 0;
        let error = greedy_reverse_search(Uint128::MAX, Uint128::one(), |_| {
            budget.charge(0, 1)?;
            calls += 1;
            Ok(Uint128::zero())
        })
        .unwrap_err();
        assert!(error.to_string().contains("REVERSE_QUERY_BUDGET_EXCEEDED"));
        assert_eq!(calls, 31);
        let mut calls = BTreeMap::new();
        let offer = greedy_reverse_search(Uint128::new(13), Uint128::new(32), |amount| {
            *calls.entry(amount).or_insert(0) += 1;
            Ok(amount)
        })
        .unwrap();
        assert_eq!(offer, Uint128::new(13));
        assert!(calls.values().all(|count| *count == 1));
    }

    #[test]
    fn issue1389_failed_nested_query_and_uint128_impossible_are_stable() {
        let error = greedy_reverse_search(Uint128::one(), Uint128::one(), |_| {
            Err(StdError::generic_err("nested failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("nested failure"));
        let mut calls = 0;
        let error = greedy_reverse_search(Uint128::MAX, Uint128::MAX, |_| {
            calls += 1;
            Ok(Uint128::zero())
        })
        .unwrap_err();
        assert!(error.to_string().contains("REVERSE_TARGET_INFEASIBLE"));
        assert_eq!(calls, 1);
        assert_eq!(
            greedy_reverse_search(Uint128::zero(), Uint128::one(), |_| panic!(
                "zero must not simulate"
            ))
            .unwrap(),
            Uint128::zero()
        );
    }
}
