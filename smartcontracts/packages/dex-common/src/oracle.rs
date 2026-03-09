// # TWAP Oracle — Geometric Mean (Uniswap V3-style)
//
// ## How it works
//
// Each pair accumulates `tick_cumulative` — the integral of `log₂(price)` over
// time — on every state-changing action (swap, provide/withdraw liquidity).
// A ring buffer of `Observation` snapshots lets consumers query any historical
// window and compute the geometric-mean TWAP:
//
//   geomean_price = 2^((tick_now − tick_then) / (t_now − t_then))
//
// ## Security model & known risks
//
// This oracle is designed to resist single-transaction manipulation (flash
// loans cannot affect the price used for accumulation because it is sampled
// from the *previous* reserves before the current action mutates them).
//
// **However, the following risks remain:**
//
// 1. **Multi-block manipulation** — A validator/proposer (or colluding set)
//    that controls consecutive blocks can cheaply skew the TWAP over short
//    windows. On Tendermint (Terra Classic), this risk is lower than PoS
//    Ethereum but not zero. Consumers SHOULD use windows of ≥30 minutes and
//    SHOULD cross-check against a secondary oracle feed (e.g. Band Protocol,
//    a governance-set reference price, or an off-chain relay) when using this
//    TWAP for high-value decisions such as liquidations or perp mark prices.
//
// 2. **Low-liquidity pairs** — Manipulation cost is proportional to pool
//    depth. Thin pools can be moved cheaply. Protocols SHOULD NOT rely on
//    this TWAP alone for pairs with < $100k TVL.
//
// 3. **Stale observations** — If no one interacts with the pair for an
//    extended period, observations stop being written. The `observe()` query
//    linearly interpolates between known points, but the accuracy degrades.
//    Critical consumers SHOULD ensure a keeper pokes the pair periodically.
//
// 4. **Geometric mean bias** — The geometric mean is strictly ≤ the
//    arithmetic mean (AM-GM inequality), meaning it slightly underestimates
//    the "average" price experienced by traders. This is a *feature* for
//    manipulation resistance but may matter for accounting.
//
// **Recommendation for perps / money markets:**
// Use this TWAP as a *primary* price source but ALWAYS pair it with at least
// one of:
//   - A secondary off-chain oracle feed with freshness & deviation checks
//   - A circuit-breaker that pauses liquidations when the TWAP deviates >N%
//     from the spot price
//   - A minimum-liquidity gate that disables the oracle for thin pairs

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};

/// Maximum number of observations the ring buffer can hold.
/// ~65 535 × 6s blocks ≈ 109 hours of history at one observation per block.
pub const MAX_OBSERVATION_CARDINALITY: u16 = 65_000;

/// Default observation cardinality for new pairs.
/// 360 × 6s blocks ≈ 36 minutes — enough for a 30-minute TWAP window.
pub const DEFAULT_OBSERVATION_CARDINALITY: u16 = 360;

// ---------------------------------------------------------------------------
// Fixed-point log₂ in Q64.64 representation
// ---------------------------------------------------------------------------
// We represent log₂(price) as an i128 in Q64.64 format: real = stored / 2^64.
// This is the value accumulated on-chain during swaps and liquidity events.
// Only `log2_ratio_q64` runs on the hot path; exp2 is query-only.

const Q64: u128 = 1u128 << 64;

/// Compute `log₂(a/b) × 2^64` for non-zero `a`, `b`.
/// Returns an i128 in Q64.64 fixed-point.
pub fn log2_ratio_q64(a: Uint128, b: Uint128) -> StdResult<i128> {
    if a.is_zero() || b.is_zero() {
        return Err(StdError::generic_err(
            "oracle: cannot compute log of zero reserves",
        ));
    }
    Ok(log2_q64_uint(a.u128()) - log2_q64_uint(b.u128()))
}

