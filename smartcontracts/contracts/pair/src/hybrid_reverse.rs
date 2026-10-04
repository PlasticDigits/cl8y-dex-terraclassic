//! Hybrid reverse simulation helpers (GitLab #257).
//!
//! Exact integer pool inversion and bounded hybrid search (issue #1376).
//! Pool-only requests reuse the minimum offer and need one forward validation;
//! declared-book requests retain the 32-call full-simulation budget.

use cosmwasm_std::{StdError, StdResult, Uint128, Uint256};

use dex_common::pair::{HybridSimulationResponse, HybridSwapParams};

#[inline]
fn u256(x: Uint128) -> Uint256 {
    Uint256::from(x)
}

/// 256-bit ceiling division (GitLab #464) — avoids the u128 product ceiling on
/// reserve math for high-decimal assets. Same rounding guarantee (result*b >= a).
fn ceil_div_u256(numerator: Uint256, denominator: Uint256) -> Uint256 {
    let d = numerator / denominator;
    if d * denominator < numerator {
        d + Uint256::one()
    } else {
        d
    }
}

fn narrow_u128(value: Uint256, ctx: &str) -> StdResult<Uint128> {
    Uint128::try_from(value)
        .map_err(|_| StdError::generic_err(format!("256-bit result exceeds u128 ({ctx})")))
}

/// Hard cap on full `simulate_hybrid_swap_with_fee` calls per reverse query.
pub const MAX_HYBRID_REVERSE_SIM_CALLS: u32 = 32;

/// Pool leg net output (ask token) for a given pool input, matching forward sim rounding.
pub fn pool_net_output_for_input(
    pool_input: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    effective_fee_bps: u16,
) -> StdResult<Uint128> {
    if pool_input.is_zero() {
        return Ok(Uint128::zero());
    }
    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Err(StdError::generic_err("pool reserves empty"));
    }
    let k = u256(input_reserve).checked_mul(u256(output_reserve))?;
    let new_input_reserve = input_reserve.checked_add(pool_input)?;
    let new_output_reserve = narrow_u128(
        ceil_div_u256(k, u256(new_input_reserve)),
        "pool_net new_output_reserve",
    )?;
    let gross_output = output_reserve.checked_sub(new_output_reserve)?;
    let commission = pool_commission(gross_output, effective_fee_bps)?;
    Ok(gross_output.checked_sub(commission)?)
}

/// Fee rounding shared by execute, forward simulation and the pool inverse.
pub fn pool_commission(gross: Uint128, fee_bps: u16) -> StdResult<Uint128> {
    if fee_bps > 10_000 {
        return Err(StdError::generic_err("pool fee exceeds 100%"));
    }
    narrow_u128(
        u256(gross).checked_mul(Uint256::from(fee_bps))? / Uint256::from(10_000u32),
        "pool commission",
    )
}

/// Minimum representable pool input whose rounded net output meets the target.
/// Net = ceil(gross * (10000-fee) / 10000), so the minimum gross is
/// floor((target-1)*10000/(10000-fee))+1. Constant product then requires
/// input_reserve+offer >= ceil(k/(output_reserve-gross)). All products are Uint256.
pub fn min_pool_input_for_net_output(
    input_reserve: Uint128,
    output_reserve: Uint128,
    effective_fee_bps: u16,
    target_net: Uint128,
) -> StdResult<Uint128> {
    if target_net.is_zero() {
        return Ok(Uint128::zero());
    }
    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Err(StdError::generic_err("pool reserves empty"));
    }
    if effective_fee_bps >= 10_000 {
        return Err(StdError::generic_err(
            "pool cannot satisfy net output target: fee >= 100%",
        ));
    }
    let gross = u256(target_net.checked_sub(Uint128::one())?)
        .checked_mul(Uint256::from(10_000u32))?
        / Uint256::from(10_000u32 - u32::from(effective_fee_bps))
        + Uint256::one();
    if gross >= u256(output_reserve) {
        return Err(StdError::generic_err(
            "pool cannot satisfy net output target",
        ));
    }
    let required_reserve = ceil_div_u256(
        u256(input_reserve).checked_mul(u256(output_reserve))?,
        u256(output_reserve).checked_sub(gross)?,
    );
    // Narrow the new reserve too: execute must be able to add the offer to it.
    let required_reserve = narrow_u128(required_reserve, "pool offer overflow")?;
    Ok(required_reserve.checked_sub(input_reserve)?)
}

