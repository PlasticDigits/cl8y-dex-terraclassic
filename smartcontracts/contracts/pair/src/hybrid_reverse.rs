//! Hybrid reverse simulation helpers (GitLab #257).
//!
//! Seeds binary search from constant-product pool math so reverse quotes avoid
//! up to 128 redundant full hybrid simulations. Execute path unchanged.

use cosmwasm_std::{StdError, StdResult, Uint128, Uint256};

use dex_common::pair::HybridSwapParams;

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
    let fee_numerator = gross_output.checked_mul(Uint128::from(effective_fee_bps as u128))?;
    let commission = fee_numerator.checked_div(Uint128::new(10_000))?;
    Ok(gross_output.checked_sub(commission)?)
}

/// Minimum pool input (offer token units on the AMM leg) whose net output is >= `target_net`.
fn min_pool_input_for_net_output(
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

    let mut lo = Uint128::one();
    let mut hi = input_reserve;
    if pool_net_output_for_input(hi, input_reserve, output_reserve, effective_fee_bps)? < target_net
    {
        return Err(StdError::generic_err(
            "pool cannot satisfy net output target",
        ));
    }

    while lo < hi {
        let mid = lo.checked_add(hi)?.checked_div(Uint128::new(2))?;
        if mid.is_zero() {
            break;
        }
        let out = pool_net_output_for_input(mid, input_reserve, output_reserve, effective_fee_bps)?;
        if out >= target_net {
            hi = mid;
        } else {
            lo = mid.checked_add(Uint128::one())?;
        }
    }
    Ok(hi)
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