/// Compute `2^(tick / 2^64)` using `Decimal` (18-digit fixed-point).
/// This is **query-only** — not used during swap execution.
pub fn exp2_tick_to_decimal(tick_q64: i128) -> StdResult<Decimal> {
    let negative = tick_q64 < 0;
    let abs_val = tick_q64.unsigned_abs();

    let int_part = (abs_val >> 64) as u32;
    let frac_q64 = abs_val & (Q64 - 1);

    if !negative && int_part > 60 {
        return Err(StdError::generic_err("oracle: price overflow (2^60+)"));
    }

    let frac_dec = Decimal::from_ratio(frac_q64, Q64);

    // 2^frac = e^(frac × ln2). Taylor series for e^x with x = frac × ln2 ∈ [0, ln2).
    let ln2 = Decimal::from_ratio(
        693_147_180_559_945_309u128,
        1_000_000_000_000_000_000u128,
    );
    let x = frac_dec * ln2;

    // e^x ≈ 1 + x + x²/2! + … + x⁹/9!  (~20-digit precision for x < 0.7)
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x3 * x;
    let x5 = x4 * x;
    let x6 = x5 * x;
    let x7 = x6 * x;
    let x8 = x7 * x;
    let x9 = x8 * x;

    let frac_result = Decimal::one()
        + x
        + x2 * Decimal::from_ratio(1u128, 2u128)
        + x3 * Decimal::from_ratio(1u128, 6u128)
        + x4 * Decimal::from_ratio(1u128, 24u128)
        + x5 * Decimal::from_ratio(1u128, 120u128)
        + x6 * Decimal::from_ratio(1u128, 720u128)
        + x7 * Decimal::from_ratio(1u128, 5040u128)
        + x8 * Decimal::from_ratio(1u128, 40320u128)
        + x9 * Decimal::from_ratio(1u128, 362880u128);

    let mut price = frac_result;
    for _ in 0..int_part {
        price = price + price;
    }

    if negative {
        if price.is_zero() {
            return Err(StdError::generic_err("oracle: division by zero in exp2"));
        }
        Ok(Decimal::one() / price)
    } else {
        Ok(price)
    }
}