/// Search a declared hybrid template while retaining the established 32-call budget.
/// Cache the sufficient upper response so success does not need an unbudgeted final
/// call; use difference midpoints and clamp growth before raw-unit addition overflow.
/// A budget error is explicit and never returned as a successful sufficient quote.
pub fn bounded_hybrid_reverse_search(
    seed: Uint128,
    max_offer: Uint128,
    target: Uint128,
    mut simulate: impl FnMut(Uint128) -> StdResult<HybridSimulationResponse>,
) -> StdResult<(Uint128, HybridSimulationResponse)> {
    let mut calls = 0;
    let mut run = |offer| {
        if calls >= MAX_HYBRID_REVERSE_SIM_CALLS {
            return Err(StdError::generic_err(
                "Hybrid reverse simulation exceeded iteration cap",
            ));
        }
        calls += 1;
        simulate(offer)
    };
    if max_offer.is_zero() {
        return Err(StdError::generic_err(
            "hybrid target infeasible: offer range exhausted",
        ));
    }
    let mut hi = seed.max(Uint128::one()).min(max_offer);
    let mut sufficient = loop {
        let sim = run(hi)?;
        if sim.return_amount >= target {
            break sim;
        }
        if hi == max_offer {
            return Err(StdError::generic_err(
                "hybrid target infeasible: offer range exhausted",
            ));
        }
        hi = hi
            .checked_mul(Uint128::new(2))
            .unwrap_or(max_offer)
            .min(max_offer);
    };
    let mut lo = Uint128::one();
    while lo < hi {
        let mid = lo.checked_add(hi.checked_sub(lo)? / Uint128::new(2))?;
        let sim = run(mid)?;
        if sim.return_amount >= target {
            hi = mid;
            sufficient = sim;
        } else {
            lo = mid.checked_add(Uint128::one())?;
        }
    }
    Ok((hi, sufficient))
}

/// Total hybrid offer such that the scaled pool leg equals `pool_input_needed`.
pub fn total_offer_for_pool_leg(
    hybrid: &HybridSwapParams,
    pool_input_needed: Uint128,
) -> Result<Uint128, StdError> {
    if hybrid.pool_input.is_zero() {
        return Err(StdError::generic_err(
            "cannot scale pool leg when template pool_input is zero",
        ));
    }
    let den = hybrid.pool_input.checked_add(hybrid.book_input)?;
    if den.is_zero() {
        return Err(StdError::generic_err(
            "hybrid template sum must be positive",
        ));
    }
    narrow_u128(
        ceil_div_u256(
            u256(pool_input_needed).checked_mul(u256(den))?,
            u256(hybrid.pool_input),
        ),
        "total_offer_for_pool_leg",
    )
}