/// Compute `log₂(x) × 2^64` for x > 0.
///
/// Uses Q63 normalization: y lives in [2^63, 2^64) so that y*y always fits
/// in u128 (max product ≈ 2^128 - 2^65 + 1 < 2^128).
fn log2_q64_uint(x: u128) -> i128 {
    debug_assert!(x > 0);
    let bits = 128u32 - x.leading_zeros();
    let msb = (bits - 1) as i128;
    let mut result: i128 = msb << 64;

    // Normalize x so that MSB sits at bit 63 → value in [2^63, 2^64).
    let mut y: u128 = if msb <= 63 {
        x << (63 - msb as u32)
    } else {
        x >> (msb as u32 - 63)
    };

    // Iteratively square and extract 64 fractional bits of log₂.
    // Invariant: y ∈ [2^63, 2^64), so y*y ∈ [2^126, 2^128) — fits in u128.
    // After >> 63 the result is in [2^63, 2^65). If ≥ 2^64 ⇒ bit is 1, shift down.
    for i in 1..=64u32 {
        y = (y.wrapping_mul(y)) >> 63;
        if y >= (1u128 << 64) {
            result |= 1i128 << (64 - i);
            y >>= 1;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Observation types
// ---------------------------------------------------------------------------

/// A single TWAP observation recorded in the ring buffer.
#[cw_serde]
pub struct Observation {
    /// Block timestamp (seconds) when this observation was recorded.
    pub timestamp: u64,
    /// Cumulative `∫ log₂(reserve_b / reserve_a) dt`, Q64.64 fixed-point.
    pub tick_cumulative: i128,
}

/// Response for the `Observe` query.
#[cw_serde]
pub struct ObserveResponse {
    pub tick_cumulatives: Vec<i128>,
}

/// Response for the `OracleInfo` query.
#[cw_serde]
pub struct OracleInfoResponse {
    pub observation_cardinality: u16,
    pub observation_index: u16,
    pub oldest_observation_timestamp: u64,
    pub newest_observation_timestamp: u64,
}

/// Computes the geometric-mean TWAP price (as `Decimal`) from two cumulative
/// tick snapshots separated by `time_elapsed` seconds.
///
/// **SECURITY WARNING:** Do NOT use this as the sole price source for
/// liquidations, mark prices, or collateral valuation. Always cross-check
/// against an independent oracle feed. See module-level documentation.
pub fn compute_twap_price(
    tick_cumulative_start: i128,
    tick_cumulative_end: i128,
    time_elapsed: u64,
) -> StdResult<Decimal> {
    if time_elapsed == 0 {
        return Err(StdError::generic_err(
            "oracle: time_elapsed must be > 0",
        ));
    }
    let avg_tick = (tick_cumulative_end - tick_cumulative_start) / (time_elapsed as i128);
    exp2_tick_to_decimal(avg_tick)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_decimal_close(actual: Decimal, expected_f: f64, tolerance_pct: f64, label: &str) {
        let actual_str = actual.to_string();
        let actual_f: f64 = actual_str.parse().unwrap();
        let pct_err = ((actual_f - expected_f) / expected_f).abs() * 100.0;
        assert!(
            pct_err < tolerance_pct,
            "{}: expected ~{}, got {} (err {:.4}%)",
            label,
            expected_f,
            actual_f,
            pct_err
        );
    }

    #[test]
    fn log2_equal_values_is_zero() {
        let r = log2_ratio_q64(Uint128::new(1_000_000), Uint128::new(1_000_000)).unwrap();
        assert_eq!(r, 0);
    }

    #[test]
    fn log2_double_is_one() {
        let r = log2_ratio_q64(Uint128::new(2_000_000), Uint128::new(1_000_000)).unwrap();
        let one_q64 = 1i128 << 64;
        let diff = (r - one_q64).unsigned_abs();
        assert!(diff < Q64 / 100_000, "log2(2) diff={}", diff);
    }

    #[test]
    fn log2_half_is_neg_one() {
        let r = log2_ratio_q64(Uint128::new(1_000_000), Uint128::new(2_000_000)).unwrap();
        let neg_one_q64 = -(1i128 << 64);
        let diff = (r - neg_one_q64).unsigned_abs();
        assert!(diff < Q64 / 100_000, "log2(0.5) diff={}", diff);
    }

    #[test]
    fn log2_rejects_zero() {
        assert!(log2_ratio_q64(Uint128::zero(), Uint128::new(1)).is_err());
        assert!(log2_ratio_q64(Uint128::new(1), Uint128::zero()).is_err());
    }

    #[test]
    fn exp2_of_zero_is_one() {
        let r = exp2_tick_to_decimal(0).unwrap();
        assert_decimal_close(r, 1.0, 0.01, "exp2(0)");
    }

    #[test]
    fn exp2_of_one_is_two() {
        let one_q64 = 1i128 << 64;
        let r = exp2_tick_to_decimal(one_q64).unwrap();
        assert_decimal_close(r, 2.0, 0.01, "exp2(1)");
    }

    #[test]
    fn exp2_of_neg_one_is_half() {
        let neg_one_q64 = -(1i128 << 64);
        let r = exp2_tick_to_decimal(neg_one_q64).unwrap();
        assert_decimal_close(r, 0.5, 0.01, "exp2(-1)");
    }

    #[test]
    fn roundtrip_3_5() {
        let a = Uint128::new(3_500_000);
        let b = Uint128::new(1_000_000);
        let tick = log2_ratio_q64(a, b).unwrap();
        let price = exp2_tick_to_decimal(tick).unwrap();
        assert_decimal_close(price, 3.5, 0.1, "roundtrip(3.5)");
    }

    #[test]
    fn roundtrip_0_25() {
        let a = Uint128::new(250_000);
        let b = Uint128::new(1_000_000);
        let tick = log2_ratio_q64(a, b).unwrap();
        let price = exp2_tick_to_decimal(tick).unwrap();
        assert_decimal_close(price, 0.25, 0.1, "roundtrip(0.25)");
    }

    #[test]
    fn roundtrip_realistic_price() {
        let reserve_a = Uint128::new(50_000_000_000);
        let reserve_b = Uint128::new(100_000_000_000);
        let tick = log2_ratio_q64(reserve_b, reserve_a).unwrap();
        let price = exp2_tick_to_decimal(tick).unwrap();
        assert_decimal_close(price, 2.0, 0.1, "roundtrip(2.0)");
    }

    #[test]
    fn roundtrip_extreme_ratio() {
        let a = Uint128::new(1_000_000_000_000u128);
        let b = Uint128::new(1_000_000u128);
        let tick = log2_ratio_q64(a, b).unwrap();
        let price = exp2_tick_to_decimal(tick).unwrap();
        assert_decimal_close(price, 1_000_000.0, 0.1, "roundtrip(1M)");
    }

    #[test]
    fn compute_twap_basic() {
        let tick0 = 0i128;
        let one_q64 = 1i128 << 64;
        let tick1 = one_q64 * 3600;
        let price = compute_twap_price(tick0, tick1, 3600).unwrap();
        assert_decimal_close(price, 2.0, 0.1, "twap constant price=2");
    }

    #[test]
    fn compute_twap_rejects_zero_elapsed() {
        assert!(compute_twap_price(0, 100, 0).is_err());
    }
}