/// Upper-bound seed for reverse search: pool-only reverse on full `ask_target`, scaled so
/// the hybrid template allocates at least that much to the pool leg.
pub fn seed_upper_offer_from_pool_math(
    hybrid: &HybridSwapParams,
    input_reserve: Uint128,
    output_reserve: Uint128,
    effective_fee_bps: u16,
    ask_target: Uint128,
) -> StdResult<Uint128> {
    if hybrid.book_input.is_zero() {
        return min_pool_input_for_net_output(
            input_reserve,
            output_reserve,
            effective_fee_bps,
            ask_target,
        );
    }
    if hybrid.pool_input.is_zero() {
        return Ok(Uint128::one());
    }

    let pool_only_offer =
        min_pool_input_for_net_output(input_reserve, output_reserve, effective_fee_bps, ask_target)
            .unwrap_or(input_reserve);
    total_offer_for_pool_leg(hybrid, pool_only_offer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dex_common::pair::pool_only_hybrid_template;

    #[test]
    fn pool_only_seed_matches_pool_reverse() {
        let template = pool_only_hybrid_template();
        let seed = seed_upper_offer_from_pool_math(
            &template,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
            30,
            Uint128::new(10_000),
        )
        .unwrap();
        assert!(seed > Uint128::zero());
        let out =
            pool_net_output_for_input(seed, Uint128::new(1_000_000), Uint128::new(1_000_000), 30)
                .unwrap();
        assert!(out >= Uint128::new(10_000));
        if seed > Uint128::one() {
            let out_lo = pool_net_output_for_input(
                seed.checked_sub(Uint128::one()).unwrap(),
                Uint128::new(1_000_000),
                Uint128::new(1_000_000),
                30,
            )
            .unwrap();
            assert!(out_lo < Uint128::new(10_000));
        }
    }

    // GitLab #464: reserves at ~20 whole tokens/side of an 18-decimal asset (2e19 raw) give
    // reserve_a * reserve_b = 4e38 > u128::MAX. The pre-fix native-u128 product reverted here;
    // the 256-bit math must return a real pool output instead of overflowing.
    #[test]
    fn pool_net_output_survives_18dec_scale_reserves() {
        let r = Uint128::new(20_000_000_000_000_000_000); // 2e19 raw = 20 whole 18-dec tokens
        let one = Uint128::new(1_000_000_000_000_000_000); // 1 token in
        let out = pool_net_output_for_input(one, r, r, 30).unwrap();
        assert!(out > Uint128::zero(), "expected a real output, got zero");
        assert!(out < r, "output must be below the reserve");
        // Widening must not change small-reserve results: same call at 1e6 reserves.
        let small = pool_net_output_for_input(
            Uint128::new(1_000),
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
            30,
        )
        .unwrap();
        assert!(small > Uint128::zero());
    }

    #[test]
    fn hybrid_template_scales_total_offer() {
        let hybrid = HybridSwapParams {
            pool_input: Uint128::new(40_000),
            book_input: Uint128::new(60_000),
            max_maker_fills: 8,
            book_start_hint: None,
        };
        let total = total_offer_for_pool_leg(&hybrid, Uint128::new(40_000)).unwrap();
        assert_eq!(total, Uint128::new(100_000));
    }
}

#[cfg(test)]
mod issue_1376_tests {
    use super::*;

    fn assert_minimum(input: u128, output: u128, fee: u16, target: u128) -> Uint128 {
        let (input, output, target) = (
            Uint128::new(input),
            Uint128::new(output),
            Uint128::new(target),
        );
        let offer = min_pool_input_for_net_output(input, output, fee, target).unwrap();
        assert!(pool_net_output_for_input(offer, input, output, fee).unwrap() >= target);
        assert!(
            pool_net_output_for_input(offer - Uint128::one(), input, output, fee).unwrap() < target
        );
        offer
    }

    #[test]
    fn issue1376_live_snapshot_minimum_offers_cross_old_cap() {
        let input = 297_665_983_808;
        let output = 2_498_115_414_904_458_263_864;
        assert_eq!(
            assert_minimum(input, output, 180, 10_000_000_000_000_000_000),
            Uint128::new(1_218_369_992)
        );
        assert_eq!(
            assert_minimum(input, output, 180, 100_000_000_000_000_000_000),
            Uint128::new(12_649_685_215)
        );
        assert_minimum(output, input, 180, 100_000_000);
    }

    /// Replay the removed redundant search to prove this fixture straddles the
    /// deployed 32-call limit, including its initial and final full simulations.
    #[test]
    fn issue1376_replays_old_32_vs_35_call_boundary() {
        let input = Uint128::new(297_665_983_808);
        let output = Uint128::new(2_498_115_414_904_458_263_864);
        for (target, expected_calls) in [
            (10_000_000_000_000_000_000, 32),
            (100_000_000_000_000_000_000, 35),
        ] {
            let target = Uint128::new(target);
            let mut hi = min_pool_input_for_net_output(input, output, 180, target).unwrap();
            let mut lo = Uint128::one();
            let mut calls = 2; // seed check and final response replay
            while lo < hi {
                let mid = lo + (hi - lo) / Uint128::new(2);
                calls += 1;
                if pool_net_output_for_input(mid, input, output, 180).unwrap() >= target {
                    hi = mid;
                } else {
                    lo = mid + Uint128::one();
                }
            }
            assert_eq!(calls, expected_calls);
        }
    }

    #[test]
    fn issue1376_inverse_high_decimals_and_more_than_half_reserve() {
        assert_minimum(
            20_000_000_000_000_000_000,
            20_000_000_000_000_000_000,
            30,
            15_000_000_000_000_000_000,
        );
        // Fee multiplication exceeds u128; the output and fee still fit.
        assert_minimum(u128::MAX / 10, u128::MAX / 10, 180, u128::MAX / 100);
        assert_minimum(1_000_000, 1_000_000, 0, 900_000);
    }

    #[test]
    fn issue1376_inverse_fee_rounding_small_domain() {
        for fee in [0, 30, 180, 9999] {
            for input in [1, 17, 99] {
                for target in 1..20 {
                    let output = 1_000_000;
                    assert_minimum(input, output, fee, target);
                }
            }
        }
    }

    #[test]
    fn issue1376_infeasible_and_overflow_are_finite_errors() {
        for (input, output, fee, target) in [
            (0, 10, 0, 1),
            (10, 0, 0, 1),
            (10, 10, 0, 10),
            (10, 10, 10_000, 1),
            (10, 10, 10_001, 1),
            (u128::MAX - 1, 10, 0, 1),
            (1, 10, 180, u128::MAX),
        ] {
            assert!(min_pool_input_for_net_output(
                Uint128::new(input),
                Uint128::new(output),
                fee,
                Uint128::new(target)
            )
            .is_err());
        }
    }

    fn response(amount: Uint128) -> HybridSimulationResponse {
        HybridSimulationResponse {
            return_amount: amount,
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
            pool_commission_amount: Uint128::zero(),
            book_commission_amount: Uint128::zero(),
            book_return_amount: Uint128::zero(),
            pool_return_amount: amount,
            limit_book_offer_consumed: Uint128::zero(),
            greedy_stop: None,
        }
    }

    #[test]
    fn issue1376_hybrid_budget_and_overflow_safe_brackets() {
        let mut calls = 0;
        let err =
            bounded_hybrid_reverse_search(Uint128::MAX, Uint128::MAX, Uint128::MAX, |offer| {
                calls += 1;
                Ok(response(offer))
            })
            .unwrap_err();
        assert!(err.to_string().contains("iteration cap"));
        assert_eq!(calls, MAX_HYBRID_REVERSE_SIM_CALLS);
        let mut calls = 0;
        let err = bounded_hybrid_reverse_search(
            Uint128::MAX - Uint128::one(),
            Uint128::MAX,
            Uint128::one(),
            |_| {
                calls += 1;
                Ok(response(Uint128::zero()))
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("infeasible"));
        assert_eq!(calls, 2);
    }

    #[test]
    fn issue1376_hybrid_reuses_sufficient_response_within_budget() {
        let mut calls = 0;
        let (offer, sim) = bounded_hybrid_reverse_search(
            Uint128::new(1 << 30),
            Uint128::MAX,
            Uint128::new(1 << 30),
            |offer| {
                calls += 1;
                Ok(response(offer))
            },
        )
        .unwrap();
        assert_eq!(offer, Uint128::new(1 << 30));
        assert_eq!(sim.return_amount, offer);
        assert_eq!(calls, 31); // initial validation + 30 bisections, no final replay.
    }
}
